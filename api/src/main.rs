mod config;
mod db;
mod model;

use std::future::Future;
use std::time::Duration;

use anyhow::{Context as _, Result};
use axum::extract::ws::{self, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use config::AppConfig;
use db::Repository;
use model::{Order, OrderGroup, PayedEvent, WaitNumber};
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::db::seaorm::SeaOrmRepository;
use crate::model::Id;

#[tokio::main]
async fn main() {
    true_main().await;
}

async fn true_main() {
    dotenv::dotenv().ok();

    let use_ansi = std::env::var("NO_COLOR").is_err();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=trace,axum::rejections=trace,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(use_ansi))
        .init();

    let config: AppConfig = {
        let file = tokio::fs::read_to_string("./config.toml")
            .await
            .expect("failed to read config.toml");
        toml::from_str(&file).expect("failed to aprse config.toml")
    };

    // do not drop receiver
    let (order_send, order_recv) = broadcast::channel(1);
    let (cooked_send, cooked_recv) = broadcast::channel(1);
    let (assign_send, assign_recv) = broadcast::channel(1);
    let (delivered_send, delivered_recv) = broadcast::channel(1);

    let consume_stream = |mut d: broadcast::Receiver<()>| {
        tokio::spawn(async move {
            loop {
                d.recv().await.unwrap()
            }
        });
    };

    consume_stream(order_recv);
    consume_stream(cooked_recv);
    consume_stream(assign_recv);
    consume_stream(delivered_recv);

    let ctx = MyContext {
        repo: SeaOrmRepository::new(&config.db_uri).await.unwrap(),
        order_chan: order_send,
        cooked_chan: cooked_send,
        assign_chan: assign_send,
        delivered_chan: delivered_send,
    };
    type C = MyContext<SeaOrmRepository>;

    let app = Router::new()
        .route("/", get(root))
        .route("/orders", post(post_orders::<C>))
        .route("/orders/by-id/:id/payment", post(pay_order::<C>))
        .route("/orders/queued_ws", get(list_queued_orders_ws::<C>))
        .route("/order/by-id/:id/assign", post(assign_order::<C>))
        .route("/order/by-id/:id/ready", post(order_cooking_done::<C>))
        .route(
            "/orders/pending_ws",
            get(list_pending_and_ready_orders_ws::<C>),
        )
        .route("/orders/ready", get(ready_orders_ws::<C>))
        .route("/order/by-id/:id/delivered", post(delivered::<C>))
        .layer(TraceLayer::new_for_http().make_span_with(
            |req: &Request<_>| info_span!("req", method=?req.method(), path=req.uri().to_string()),
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::any())
                .allow_headers(Any)
                .allow_methods(Any),
        )
        .with_state(ctx);

    tracing::info!("listening at {}", config.listen_addr);

    axum::Server::bind(&config.listen_addr.into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

trait Context: Send + Sync + 'static {
    fn repo(&self) -> impl Repository;

    fn order_queue_sender(&self) -> broadcast::Sender<()>;
    fn order_queue_subscriber(&self) -> broadcast::Receiver<()> {
        self.order_queue_sender().subscribe()
    }

    fn assign_sender(&self) -> broadcast::Sender<()>;
    fn assign_subscriber(&self) -> broadcast::Receiver<()> {
        self.assign_sender().subscribe()
    }

    fn cooking_done_sender(&self) -> broadcast::Sender<()>;
    fn cooking_done_subscriber(&self) -> broadcast::Receiver<()> {
        self.cooking_done_sender().subscribe()
    }

    fn delivered_sender(&self) -> broadcast::Sender<()>;
    fn delivered_subscriber(&self) -> broadcast::Receiver<()> {
        self.delivered_sender().subscribe()
    }
}

#[derive(Debug)]
enum Trigger {
    Order,
    Assign,
    Cooked,
    Delivered,
}

#[derive(Clone)]
struct MyContext<R> {
    repo: R,
    order_chan: broadcast::Sender<()>,
    assign_chan: broadcast::Sender<()>,
    cooked_chan: broadcast::Sender<()>,
    delivered_chan: broadcast::Sender<()>,
}
impl<R: Repository + Clone + Send + Sync + 'static> Context for MyContext<R> {
    fn repo(&self) -> impl Repository {
        self.repo.clone()
    }
    fn order_queue_sender(&self) -> broadcast::Sender<()> {
        self.order_chan.clone()
    }
    fn cooking_done_sender(&self) -> broadcast::Sender<()> {
        self.cooked_chan.clone()
    }
    fn assign_sender(&self) -> broadcast::Sender<()> {
        self.assign_chan.clone()
    }
    fn delivered_sender(&self) -> broadcast::Sender<()> {
        self.delivered_chan.clone()
    }
}

struct AppError(anyhow::Error);
impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        AppError(value)
    }
}
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("internal server error: {:#?}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateOrderRequest {
    count: u8,
    egg: bool,
    cheese: bool,
    spicy_mayonnaise: bool,
    no_mayonnaise: bool,
    no_sauce: bool,
    no_bonito: bool,
    no_aonori: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateOrderResponse {
    id: Id<OrderGroup>,
    price: u16,
}

async fn post_orders<C: Context>(
    State(ctx): State<C>,
    Json(body): Json<Vec<CreateOrderRequest>>,
) -> Result<Json<CreateOrderResponse>, AppError> {
    let price_table = ctx
        .repo()
        .get_latest_price_table()
        .await
        .context("failed to get latest price table")?;

    let mut group = OrderGroup {
        id: Id::new(),
        orders: vec![],
        price_table,
        created_at: Utc::now(),
    };

    for order in body {
        let CreateOrderRequest {
            count,
            egg,
            cheese,
            spicy_mayonnaise,
            no_mayonnaise,
            no_sauce,
            no_bonito,
            no_aonori,
        } = order;

        group.orders.extend((0..count).map(|_| Order {
            id: Id::new(),
            egg,
            cheese,
            spicy_mayonnaise,
            no_mayonnaise,
            no_sauce,
            no_bonito,
            no_aonori,
            created_at: Utc::now(),
        }));
    }

    ctx.repo()
        .insert_order_group(&group)
        .await
        .context("failed to insert order")?;

    Ok(Json(CreateOrderResponse {
        id: group.id,
        price: group.total_price(),
    }))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PayRequest {
    payed_amount: u16,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct PayResponse {
    /// 受付番号
    recept_number: Vec<WaitNumber>,
}

async fn pay_order<C: Context>(
    State(ctx): State<C>,
    Path(order_group_id): Path<Id<OrderGroup>>,
    Json(body): Json<PayRequest>,
) -> Result<Json<PayResponse>, AppError> {
    let event = PayedEvent {
        order_group_id,
        payed_amount: body.payed_amount,
        created_at: Utc::now(),
    };

    ctx.repo().insert_payed_event(&event).await?;

    let order_ids = ctx.repo().queue_orders_for_cook(&order_group_id).await?;

    ctx.order_queue_sender()
        .send(())
        .context("failed to notify ordered event")?;

    Ok(Json(PayResponse {
        recept_number: order_ids,
    }))
}

async fn list_queued_orders_ws<C: Context>(
    wsu: WebSocketUpgrade,
    State(ctx): State<C>,
) -> impl IntoResponse {
    return handle_ws(wsu, ctx, SyncerImpl).await;

    struct SyncerImpl;
    impl<C: Context> Syncer<C> for SyncerImpl {
        type Res = serde_json::Value;

        async fn wait(&self, ctx: &C) -> Trigger {
            let mut order = ctx.order_queue_subscriber();
            let mut assign = ctx.assign_subscriber();
            let mut cooked = ctx.cooking_done_subscriber();
            tokio::select! {
                _ = order.recv() => Trigger::Order,
                _ = assign.recv() => Trigger::Assign,
                _ = cooked.recv() => Trigger::Cooked,
            }
        }

        async fn sync(&self, ctx: &C) -> Result<Self::Res> {
            let queue = ctx
                .repo()
                .get_queued_orders()
                .await
                .context("failed to get queued orders")?;

            Ok(serde_json::json! {{
                "type": "sync",
                "queue": queue,
            }})
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssignOrderBody {
    chef_number: u8,
}
async fn assign_order<C: Context>(
    State(ctx): State<C>,
    Path(order_id): Path<Id<Order>>,
    Json(body): Json<AssignOrderBody>,
) -> Result<(), AppError> {
    ctx.repo()
        .assign_order(order_id, body.chef_number)
        .await
        .context("failed to assign")?;

    ctx.assign_sender()
        .send(())
        .context("failed to notify ordered event")?;

    Ok(())
}

async fn order_cooking_done<C: Context>(
    State(ctx): State<C>,
    Path(order_id): Path<Id<Order>>,
) -> Result<(), AppError> {
    ctx.repo()
        .order_ready(order_id)
        .await
        .context("failed to mark ready")?;

    ctx.cooking_done_sender()
        .send(())
        .context("failed to notify cook ready event")?;

    Ok(())
}

async fn list_pending_and_ready_orders_ws<C: Context>(
    wsu: WebSocketUpgrade,
    State(ctx): State<C>,
) -> impl IntoResponse {
    return handle_ws(wsu, ctx, SyncerImpl).await;

    struct SyncerImpl;
    impl<C: Context> Syncer<C> for SyncerImpl {
        type Res = serde_json::Value;

        async fn wait(&self, ctx: &C) -> Trigger {
            let mut order = ctx.order_queue_subscriber();
            let mut cooked = ctx.cooking_done_subscriber();
            let mut delivered = ctx.delivered_subscriber();
            tokio::select! {
                _ = order.recv() => Trigger::Order,
                _ = cooked.recv() => Trigger::Cooked,
                _ = delivered.recv() => Trigger::Delivered,
            }
        }

        async fn sync(&self, ctx: &C) -> Result<Self::Res> {
            let mut queued = ctx
                .repo()
                .get_queued_orders()
                .await
                .context("failed to get queued orders")?
                .into_iter()
                .map(|x| x.wait_number)
                .collect::<Vec<_>>();
            queued.sort_unstable_by_key(|x| x.0);

            let mut ready = ctx
                .repo()
                .get_ready_orders()
                .await
                .context("failed to get ready orders")?
                .into_iter()
                .map(|x| x.wait_number)
                .collect::<Vec<_>>();
            ready.sort_unstable_by_key(|x| x.0);

            Ok(serde_json::json! {{
                "type": "sync",
                "pending": queued,
                "calling": ready,
            }})
        }
    }
}

async fn ready_orders_ws<C: Context>(
    wsu: WebSocketUpgrade,
    State(ctx): State<C>,
) -> impl IntoResponse {
    return handle_ws(wsu, ctx, SyncerImpl).await;

    struct SyncerImpl;
    impl<C: Context> Syncer<C> for SyncerImpl {
        type Res = serde_json::Value;

        async fn wait(&self, ctx: &C) -> Trigger {
            let mut cooked = ctx.cooking_done_subscriber();
            let mut delivered = ctx.delivered_subscriber();
            tokio::select! {
                _ = cooked.recv() => Trigger::Cooked,
                _ = delivered.recv() => Trigger::Delivered,
            }
        }

        async fn sync(&self, ctx: &C) -> Result<Self::Res> {
            let ready = ctx
                .repo()
                .get_ready_orders()
                .await
                .context("failed to get ready orders")?;

            Ok(serde_json::json! {{
                "type": "sync",
                "calling": ready,
            }})
        }
    }
}

async fn delivered<C: Context>(
    State(ctx): State<C>,
    Path(order_id): Path<Id<Order>>,
) -> Result<(), AppError> {
    ctx.repo()
        .order_delivered(order_id)
        .await
        .context("failed to mark order delivered")?;

    ctx.delivered_sender()
        .send(())
        .context("failed to notify ordered event")?;

    Ok(())
}

trait Syncer<C: Context>: Send + Sync + 'static {
    type Res: serde::Serialize + Send + Sync + 'static;

    fn wait(&self, ctx: &C) -> impl Future<Output = Trigger> + Send;
    fn sync(&self, ctx: &C) -> impl Future<Output = Result<Self::Res>> + Send;
}

async fn handle_ws<C: Context>(
    wsu: WebSocketUpgrade,
    ctx: C,
    syncer: impl Syncer<C>,
) -> impl IntoResponse {
    return wsu.on_upgrade(|ws| async move {
        if let Err(e) = handle_socket(ctx, syncer, ws).await {
            tracing::error!("websocket error: {e:#?}");
        }
    });

    async fn handle_socket<C: Context>(
        ctx: C,
        syncer: impl Syncer<C>,
        mut ws: WebSocket,
    ) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(10));

        const PING_MAGIC: &[u8] = &[0x07, 0x08];

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    tracing::debug!("syncing due to interval");
                    sync(&ctx, &syncer, &mut ws).await?
                }
                _ = syncer.wait(&ctx) => {
                    tracing::debug!("syncing due to manual trigger");
                    sync(&ctx, &syncer, &mut ws).await?
                }

                msg = ws.recv() => {
                    let Some(msg) = msg else {
                        return Ok(());
                    };

                    let msg = msg.context("failed to decode message")?;

                    match msg {
                        ws::Message::Pong(m) => {
                            if m != PING_MAGIC {
                                tracing::warn!("unknown pong magic: {PING_MAGIC:?}");
                            }
                            continue;
                        }

                        // doc says
                        // > Ping messages will be automatically responded to by the server, so you do not have to worry
                        // > about dealing with them yourself.
                        ws::Message::Ping(_) => continue,

                        ws::Message::Close(_) => return Ok(()),
                        _ => continue,
                    };
                }
            }
        }

        async fn sync<C: Context>(
            ctx: &C,
            syncer: &impl Syncer<C>,
            ws: &mut WebSocket,
        ) -> Result<()> {
            ws.send(ws::Message::Ping(PING_MAGIC.to_vec()))
                .await
                .context("failed to send ping message")?;

            let d = syncer.sync(ctx).await?;
            let d = serde_json::to_string(&d).context("failed to serialize sync response")?;

            ws.send(ws::Message::Text(d))
                .await
                .context("failed to sync queued orders with client")?;
            Ok(())
        }
    }
}

pub mod entities;

use entities::*;

use anyhow::{Context, Result};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, EntityTrait, QueryOrder};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::db::Repository;
use crate::model::{ChefCode, Id, Order, OrderGroup, PayedEvent, PriceTable, WaitNumber};

use super::QueuedOrder;

#[derive(Clone)]
pub struct SeaOrmRepository {
    con: DatabaseConnection,
    sqlx: Pool<Postgres>,
}
impl SeaOrmRepository {
    pub async fn new(uri: &str) -> Result<Self> {
        let con = Database::connect(uri)
            .await
            .context("failed to connect to database (seaorm)")?;

        let sqlx = PgPoolOptions::new()
            .connect(uri)
            .await
            .context("failed to connect to database (sqlx)")?;

        Ok(Self { con, sqlx })
    }
}

impl Repository for SeaOrmRepository {
    async fn get_latest_price_table(&self) -> Result<PriceTable> {
        let res = price::Entity::find()
            .order_by_desc(price::Column::CreatedAt)
            .one(&self.con)
            .await
            .context("failed to query")?
            .context("database did not returned any result")?;

        let price::Model {
            id,
            base,
            egg,
            cheese,
            spicy_mayonnaise,
            no_mayonnaise,
            no_sauce,
            no_bonito,
            no_aonori,
            created_at: _,
        } = res;

        Ok(PriceTable {
            id: id.into(),
            base: base as u16,
            egg: egg as u16,
            cheese: cheese as u16,
            spicy_mayonnaise: spicy_mayonnaise as u16,
            no_mayonnaise: no_mayonnaise as u16,
            no_sauce: no_sauce as u16,
            no_bonito: no_bonito as u16,
            no_aonori: no_aonori as u16,
        })
    }

    async fn insert_price_table(&self, table: &PriceTable) -> Result<()> {
        let PriceTable {
            id,
            base,
            egg,
            cheese,
            spicy_mayonnaise,
            no_mayonnaise,
            no_sauce,
            no_bonito,
            no_aonori,
        } = table.clone();

        use sea_orm::ActiveValue::*;

        entities::price::ActiveModel {
            id: Set(id.into()),
            base: Set(base as i32),
            egg: Set(egg as i32),
            cheese: Set(cheese as i32),
            spicy_mayonnaise: Set(spicy_mayonnaise as i32),
            no_mayonnaise: Set(no_mayonnaise as i32),
            no_sauce: Set(no_sauce as i32),
            no_bonito: Set(no_bonito as i32),
            no_aonori: Set(no_aonori as i32),
            created_at: NotSet,
        }
        .insert(&self.con)
        .await
        .context("failed to insert new price table")?;

        Ok(())
    }

    async fn insert_order_group(&self, group: &OrderGroup) -> Result<()> {
        use sea_orm::ActiveValue::*;

        entities::order_group::ActiveModel {
            id: Set(group.id.into()),
            price_table_id: Set(group.price_table.id.into()),
            created_at: Set(group.created_at.into()),
        }
        .insert(&self.con)
        .await
        .context("failed to insert order group")?;

        let orders = group
            .orders
            .iter()
            .map(|order| {
                let Order {
                    id,
                    egg,
                    cheese,
                    spicy_mayonnaise,
                    no_mayonnaise,
                    no_sauce,
                    no_bonito,
                    no_aonori,
                    created_at,
                } = order.clone();

                entities::orders::ActiveModel {
                    id: Set(id.into()),
                    group_id: Set(group.id.into()),
                    egg: Set(egg),
                    cheese: Set(cheese),
                    spicy_mayonnaise: Set(spicy_mayonnaise),
                    no_mayonnaise: Set(no_mayonnaise),
                    no_sauce: Set(no_sauce),
                    no_bonito: Set(no_bonito),
                    no_aonori: Set(no_aonori),
                    created_at: Set(created_at.into()),
                }
            })
            .collect::<Vec<_>>();

        entities::orders::Entity::insert_many(orders)
            .exec(&self.con)
            .await
            .context("failed to insert orders")?;
        Ok(())
    }

    async fn cancel_order_group(&self, id: Id<OrderGroup>) -> Result<()> {
        let id: Uuid = id.into();
        let res = sqlx::query!("select * from order_payed where order_group_id=$1", id)
            .fetch_optional(&self.sqlx)
            .await
            .context("failed to assert that not cancelling group is not payed already")?;

        if res.is_some() {
            anyhow::bail!("tried to cancel group that is already payed");
        }

        sqlx::query!("delete from orders where group_id=$1", id)
            .execute(&self.sqlx)
            .await
            .context("failed to delete orders")?;
        sqlx::query!("delete from orders where id=$1", id)
            .execute(&self.sqlx)
            .await
            .context("failed to delete group")?;

        Ok(())
    }

    async fn insert_payed_event(&self, event: &PayedEvent) -> Result<()> {
        let PayedEvent {
            payed_amount,
            order_group_id,
            created_at,
        } = event.clone();

        use sea_orm::ActiveValue::*;

        entities::order_payed::ActiveModel {
            payed_amount: Set(payed_amount as i32),
            order_group_id: Set(order_group_id.into()),
            created_at: Set(created_at.into()),
        }
        .insert(&self.con)
        .await
        .context("failed to insert")?;

        Ok(())
    }

    async fn queue_orders_for_cook(&self, group_id: &Id<OrderGroup>) -> Result<Vec<WaitNumber>> {
        let group_id: Uuid = (*group_id).into();

        let res = sqlx::query!(
            "insert into order_queued(order_id)
             select orders.id from orders
             where orders.group_id = $1
             returning order_queued.wait_number as wait_number",
            group_id
        )
        .fetch_all(&self.sqlx)
        .await
        .context("failed to queue orders for cooking")?;

        Ok(res
            .into_iter()
            .map(|x| WaitNumber(x.wait_number as u32))
            .collect())
    }

    async fn get_queued_orders(&self) -> Result<Vec<QueuedOrder>> {
        let res = sqlx::query!(
            r#"select
                orders.*,
                order_queued.wait_number as wait_number,
                order_assigned.chef_number as "chef_number?"
            from order_queued
            inner      join orders         on order_queued.order_id=orders.id
            left outer join order_ready    on order_queued.order_id=order_ready.order_id
            left outer join order_assigned on order_queued.order_id=order_assigned.order_id
            where order_ready.order_id is null
            order by orders.created_at"#
        )
        .fetch_all(&self.sqlx)
        .await
        .context("failed to fetch queued (not payed) orders")?
        .into_iter()
        .map(|x| QueuedOrder {
            assigned_cheff: x.chef_number.map(|x| ChefCode(x as u8)),
            wait_number: WaitNumber(x.wait_number as u32),
            order: Order {
                id: x.id.into(),
                egg: x.egg,
                cheese: x.cheese,
                spicy_mayonnaise: x.spicy_mayonnaise,
                no_mayonnaise: x.no_mayonnaise,
                no_sauce: x.no_sauce,
                no_bonito: x.no_bonito,
                no_aonori: x.no_aonori,
                created_at: x.created_at,
            },
        })
        .collect();

        Ok(res)
    }

    async fn assign_order(&self, order_id: Id<Order>, chef_number: u8) -> Result<()> {
        use sea_orm::ActiveValue::*;

        entities::order_assigned::ActiveModel {
            order_id: Set(order_id.into()),
            chef_number: Set(chef_number as i32),
            created_at: NotSet,
        }
        .insert(&self.con)
        .await
        .context("failed to insert assigned order")?;

        Ok(())
    }

    async fn order_ready(&self, order_id: Id<Order>) -> Result<()> {
        use sea_orm::ActiveValue::*;

        entities::order_ready::ActiveModel {
            order_id: Set(order_id.into()),
            created_at: NotSet,
        }
        .insert(&self.con)
        .await
        .context("failed to insert assigned order")?;

        Ok(())
    }

    async fn get_ready_orders(&self) -> Result<Vec<QueuedOrder>> {
        let res = sqlx::query!(
            r#"
                select
                    order_queued.wait_number,
                    order_assigned.chef_number as "chef_number?",
                    orders.*
                from order_ready
                inner      join order_queued    on order_ready.order_id=order_queued.order_id
                inner      join orders          on order_ready.order_id=orders.id
                inner      join order_assigned  on order_ready.order_id=order_assigned.order_id
                left outer join order_delivered on order_ready.order_id=order_delivered.order_id
                where order_delivered.order_id is null
            "#
        )
        .fetch_all(&self.sqlx)
        .await
        .context("failed to fetch queued (not payed) orders")?
        .into_iter()
        .map(|x| QueuedOrder {
            assigned_cheff: x.chef_number.map(|x| ChefCode(x as u8)),
            wait_number: WaitNumber(x.wait_number as u32),
            order: Order {
                id: x.id.into(),
                egg: x.egg,
                cheese: x.cheese,
                spicy_mayonnaise: x.spicy_mayonnaise,
                no_mayonnaise: x.no_mayonnaise,
                no_sauce: x.no_sauce,
                no_bonito: x.no_bonito,
                no_aonori: x.no_aonori,
                created_at: x.created_at,
            },
        })
        .collect();

        Ok(res)
    }

    async fn order_delivered(&self, order_id: Id<Order>) -> Result<()> {
        use sea_orm::ActiveValue::*;

        entities::order_delivered::ActiveModel {
            order_id: Set(order_id.into()),
            created_at: NotSet,
        }
        .insert(&self.con)
        .await
        .context("failed to insert order_delivered")?;

        Ok(())
    }
}

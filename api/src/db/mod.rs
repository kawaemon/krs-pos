pub mod seaorm;

use anyhow::Result;
use std::future::Future;

use crate::model::{ChefCode, Id, Order, OrderGroup, PayedEvent, PriceTable, WaitNumber};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedOrder {
    pub wait_number: WaitNumber,
    pub assigned_cheff: Option<ChefCode>,
    pub order: Order,
}

pub trait Repository: Send + 'static {
    fn get_latest_price_table(&self) -> impl Future<Output = Result<PriceTable>> + Send;
    fn insert_price_table(&self, table: &PriceTable) -> impl Future<Output = Result<()>> + Send;

    fn insert_order_group(&self, order: &OrderGroup) -> impl Future<Output = Result<()>> + Send;
    fn cancel_order_group(&self, id: Id<OrderGroup>) -> impl Future<Output = Result<()>> + Send;

    fn insert_payed_event(&self, event: &PayedEvent) -> impl Future<Output = Result<()>> + Send;

    fn queue_orders_for_cook(
        &self,
        group: &Id<OrderGroup>,
    ) -> impl Future<Output = Result<Vec<WaitNumber>>> + Send;

    /// sort required
    fn get_queued_orders(&self) -> impl Future<Output = Result<Vec<QueuedOrder>>> + Send;

    fn assign_order(
        &self,
        order_id: Id<Order>,
        chef_number: u8,
    ) -> impl Future<Output = Result<()>> + Send;

    fn order_ready(&self, order_id: Id<Order>) -> impl Future<Output = Result<()>> + Send;

    fn get_ready_orders(&self) -> impl Future<Output = Result<Vec<QueuedOrder>>> + Send;

    fn order_delivered(&self, order_id: Id<Order>) -> impl Future<Output = Result<()>> + Send;
}

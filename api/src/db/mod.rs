pub mod seaorm;

use anyhow::Result;
use std::future::Future;

use crate::model::{Id, Order, OrderGroup, PayedEvent, PriceTable, WaitNumber};

pub trait Repository: Send + 'static {
    fn get_latest_price_table(&self) -> impl Future<Output = Result<PriceTable>> + Send;

    fn insert_order_group(&self, order: &OrderGroup) -> impl Future<Output = Result<()>> + Send;

    fn insert_payed_event(&self, event: &PayedEvent) -> impl Future<Output = Result<()>> + Send;

    fn queue_orders_for_cook(
        &self,
        group: &Id<OrderGroup>,
    ) -> impl Future<Output = Result<Vec<WaitNumber>>> + Send;

    fn get_queued_orders(&self) -> impl Future<Output = Result<Vec<(WaitNumber, Order)>>> + Send;
}

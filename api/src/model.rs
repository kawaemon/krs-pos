use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use uuid::Uuid;

pub struct Id<T>(Uuid, PhantomData<fn() -> T>);
impl<T> Id<T> {
    pub fn new() -> Self {
        // v7 is not usable as its marked as 'unstable feature'
        Id(Uuid::new_v4(), PhantomData)
    }
}

#[rustfmt::skip]
impl<T> Clone for Id<T> { fn clone(&self) -> Self { *self } }
impl<T> Copy for Id<T> {}
#[rustfmt::skip]
impl<T> Default for Id<T> { fn default() -> Self { Self::new() } }
impl<T> serde::Serialize for Id<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de, T> serde::Deserialize<'de> for Id<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(Uuid::deserialize(deserializer)?, PhantomData))
    }
}
impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}
impl<T> From<Uuid> for Id<T> {
    fn from(value: Uuid) -> Self {
        Id(value, PhantomData)
    }
}
impl<T> From<Id<T>> for Uuid {
    fn from(value: Id<T>) -> Self {
        value.0
    }
}

#[derive(Debug, Clone)]
pub struct PriceTable {
    pub id: Id<Self>,

    /// はしまき自体の金額
    pub base: u16,

    /// 目玉焼き
    pub egg: u16,
    /// チーズ
    pub cheese: u16,
    /// からしマヨネーズ
    pub spicy_mayonnaise: u16,

    /// マヨネーズ抜き
    pub no_mayonnaise: u16,
    /// ソース抜き
    pub no_sauce: u16,
    /// かつお節抜き
    pub no_bonito: u16,
    /// 青のりぬき
    pub no_aonori: u16,
}

pub struct OrderGroup {
    pub id: Id<Self>,
    pub orders: Vec<Order>,
    pub price_table: PriceTable,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Order {
    pub id: Id<Self>,

    /// 目玉焼き
    pub egg: bool,
    /// チーズ
    pub cheese: bool,
    /// からしマヨネーズ
    pub spicy_mayonnaise: bool,

    /// マヨネーズ抜き
    pub no_mayonnaise: bool,
    /// ソース抜き
    pub no_sauce: bool,
    /// かつお節抜き
    pub no_bonito: bool,
    /// 青のりぬき
    pub no_aonori: bool,

    pub created_at: DateTime<Utc>,
}

impl OrderGroup {
    pub fn total_price(&self) -> u16 {
        let PriceTable {
            id: _,
            base,
            egg,
            cheese,
            spicy_mayonnaise,
            no_mayonnaise,
            no_sauce,
            no_bonito,
            no_aonori,
        } = self.price_table;

        self.orders
            .iter()
            .map(|order| {
                macro_rules! p {
                    ($k:ident) => {
                        order.$k.then_some($k).unwrap_or_default()
                    };
                }

                base + p!(egg)
                    + p!(cheese)
                    + p!(spicy_mayonnaise)
                    + p!(no_mayonnaise)
                    + p!(no_sauce)
                    + p!(no_bonito)
                    + p!(no_aonori)
            })
            .sum()
    }
}

#[derive(Debug, Clone)]
pub struct PayedEvent {
    pub order_group_id: Id<OrderGroup>,
    pub payed_amount: u16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WaitNumber(pub u32);

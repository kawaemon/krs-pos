//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "order_group")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub price_table_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_payed::Entity")]
    OrderPayed,
    #[sea_orm(has_many = "super::orders::Entity")]
    Orders,
    #[sea_orm(
        belongs_to = "super::price::Entity",
        from = "Column::PriceTableId",
        to = "super::price::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Price,
}

impl Related<super::order_payed::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderPayed.def()
    }
}

impl Related<super::orders::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Orders.def()
    }
}

impl Related<super::price::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Price.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

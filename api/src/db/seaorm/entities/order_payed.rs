//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "order_payed")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub order_group_id: Uuid,
    pub payed_amount: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::order_group::Entity",
        from = "Column::OrderGroupId",
        to = "super::order_group::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    OrderGroup,
}

impl Related<super::order_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
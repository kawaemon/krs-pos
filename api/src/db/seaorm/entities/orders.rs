//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "orders")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub group_id: Uuid,
    pub egg: bool,
    pub cheese: bool,
    pub spicy_mayonnaise: bool,
    pub no_mayonnaise: bool,
    pub no_sauce: bool,
    pub no_bonito: bool,
    pub no_aonori: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_assigned::Entity")]
    OrderAssigned,
    #[sea_orm(has_many = "super::order_delivered::Entity")]
    OrderDelivered,
    #[sea_orm(
        belongs_to = "super::order_group::Entity",
        from = "Column::GroupId",
        to = "super::order_group::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    OrderGroup,
    #[sea_orm(has_many = "super::order_queued::Entity")]
    OrderQueued,
    #[sea_orm(has_many = "super::order_ready::Entity")]
    OrderReady,
}

impl Related<super::order_assigned::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderAssigned.def()
    }
}

impl Related<super::order_delivered::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderDelivered.def()
    }
}

impl Related<super::order_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderGroup.def()
    }
}

impl Related<super::order_queued::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderQueued.def()
    }
}

impl Related<super::order_ready::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderReady.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

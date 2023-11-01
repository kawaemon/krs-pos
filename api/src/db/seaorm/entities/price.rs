//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.4

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "price")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub base: i32,
    pub egg: i32,
    pub cheese: i32,
    pub spicy_mayonnaise: i32,
    pub no_mayonnaise: i32,
    pub no_sauce: i32,
    pub no_bonito: i32,
    pub no_aonori: i32,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::order_group::Entity")]
    OrderGroup,
}

impl Related<super::order_group::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderGroup.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

use crate::journal::{self, journal_user};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Hash, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
  #[sea_orm(primary_key, auto_increment = false)]
  pub id: Uuid,
  #[sea_orm(unique, indexed)]
  pub name: String,
  #[sea_orm(indexed)]
  pub role: Role,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[derive(
  Debug,
  Clone,
  Copy,
  Hash,
  Eq,
  PartialEq,
  Ord,
  PartialOrd,
  strum_macros::Display,
  Serialize,
  Deserialize,
  EnumIter,
  DeriveActiveEnum,
)]
#[sea_orm(rs_type = "String", db_type = "String(Some(1))")]
pub enum Role {
  #[sea_orm(string_value = "U")]
  User,
  #[sea_orm(string_value = "A")]
  Admin,
}

impl Related<journal::Entity> for Entity {
  fn to() -> RelationDef {
    journal_user::Relation::Journal.def()
  }

  fn via() -> Option<RelationDef> {
    Some(journal_user::Relation::User.def().rev())
  }
}

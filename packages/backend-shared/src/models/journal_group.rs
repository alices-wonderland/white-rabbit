use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "journals_groups")]
pub struct Model {
  #[sea_orm(primary_key)]
  pub journal_id: i32,
  #[sea_orm(primary_key)]
  pub group_id: i32,
  #[sea_orm(primary_key)]
  pub is_admin: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
  #[sea_orm(
    belongs_to = "super::Group",
    from = "Column::GroupId",
    to = "super::group::Column::Id",
    on_update = "Cascade",
    on_delete = "Cascade"
  )]
  Group,
  #[sea_orm(
    belongs_to = "super::Journal",
    from = "Column::JournalId",
    to = "super::journal::Column::Id",
    on_update = "Cascade",
    on_delete = "Cascade"
  )]
  Journal,
}

impl ActiveModelBehavior for ActiveModel {}

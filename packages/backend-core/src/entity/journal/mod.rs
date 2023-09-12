mod command;
mod database;
mod query;

use crate::entity::{
  journal_tag, FIELD_DESCRIPTION, FIELD_NAME, FIELD_TAGS, FIELD_TAG_EACH, FIELD_UNIT,
  MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH, MAX_SHORT_TEXT_LENGTH, MAX_TAGS_LENGTH, MIN_NAME_LENGTH,
  MIN_SHORT_TEXT_LENGTH,
};
pub use command::*;
pub use database::*;
use itertools::Itertools;
pub use query::*;
use sea_orm::entity::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, DbConn, EntityTrait, IntoActiveModel, QuerySelect};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const TYPE: &str = "Journal";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Root {
  pub id: Uuid,
  pub name: String,
  pub description: String,
  pub unit: String,
  pub tags: HashSet<String>,
}

impl Root {
  pub fn new(
    id: Option<Uuid>,
    name: impl ToString,
    description: impl ToString,
    unit: impl ToString,
    tags: impl IntoIterator<Item = impl ToString>,
  ) -> crate::Result<Root> {
    let name = name.to_string().trim().to_string();
    let description = description.to_string().trim().to_string();
    let unit = unit.to_string().trim().to_string();
    let mut trimmed_tags = HashSet::new();

    for tag in tags {
      let tag = tag.to_string().trim().to_string();
      if tag.len() < MIN_SHORT_TEXT_LENGTH || tag.len() > MAX_SHORT_TEXT_LENGTH {
        return Err(crate::Error::OutOfRange {
          typ: TYPE.to_string(),
          field: FIELD_TAG_EACH.to_string(),
          start: Some(MIN_SHORT_TEXT_LENGTH.to_string()),
          end: Some(MAX_SHORT_TEXT_LENGTH.to_string()),
        });
      }

      trimmed_tags.insert(tag);
    }

    if name.len() < MIN_NAME_LENGTH || name.len() > MAX_NAME_LENGTH {
      Err(crate::Error::OutOfRange {
        typ: TYPE.to_string(),
        field: FIELD_NAME.to_string(),
        start: Some(MIN_NAME_LENGTH.to_string()),
        end: Some(MAX_NAME_LENGTH.to_string()),
      })
    } else if description.len() > MAX_DESCRIPTION_LENGTH {
      Err(crate::Error::OutOfRange {
        typ: TYPE.to_string(),
        field: FIELD_DESCRIPTION.to_string(),
        start: None,
        end: Some(MAX_NAME_LENGTH.to_string()),
      })
    } else if unit.len() < MIN_SHORT_TEXT_LENGTH || unit.len() > MAX_SHORT_TEXT_LENGTH {
      Err(crate::Error::OutOfRange {
        typ: TYPE.to_string(),
        field: FIELD_UNIT.to_string(),
        start: Some(MIN_SHORT_TEXT_LENGTH.to_string()),
        end: Some(MAX_SHORT_TEXT_LENGTH.to_string()),
      })
    } else if trimmed_tags.len() > MAX_TAGS_LENGTH {
      Err(crate::Error::OutOfRange {
        typ: TYPE.to_string(),
        field: FIELD_TAGS.to_string(),
        start: None,
        end: Some(MAX_TAGS_LENGTH.to_string()),
      })
    } else {
      Ok(Root { id: id.unwrap_or_else(Uuid::new_v4), name, description, unit, tags: trimmed_tags })
    }
  }

  pub async fn from_model(
    db: &DbConn,
    models: impl IntoIterator<Item = Model>,
  ) -> anyhow::Result<Vec<Root>> {
    let mut roots = Vec::new();
    let mut ids = HashSet::<Uuid>::new();

    for model in models {
      roots.push(Root {
        id: model.id,
        name: model.name,
        description: model.description,
        unit: model.unit,
        tags: HashSet::default(),
      });
      ids.insert(model.id);
    }

    let tags = journal_tag::Entity::find()
      .filter(journal_tag::Column::JournalId.is_in(ids))
      .all(db)
      .await?
      .into_iter()
      .into_group_map_by(|tag| tag.journal_id)
      .into_iter()
      .map(|(k, v)| (k, v.into_iter().map(|m| m.tag).collect::<HashSet<_>>()))
      .collect::<HashMap<_, _>>();

    Ok(
      roots
        .into_iter()
        .map(|root| Self { tags: tags.get(&root.id).cloned().unwrap_or_default(), ..root })
        .collect(),
    )
  }

  pub async fn save(db: &DbConn, roots: impl IntoIterator<Item = Root>) -> anyhow::Result<()> {
    let mut model_ids = HashSet::new();
    let mut models: Vec<ActiveModel> = vec![];
    let mut tags: Vec<journal_tag::ActiveModel> = vec![];

    for ref root in roots {
      model_ids.insert(root.id);
      models.push(
        Model {
          id: root.id,
          name: root.name.to_string(),
          description: root.description.to_string(),
          unit: root.unit.to_string(),
        }
        .into_active_model(),
      );
      for tag in &root.tags {
        tags.push(
          journal_tag::Model { journal_id: root.id, tag: tag.to_string() }.into_active_model(),
        );
      }
    }

    journal_tag::Entity::delete_many()
      .filter(journal_tag::Column::JournalId.is_in(model_ids.clone()))
      .exec(db)
      .await?;

    let mut on_conflict = OnConflict::column(Column::Id);
    on_conflict.update_columns([Column::Name, Column::Description, Column::Unit]);

    Entity::insert_many(models).on_conflict(on_conflict).exec(db).await?;
    journal_tag::Entity::insert_many(tags).exec(db).await?;

    Ok(())
  }

  pub async fn delete(db: &DbConn, ids: impl IntoIterator<Item = Uuid>) -> anyhow::Result<()> {
    Entity::delete_many().filter(Column::Id.is_in(ids)).exec(db).await?;
    Ok(())
  }

  pub async fn find_one(db: &DbConn, query: Option<Query>) -> anyhow::Result<Option<Root>> {
    Ok(Self::find_all(db, query, Some(1)).await?.into_iter().next())
  }

  pub async fn find_all(
    db: &DbConn,
    query: Option<Query>,
    limit: Option<u64>,
  ) -> anyhow::Result<Vec<Root>> {
    let select =
      if let Some(query) = query { Entity::find().filter(query) } else { Entity::find() };
    let models = select.limit(limit).all(db).await?;
    Self::from_model(db, models).await
  }
}

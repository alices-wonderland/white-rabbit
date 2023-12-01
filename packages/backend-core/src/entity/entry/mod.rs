mod command;
mod database;
mod query;

pub use command::*;
pub use database::*;
pub use query::*;

use crate::entity::{
  account, entry_item, entry_tag, journal, normalize_description, normalize_name, normalize_tags,
  FIELD_ID, FIELD_JOURNAL, FIELD_NAME, FIELD_TYPE,
};
use chrono::NaiveDate;
use itertools::Itertools;
use rust_decimal::Decimal;
use sea_orm::sea_query::{BinOper, Expr, OnConflict};
use sea_orm::{
  ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, Order, QueryFilter, QueryOrder,
  QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub const TYPE: &str = "Entry";
pub const FIELD_AMOUNT: &str = "amount";
pub const FIELD_PRICE: &str = "price";
pub const FIELD_DATE: &str = "date";

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
  pub account: Uuid,
  pub amount: Decimal,
  #[serde(default)]
  pub price: Option<Decimal>,
}

#[derive(Debug, Default)]
pub struct Builder {
  id: Option<Uuid>,
  journal_id: Option<Uuid>,
  name: String,
  description: String,
  typ: Option<Type>,
  date: NaiveDate,
  tags: HashSet<String>,
  items: Vec<Item>,
}

impl From<Root> for Builder {
  fn from(value: Root) -> Self {
    Builder {
      id: Some(value.id),
      journal_id: Some(value.journal_id),
      name: value.name,
      description: value.description,
      typ: Some(value.typ),
      date: value.date,
      tags: value.tags,
      items: value.items,
    }
  }
}

impl Builder {
  pub fn build(self, accounts: &HashMap<Uuid, account::Root>) -> crate::Result<Root> {
    let name = normalize_name(TYPE, self.name)?;
    let description = normalize_description(TYPE, self.description)?;
    let tags = normalize_tags(TYPE, self.tags)?;
    let mut filtered_items = HashMap::new();
    let journal_id = self.journal_id.ok_or_else(|| crate::Error::RequiredField {
      typ: TYPE.to_string(),
      field: FIELD_JOURNAL.to_string(),
    })?;

    for Item { account, amount, price } in self.items {
      if let Some(account) = accounts.get(&account) {
        if account.journal_id != journal_id {
          return Err(crate::Error::NotFound {
            typ: account::TYPE.to_string(),
            values: vec![
              (FIELD_JOURNAL.to_string(), journal_id.to_string()),
              (FIELD_ID.to_string(), account.id.to_string()),
            ],
          });
        } else if amount.is_sign_negative() {
          return Err(crate::Error::OutOfRange {
            typ: TYPE.to_string(),
            field: FIELD_AMOUNT.to_string(),
            start: Some(0.to_string()),
            end: None,
          });
        } else if let Some(price) = price {
          if price.is_sign_negative() {
            return Err(crate::Error::OutOfRange {
              typ: TYPE.to_string(),
              field: FIELD_PRICE.to_string(),
              start: Some(0.to_string()),
              end: None,
            });
          }
        }

        filtered_items.insert(account.id, (amount, price));
      } else {
        return Err(crate::Error::NotFound {
          typ: account::TYPE.to_string(),
          values: vec![(FIELD_ID.to_string(), account.to_string())],
        });
      }
    }

    Ok(Root {
      id: self.id.unwrap_or_else(Uuid::new_v4),
      journal_id,
      name,
      description,
      typ: self.typ.ok_or_else(|| crate::Error::RequiredField {
        typ: TYPE.to_string(),
        field: FIELD_TYPE.to_string(),
      })?,
      date: self.date,
      tags,
      items: filtered_items
        .into_iter()
        .map(|(account, (amount, price))| Item { account, amount, price })
        .collect(),
    })
  }

  pub fn id(self, id: Uuid) -> Builder {
    Builder { id: Some(id), ..self }
  }

  pub fn journal_id(self, journal_id: Uuid) -> Builder {
    Builder { journal_id: Some(journal_id), ..self }
  }

  pub fn name(self, name: impl ToString) -> Builder {
    Builder { name: name.to_string(), ..self }
  }

  pub fn description(self, description: impl ToString) -> Builder {
    Builder { description: description.to_string(), ..self }
  }

  pub fn typ(self, typ: Type) -> Builder {
    Builder { typ: Some(typ), ..self }
  }

  pub fn date(self, date: NaiveDate) -> Builder {
    Builder { date, ..self }
  }
  pub fn tags(self, tags: impl IntoIterator<Item = impl ToString>) -> Builder {
    Builder { tags: tags.into_iter().map(|s| s.to_string()).collect(), ..self }
  }

  pub fn items(self, items: Vec<Item>) -> Builder {
    Builder { items, ..self }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
  #[serde(rename = "name")]
  Name,
  #[serde(rename = "-name")]
  MinusName,
  #[serde(rename = "date")]
  Date,
  #[serde(rename = "-date")]
  MinusDate,
}

impl From<Sort> for (Column, Order) {
  fn from(value: Sort) -> Self {
    match value {
      Sort::Name => (Column::Name, Order::Asc),
      Sort::MinusName => (Column::Name, Order::Desc),
      Sort::Date => (Column::Date, Order::Asc),
      Sort::MinusDate => (Column::Date, Order::Desc),
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Root {
  pub id: Uuid,
  pub journal_id: Uuid,
  pub name: String,
  pub description: String,
  #[serde(rename = "type")]
  pub typ: Type,
  pub date: NaiveDate,
  pub tags: HashSet<String>,
  pub items: Vec<Item>,
}

impl super::Root for Root {
  fn id(&self) -> Uuid {
    self.id
  }
}

impl Root {
  pub async fn from_model(
    db: &impl ConnectionTrait,
    models: impl IntoIterator<Item = Model>,
  ) -> crate::Result<Vec<Root>> {
    let mut roots = Vec::new();
    let mut ids = HashSet::<Uuid>::new();

    for model in models {
      roots.push(Root {
        id: model.id,
        journal_id: model.journal_id,
        name: model.name,
        description: model.description,
        typ: model.typ,
        date: model.date,
        tags: HashSet::default(),
        items: Vec::default(),
      });
      ids.insert(model.id);
    }

    let tags = entry_tag::Entity::find()
      .filter(entry_tag::Column::EntryId.is_in(ids.clone()))
      .all(db)
      .await?
      .into_iter()
      .into_group_map_by(|tag| tag.entry_id)
      .into_iter()
      .map(|(k, v)| (k, v.into_iter().map(|m| m.tag).collect::<HashSet<_>>()))
      .collect::<HashMap<_, _>>();

    let items = entry_item::Entity::find()
      .filter(entry_item::Column::EntryId.is_in(ids))
      .all(db)
      .await?
      .into_iter()
      .into_group_map_by(|item| item.entry_id)
      .into_iter()
      .map(|(k, v)| {
        (
          k,
          v.into_iter()
            .map(|item| Item { account: item.account_id, amount: item.amount, price: item.price })
            .collect::<Vec<_>>(),
        )
      })
      .collect::<HashMap<_, _>>();

    Ok(
      roots
        .into_iter()
        .map(|root| Self {
          tags: tags.get(&root.id).cloned().unwrap_or_default(),
          items: items.get(&root.id).into_iter().flatten().copied().collect(),
          ..root
        })
        .collect(),
    )
  }

  pub async fn save(
    db: &impl ConnectionTrait,
    roots: impl IntoIterator<Item = Root>,
  ) -> crate::Result<Vec<Root>> {
    let roots: Vec<Root> = roots.into_iter().collect();
    if roots.is_empty() {
      return Ok(roots);
    }

    let mut model_ids = HashSet::new();
    let mut models: Vec<ActiveModel> = vec![];
    let mut tags: Vec<entry_tag::ActiveModel> = vec![];
    let mut items: Vec<entry_item::ActiveModel> = vec![];

    for ref root in roots {
      model_ids.insert(root.id);
      models.push(
        Model {
          id: root.id,
          journal_id: root.journal_id,
          name: root.name.to_string(),
          description: root.description.to_string(),
          typ: root.typ,
          date: root.date,
        }
        .into_active_model(),
      );
      for tag in &root.tags {
        tags.push(entry_tag::Model { entry_id: root.id, tag: tag.to_string() }.into_active_model());
      }
      for Item { account, amount, price } in &root.items {
        items.push(
          entry_item::Model {
            entry_id: root.id,
            account_id: *account,
            amount: *amount,
            price: *price,
          }
          .into_active_model(),
        )
      }
    }

    entry_tag::Entity::delete_many()
      .filter(entry_tag::Column::EntryId.is_in(model_ids.clone()))
      .exec(db)
      .await?;

    entry_item::Entity::delete_many()
      .filter(entry_item::Column::EntryId.is_in(model_ids.clone()))
      .exec(db)
      .await?;

    let mut on_conflict = OnConflict::column(Column::Id);
    on_conflict.update_columns([
      Column::JournalId,
      Column::Name,
      Column::Description,
      Column::Typ,
      Column::Date,
    ]);

    // Update unique column name to temp value
    Entity::update_many()
      .col_expr(
        Column::Name,
        Expr::col((Entity, Column::Name)).binary(BinOper::Custom("||"), Expr::current_timestamp()),
      )
      .filter(Column::Id.is_in(model_ids.clone()))
      .exec(db)
      .await?;

    Entity::insert_many(models).on_conflict(on_conflict).exec(db).await?;

    if !tags.is_empty() {
      entry_tag::Entity::insert_many(tags).exec(db).await?;
    }

    if !items.is_empty() {
      entry_item::Entity::insert_many(items).exec(db).await?;
    }

    Self::find_all(db, Some(Query { id: model_ids, ..Default::default() }), None, None).await
  }

  pub async fn delete(
    db: &impl ConnectionTrait,
    ids: impl IntoIterator<Item = Uuid>,
  ) -> crate::Result<()> {
    Entity::delete_many().filter(Column::Id.is_in(ids)).exec(db).await?;
    Ok(())
  }

  pub async fn find_one(
    db: &impl ConnectionTrait,
    query: Option<Query>,
  ) -> crate::Result<Option<Root>> {
    Ok(Self::find_all(db, query, Some(1), None).await?.into_iter().next())
  }

  pub async fn find_all(
    db: &impl ConnectionTrait,
    query: Option<Query>,
    limit: Option<u64>,
    sort: Option<Sort>,
  ) -> crate::Result<Vec<Root>> {
    let select =
      if let Some(query) = query { Entity::find().filter(query) } else { Entity::find() };
    let select = if let Some(sort) = sort {
      let (field, order) = Into::<(Column, Order)>::into(sort);
      select.order_by(field, order)
    } else {
      select
    };
    let models = select.limit(limit).all(db).await?;
    Self::from_model(db, models).await
  }

  pub async fn handle(db: &impl ConnectionTrait, command: Command) -> crate::Result<Vec<Root>> {
    match command {
      Command::Create(command) => Self::create(db, vec![command]).await,
      Command::Update(command) => Self::update(db, vec![command]).await,
      Command::Delete(CommandDelete { id }) => {
        Self::delete(db, id).await?;
        Ok(Vec::default())
      }
      Command::Batch(CommandBatch { create, update, delete }) => {
        let mut ids = HashSet::<Uuid>::new();

        Self::delete(db, delete).await?;

        for root in Self::update(db, update).await? {
          ids.insert(root.id);
        }

        for root in Self::create(db, create).await? {
          ids.insert(root.id);
        }

        Self::find_all(db, Some(Query { id: ids, ..Default::default() }), None, None).await
      }
    }
  }

  pub async fn create(
    db: &impl ConnectionTrait,
    commands: Vec<CommandCreate>,
  ) -> crate::Result<Vec<Root>> {
    if commands.is_empty() {
      return Ok(vec![]);
    }

    let journals = journal::Root::find_all(
      db,
      Some(journal::Query {
        id: commands.iter().map(|c| c.journal_id).collect(),
        ..Default::default()
      }),
      None,
      None,
    )
    .await?
    .into_iter()
    .map(|journal| (journal.id, journal))
    .collect::<HashMap<_, _>>();

    let accounts = account::Root::find_all(
      db,
      Some(account::Query {
        id: commands.iter().flat_map(|c| c.items.iter()).map(|item| item.account).collect(),
        ..Default::default()
      }),
      None,
      None,
    )
    .await?
    .into_iter()
    .map(|account| (account.id, account))
    .collect::<HashMap<_, _>>();

    let names_by_journal = commands
      .iter()
      .map(|command| (command.journal_id, command.name.clone()))
      .into_group_map_by(|p| p.0)
      .into_iter()
      .map(|(k, vec)| (k, vec.into_iter().map(|(_, v)| v).unique().collect::<Vec<_>>()))
      .collect::<HashMap<_, _>>();

    for (journal_id, names) in names_by_journal {
      if !journals.contains_key(&journal_id) {
        return Err(crate::Error::NotFound {
          typ: journal::TYPE.to_string(),
          values: vec![(FIELD_ID.to_string(), journal_id.to_string())],
        });
      }

      let existings = Root::find_all(
        db,
        Some(Query {
          journal_id: HashSet::from_iter([journal_id]),
          name: HashSet::from_iter(names),
          ..Default::default()
        }),
        None,
        None,
      )
      .await?;
      if !existings.is_empty() {
        let existing_names = existings.iter().map(|model| model.name.clone()).sorted().join(", ");

        return Err(crate::Error::ExistingEntity {
          typ: TYPE.to_string(),
          values: vec![
            (FIELD_JOURNAL.to_string(), journal_id.to_string()),
            (FIELD_NAME.to_string(), existing_names),
          ],
        });
      }
    }

    let roots: Vec<_> = commands
      .into_iter()
      .filter_map(|command| {
        if let Some(journal) = journals.get(&command.journal_id) {
          Some(
            Builder::default()
              .journal_id(journal.id)
              .name(command.name)
              .description(command.description)
              .typ(command.typ)
              .date(command.date)
              .tags(command.tags)
              .items(command.items)
              .build(&accounts),
          )
        } else {
          None
        }
      })
      .try_collect()?;

    Self::save(db, roots).await
  }

  async fn do_update(
    db: &impl ConnectionTrait,
    journal: &journal::Root,
    entries: &mut HashMap<Uuid, Root>,
    commands: &[CommandUpdate],
  ) -> crate::Result<Vec<Root>> {
    if commands.is_empty() {
      return Ok(vec![]);
    }

    let accounts: HashMap<Uuid, account::Root> = account::Root::find_all(
      db,
      Some(account::Query { journal_id: HashSet::from_iter([journal.id]), ..Default::default() }),
      None,
      None,
    )
    .await?
    .into_iter()
    .map(|model| (model.id, model))
    .collect();

    let mut name_mappings = HashMap::new();
    let mut model_ids = HashSet::new();

    for command in commands {
      if !command.name.is_empty() {
        name_mappings.insert(command.name.clone(), command.id);
      }
      model_ids.insert(command.id);
    }

    let existings_by_name = if name_mappings.is_empty() {
      vec![]
    } else {
      Self::find_all(
        db,
        Some(Query {
          journal_id: HashSet::from_iter([journal.id]),
          name: name_mappings.keys().cloned().collect(),
          ..Default::default()
        }),
        None,
        None,
      )
      .await?
    };

    for model in existings_by_name {
      if let Some(updating_id) = name_mappings.get(&model.name) {
        if updating_id != &model.id && !name_mappings.values().contains(&model.id) {
          return Err(crate::Error::ExistingEntity {
            typ: TYPE.to_string(),
            values: vec![
              (FIELD_JOURNAL.to_string(), journal.id.to_string()),
              (FIELD_NAME.to_string(), model.name.clone()),
            ],
          });
        }
      }
    }

    let mut updated = HashMap::new();
    for command in commands {
      let model = entries.get(&command.id).ok_or_else(|| crate::Error::NotFound {
        typ: TYPE.to_string(),
        values: vec![(FIELD_ID.to_string(), command.id.to_string())],
      })?;

      if command.name.is_empty()
        && command.description.is_none()
        && command.typ.is_none()
        && command.date.is_none()
        && command.tags.is_none()
        && command.items.is_empty()
      {
        continue;
      }

      let mut builder = Builder::from(model.clone());
      if !command.name.is_empty() {
        builder = builder.name(command.name.clone());
      }

      if let Some(description) = &command.description {
        builder = builder.description(description.clone());
      }

      if let Some(typ) = &command.typ {
        builder = builder.typ(*typ);
      }

      if let Some(date) = &command.date {
        builder = builder.date(*date);
      }

      if let Some(tags) = &command.tags {
        builder = builder.tags(tags.clone());
      }

      if !command.items.is_empty() {
        builder = builder.items(command.items.clone());
      }

      let model = builder.build(&accounts)?;

      entries.insert(model.id, model.clone());
      updated.insert(model.id, model);
    }
    Ok(updated.into_values().collect())
  }

  pub async fn update(
    db: &impl ConnectionTrait,
    commands: Vec<CommandUpdate>,
  ) -> crate::Result<Vec<Root>> {
    let model_ids = commands.iter().map(|command| command.id).collect::<HashSet<_>>();
    let mut models =
      Self::find_all(db, Some(Query { id: model_ids, ..Default::default() }), None, None)
        .await?
        .into_iter()
        .map(|model| (model.id, model))
        .collect::<HashMap<_, _>>();
    let journal_ids = models.values().map(|model| model.journal_id).collect::<HashSet<_>>();
    let journals = journal::Root::find_all(
      db,
      Some(journal::Query { id: journal_ids, ..Default::default() }),
      None,
      None,
    )
    .await?
    .into_iter()
    .map(|model| (model.id, model))
    .collect::<HashMap<_, _>>();

    let commands_by_journal: HashMap<Uuid, Vec<CommandUpdate>> = commands
      .into_iter()
      .filter_map(|command| {
        if let Some(model) = models.get(&command.id) {
          if let Some(journal) = journals.get(&model.journal_id) {
            return Some((journal.id, command));
          }
        }
        None
      })
      .fold(HashMap::new(), |mut acc, (journal_id, command)| {
        if let Some(commands) = acc.get_mut(&journal_id) {
          commands.push(command);
        } else {
          acc.insert(journal_id, vec![command]);
        }
        acc
      });

    let mut updated = HashMap::new();
    for (journal_id, commands) in commands_by_journal {
      if let Some(journal) = journals.get(&journal_id) {
        for model in Self::do_update(db, journal, &mut models, &commands).await? {
          updated.insert(model.id, model);
        }
      }
    }

    Self::save(db, updated.into_values()).await
  }
}

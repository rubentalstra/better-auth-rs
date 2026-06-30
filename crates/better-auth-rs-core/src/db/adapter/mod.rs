//! Upstream reference: db/adapter/index.ts  (in progress — see note below)
//!
//! The storage layer's contract. This file ports the **Options-independent** surface: the query
//! model (`Where`/operators/joins/sort), the per-operation argument structs, and the low-level
//! [`CustomAdapter`] trait that every storage backend implements (operating on dynamic [`Row`]s).
//!
//! **Deferred** (gated on `BetterAuthOptions`, ported with `types/init-options.ts`, and on the
//! adapter factory in `db/adapter/factory.ts`): `DBAdapterFactoryConfig`, the high-level `DBAdapter`
//! / `DBTransactionAdapter` (the factory-wrapped adapter, with transactions), `DBAdapterInstance`,
//! the `JoinOption` (pre-factory join form), and `DBAdapterDebugLogOption`. The manifest row for
//! `index.ts` stays `building` until those land.
//!
//! Idiomatic choices: the dynamic record is a [`Row`] (`field -> DbValue`) rather than a generic
//! `T` (the TS generics are caller-side typing — entity structs convert to/from `Row` at the
//! boundary). The `where` array is named `conditions` (`where` is a Rust keyword).

use std::collections::BTreeMap;

use async_trait::async_trait;

use super::types::{BetterAuthDbSchema, DbValue};

/// A dynamic database record: field name → value.
pub type Row = BTreeMap<String, DbValue>;

/// An error from a storage backend.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The backend reported an error (connection, query, serialization, …).
    #[error("adapter backend error: {0}")]
    Backend(String),
}

/// Result alias for adapter operations.
pub type AdapterResult<T> = Result<T, AdapterError>;

/// A `where`-clause comparison operator (`WhereOperator`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhereOperator {
    /// `=` (the default).
    #[default]
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Lte,
    /// `>`
    Gt,
    /// `>=`
    Gte,
    /// `IN (...)`
    In,
    /// `NOT IN (...)`
    NotIn,
    /// substring match
    Contains,
    /// prefix match
    StartsWith,
    /// suffix match
    EndsWith,
}

/// How a [`Where`] combines with its neighbors (`connector`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Connector {
    /// `AND` (the default).
    #[default]
    And,
    /// `OR`
    Or,
}

/// Case sensitivity for string comparisons (`mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchMode {
    /// Case-sensitive (the default).
    #[default]
    Sensitive,
    /// Case-insensitive (equality + `contains`/`starts_with`/`ends_with`).
    Insensitive,
}

/// Sort direction for `findMany`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// A sort specification (`sortBy`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortBy {
    /// The field to sort by.
    pub field: String,
    /// The direction.
    pub direction: SortDirection,
}

/// A single `where` condition (`Where`). All fields are populated — this is also the "cleaned"
/// form the [`CustomAdapter`] receives (upstream `CleanedWhere = Required<Where>`).
#[derive(Debug, Clone, PartialEq)]
pub struct Where {
    /// The field to compare.
    pub field: String,
    /// The comparison value.
    pub value: DbValue,
    /// The comparison operator.
    pub operator: WhereOperator,
    /// How this condition combines with others.
    pub connector: Connector,
    /// String comparison case sensitivity.
    pub mode: MatchMode,
}

impl Where {
    /// An equality condition (`field = value`) with default connector/mode.
    #[must_use]
    pub fn eq(field: impl Into<String>, value: impl Into<DbValue>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            operator: WhereOperator::Eq,
            connector: Connector::And,
            mode: MatchMode::Sensitive,
        }
    }

    /// A condition with an explicit operator (default connector/mode).
    #[must_use]
    pub fn new(
        field: impl Into<String>,
        operator: WhereOperator,
        value: impl Into<DbValue>,
    ) -> Self {
        Self {
            operator,
            ..Self::eq(field, value)
        }
    }

    /// Set the connector (builder).
    #[must_use]
    pub fn with_connector(mut self, connector: Connector) -> Self {
        self.connector = connector;
        self
    }

    /// Set the match mode (builder).
    #[must_use]
    pub fn with_mode(mut self, mode: MatchMode) -> Self {
        self.mode = mode;
        self
    }
}

/// The relation type of a join (`relation`), determining the output shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelationType {
    /// A single related object.
    OneToOne,
    /// A list of related objects (the default).
    #[default]
    OneToMany,
    /// A list of related objects via a join table.
    ManyToMany,
}

/// The joining columns (`on`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinOn {
    /// Column on the main table.
    pub from: String,
    /// Column on the joined table.
    pub to: String,
}

/// A single join in a [`JoinConfig`] (the factory-resolved form of a join request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinConfigEntry {
    /// The joining columns.
    pub on: JoinOn,
    /// Max rows to return (a `unique` relation forces 1). Default 100 at the factory.
    pub limit: Option<u64>,
    /// The relation type.
    pub relation: RelationType,
}

/// Resolved join configuration (`JoinConfig`): joined-model name → [`JoinConfigEntry`].
pub type JoinConfig = BTreeMap<String, JoinConfigEntry>;

/// A schema-generation result (`DBAdapterSchemaCreation`) for the `generate` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbAdapterSchemaCreation {
    /// Code to write into the file.
    pub code: String,
    /// Destination path (relative to the project cwd).
    pub path: String,
    /// Append to the file if it exists (ignored when `overwrite` is set).
    pub append: Option<bool>,
    /// Overwrite the file if it exists.
    pub overwrite: Option<bool>,
}

/// Arguments to [`CustomAdapter::create`].
#[derive(Debug, Clone, Default)]
pub struct CreateArgs {
    /// The (default) model name.
    pub model: String,
    /// The record to insert.
    pub data: Row,
    /// Columns to return (all if `None`).
    pub select: Option<Vec<String>>,
}

/// Arguments to [`CustomAdapter::update`].
#[derive(Debug, Clone, Default)]
pub struct UpdateArgs {
    /// The model name.
    pub model: String,
    /// The match conditions (upstream `where`).
    pub conditions: Vec<Where>,
    /// The fields to update.
    pub update: Row,
}

/// Arguments to [`CustomAdapter::update_many`].
pub type UpdateManyArgs = UpdateArgs;

/// Arguments to [`CustomAdapter::find_one`].
#[derive(Debug, Clone, Default)]
pub struct FindOneArgs {
    /// The model name.
    pub model: String,
    /// The match conditions.
    pub conditions: Vec<Where>,
    /// Columns to return (all if `None`).
    pub select: Option<Vec<String>>,
    /// Optional joins.
    pub join: Option<JoinConfig>,
}

/// Arguments to [`CustomAdapter::find_many`].
#[derive(Debug, Clone, Default)]
pub struct FindManyArgs {
    /// The model name.
    pub model: String,
    /// The match conditions (empty = no filter).
    pub conditions: Vec<Where>,
    /// Max rows to return (`None` = unlimited).
    pub limit: Option<u64>,
    /// Columns to return (all if `None`).
    pub select: Option<Vec<String>>,
    /// Optional sort.
    pub sort_by: Option<SortBy>,
    /// Rows to skip.
    pub offset: Option<u64>,
    /// Optional joins.
    pub join: Option<JoinConfig>,
}

/// Arguments to [`CustomAdapter::delete`] / [`CustomAdapter::delete_many`].
#[derive(Debug, Clone, Default)]
pub struct DeleteArgs {
    /// The model name.
    pub model: String,
    /// The match conditions.
    pub conditions: Vec<Where>,
}

/// Arguments to [`CustomAdapter::count`].
#[derive(Debug, Clone, Default)]
pub struct CountArgs {
    /// The model name.
    pub model: String,
    /// The match conditions (empty = count all).
    pub conditions: Vec<Where>,
}

/// Arguments to [`CustomAdapter::consume_one`].
pub type ConsumeOneArgs = DeleteArgs;

/// Arguments to [`CustomAdapter::increment_one`].
#[derive(Debug, Clone, Default)]
pub struct IncrementOneArgs {
    /// The model name.
    pub model: String,
    /// The match conditions, acting as both selector and guard.
    pub conditions: Vec<Where>,
    /// Signed deltas to add per field (`field = field + delta`).
    pub increment: BTreeMap<String, f64>,
    /// Absolute values to set in the same operation.
    pub set: Option<Row>,
}

/// The low-level adapter a storage backend implements (`CustomAdapter`). It operates on dynamic
/// [`Row`]s and receives fully-populated [`Where`] conditions. The high-level, factory-wrapped
/// `DBAdapter` (transforms, id-generation, transactions) is layered on top later.
#[async_trait]
pub trait CustomAdapter: Send + Sync {
    /// Insert a record and return it.
    async fn create(&self, args: CreateArgs) -> AdapterResult<Row>;

    /// Update a single matching row; returns it, or `None` if none matched.
    async fn update(&self, args: UpdateArgs) -> AdapterResult<Option<Row>>;

    /// Update all matching rows; returns the count affected.
    async fn update_many(&self, args: UpdateManyArgs) -> AdapterResult<u64>;

    /// Find a single matching row.
    async fn find_one(&self, args: FindOneArgs) -> AdapterResult<Option<Row>>;

    /// Find all matching rows.
    async fn find_many(&self, args: FindManyArgs) -> AdapterResult<Vec<Row>>;

    /// Delete all matching rows.
    async fn delete(&self, args: DeleteArgs) -> AdapterResult<()>;

    /// Delete all matching rows; returns the count affected.
    async fn delete_many(&self, args: DeleteArgs) -> AdapterResult<u64>;

    /// Count matching rows.
    async fn count(&self, args: CountArgs) -> AdapterResult<u64>;

    /// Atomically consume (delete-and-return) a single matching row.
    ///
    /// The default is a **non-atomic** find-then-delete; backends that can do this atomically
    /// (e.g. `DELETE ... RETURNING *`, or a global lock for an in-memory store) should override it.
    /// Implementations must delete at most one matching row.
    async fn consume_one(&self, args: ConsumeOneArgs) -> AdapterResult<Option<Row>> {
        let found = self
            .find_one(FindOneArgs {
                model: args.model.clone(),
                conditions: args.conditions.clone(),
                select: None,
                join: None,
            })
            .await?;
        if found.is_some() {
            self.delete(args).await?;
        }
        Ok(found)
    }

    /// Atomically apply signed numeric deltas (and optional absolute `set`s) to a single matching
    /// row, with `conditions` acting as both selector and guard; returns the updated row or `None`.
    ///
    /// The default is a **non-atomic** find-mutate-update; backends that can do this atomically
    /// (e.g. `UPDATE ... SET n = n + $d WHERE ... RETURNING *`, or a global lock) should override it.
    async fn increment_one(&self, args: IncrementOneArgs) -> AdapterResult<Option<Row>> {
        let found = self
            .find_one(FindOneArgs {
                model: args.model.clone(),
                conditions: args.conditions.clone(),
                select: None,
                join: None,
            })
            .await?;
        if found.is_none() {
            return Ok(None);
        }
        let mut update = Row::new();
        for (field, delta) in &args.increment {
            let current = found
                .as_ref()
                .and_then(|r| r.get(field))
                .map_or(0.0, |v| match v {
                    DbValue::Int(n) => *n as f64,
                    DbValue::Float(f) => *f,
                    _ => 0.0,
                });
            let next = current + delta;
            let value = if next.fract() == 0.0 {
                DbValue::Int(next as i64)
            } else {
                DbValue::Float(next)
            };
            update.insert(field.clone(), value);
        }
        if let Some(set) = &args.set {
            for (k, v) in set {
                update.insert(k.clone(), v.clone());
            }
        }
        self.update(UpdateArgs {
            model: args.model,
            conditions: args.conditions,
            update,
        })
        .await
    }

    /// Generate the backend's schema for the `generate` command. `None` by default.
    async fn create_schema(
        &self,
        _tables: &BetterAuthDbSchema,
        _file: Option<&str>,
    ) -> AdapterResult<Option<DbAdapterSchemaCreation>> {
        Ok(None)
    }
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod adapter_tests;

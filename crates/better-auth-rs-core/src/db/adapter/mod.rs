//! The database adapter interface (port of `@better-auth/core` `db/adapter/index.ts`).
//!
//! Every storage backend implements [`DatabaseAdapter`]: a model-string-keyed CRUD contract
//! driven by a portable [`Where`] predicate list, rather than per-query typed SQL. The default
//! `eq`/`AND`/`sensitive` values mean a [`Where`] is already the "cleaned" form better-auth
//! produces inside its adapter factory.

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::db::field::{DbValue, Row};

/// Comparison operator for a [`Where`] clause (port of `WhereOperator`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhereOperator {
    #[default]
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    NotIn,
    Contains,
    StartsWith,
    EndsWith,
}

/// How a clause combines with the previous one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Connector {
    #[default]
    And,
    Or,
}

/// Case sensitivity for string comparisons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MatchMode {
    #[default]
    Sensitive,
    Insensitive,
}

/// A single filter clause (port of `Where`/`CleanedWhere`). All fields are concrete: the
/// defaults (`Eq`, `And`, `Sensitive`) match better-auth's cleaned form.
#[derive(Clone, Debug, PartialEq)]
pub struct Where {
    pub field: String,
    pub value: DbValue,
    pub operator: WhereOperator,
    pub connector: Connector,
    pub mode: MatchMode,
}

impl Where {
    /// `field = value` (the common case).
    pub fn eq(field: impl Into<String>, value: impl Into<DbValue>) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            operator: WhereOperator::Eq,
            connector: Connector::And,
            mode: MatchMode::Sensitive,
        }
    }

    /// `field <op> value`.
    pub fn op(
        field: impl Into<String>,
        operator: WhereOperator,
        value: impl Into<DbValue>,
    ) -> Self {
        Self {
            operator,
            ..Self::eq(field, value)
        }
    }

    /// Combine this clause with `OR` instead of the default `AND`.
    pub fn or(mut self) -> Self {
        self.connector = Connector::Or;
        self
    }

    /// Compare case-insensitively (string values only).
    pub fn insensitive(mut self) -> Self {
        self.mode = MatchMode::Insensitive;
        self
    }
}

/// Sort direction for `find_many`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Ordering for `find_many` (port of `sortBy`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortBy {
    pub field: String,
    pub direction: SortDirection,
}

/// Relation cardinality for a join (port of `JoinConfig.relation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RelationType {
    OneToOne,
    #[default]
    OneToMany,
    ManyToMany,
}

/// The columns a join matches on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinOn {
    /// Column on the main table.
    pub from: String,
    /// Column on the joined table.
    pub to: String,
}

/// A single join (port of a `JoinConfig` entry).
#[derive(Clone, Debug, PartialEq)]
pub struct JoinEntry {
    pub on: JoinOn,
    pub limit: Option<u64>,
    pub relation: RelationType,
}

/// Relational joins keyed by joined-model name (port of `JoinConfig`).
pub type JoinConfig = BTreeMap<String, JoinEntry>;

// ---------------------------------------------------------------------------
// Operation arguments. Fields are public so callers may use struct literals;
// constructors + setters cover the common, mostly-defaulted cases.
// ---------------------------------------------------------------------------

/// Arguments to [`DatabaseAdapter::create`].
#[derive(Clone, Debug)]
pub struct CreateArgs {
    pub model: String,
    pub data: Row,
    pub select: Option<Vec<String>>,
    /// By default any `id` in `data` is ignored; set to keep it.
    pub force_allow_id: bool,
}

impl CreateArgs {
    pub fn new(model: impl Into<String>, data: Row) -> Self {
        Self {
            model: model.into(),
            data,
            select: None,
            force_allow_id: false,
        }
    }
}

/// Arguments to [`DatabaseAdapter::find_one`].
#[derive(Clone, Debug, Default)]
pub struct FindOneArgs {
    pub model: String,
    pub r#where: Vec<Where>,
    pub select: Option<Vec<String>>,
    pub join: Option<JoinConfig>,
}

impl FindOneArgs {
    pub fn new(model: impl Into<String>, r#where: Vec<Where>) -> Self {
        Self {
            model: model.into(),
            r#where,
            select: None,
            join: None,
        }
    }
}

/// Arguments to [`DatabaseAdapter::find_many`].
#[derive(Clone, Debug, Default)]
pub struct FindManyArgs {
    pub model: String,
    pub r#where: Vec<Where>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub sort_by: Option<SortBy>,
    pub select: Option<Vec<String>>,
    pub join: Option<JoinConfig>,
}

impl FindManyArgs {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }
    pub fn filter(mut self, r#where: Vec<Where>) -> Self {
        self.r#where = r#where;
        self
    }
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
    pub fn sort_by(mut self, sort_by: SortBy) -> Self {
        self.sort_by = Some(sort_by);
        self
    }
}

/// Arguments to [`DatabaseAdapter::count`].
#[derive(Clone, Debug, Default)]
pub struct CountArgs {
    pub model: String,
    pub r#where: Vec<Where>,
}

/// Arguments to [`DatabaseAdapter::update`] / `update_many`.
#[derive(Clone, Debug)]
pub struct UpdateArgs {
    pub model: String,
    pub r#where: Vec<Where>,
    pub update: Row,
}

/// Arguments to [`DatabaseAdapter::delete`] / `delete_many` / `consume_one`.
#[derive(Clone, Debug, Default)]
pub struct DeleteArgs {
    pub model: String,
    pub r#where: Vec<Where>,
}

/// Arguments to [`DatabaseAdapter::increment_one`] — `where` is both selector and guard.
#[derive(Clone, Debug)]
pub struct IncrementArgs {
    pub model: String,
    pub r#where: Vec<Where>,
    /// Per-field signed deltas applied atomically (`field = field + delta`).
    pub increment: BTreeMap<String, f64>,
    /// Absolute assignments applied in the same atomic operation.
    pub set: Option<Row>,
}

/// Errors a [`DatabaseAdapter`] can return.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("model `{0}` is not defined in the schema")]
    UnknownModel(String),
    #[error("adapter `{adapter}` backend error: {message}")]
    Backend { adapter: String, message: String },
    #[error("failed to (de)serialize value: {0}")]
    Serialization(String),
    #[error("{0}")]
    Other(String),
}

/// A model-keyed CRUD contract over portable [`Where`] predicates (port of `DBAdapter`).
///
/// Application code never calls this directly — it goes through the internal domain adapter and
/// the `with_hooks` lifecycle layer (later phases). `transaction` is intentionally omitted here
/// and lands with the SQL adapter; `consume_one`/`increment_one` are race-safe primitives an
/// adapter implements natively (e.g. `DELETE … RETURNING`) or via its own atomicity.
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Stable adapter id (e.g. `"memory"`, `"sqlx-postgres"`).
    fn id(&self) -> &str;

    /// Insert a row and return it (with generated id/defaults applied).
    async fn create(&self, args: CreateArgs) -> Result<Row, AdapterError>;

    /// Find the first row matching `where`, or `None`.
    async fn find_one(&self, args: FindOneArgs) -> Result<Option<Row>, AdapterError>;

    /// Find all rows matching `where` (with optional limit/offset/sort).
    async fn find_many(&self, args: FindManyArgs) -> Result<Vec<Row>, AdapterError>;

    /// Count rows matching `where`.
    async fn count(&self, args: CountArgs) -> Result<u64, AdapterError>;

    /// Update a single matching row; returns the updated row, or `None` if nothing matched.
    async fn update(&self, args: UpdateArgs) -> Result<Option<Row>, AdapterError>;

    /// Update all matching rows; returns the number affected.
    async fn update_many(&self, args: UpdateArgs) -> Result<u64, AdapterError>;

    /// Delete a single matching row.
    async fn delete(&self, args: DeleteArgs) -> Result<(), AdapterError>;

    /// Delete all matching rows; returns the number affected.
    async fn delete_many(&self, args: DeleteArgs) -> Result<u64, AdapterError>;

    /// Atomically delete one matching row and return it (single-use credential consume).
    async fn consume_one(&self, args: DeleteArgs) -> Result<Option<Row>, AdapterError>;

    /// Atomically apply signed deltas to one matching row (guarded counter update).
    async fn increment_one(&self, args: IncrementArgs) -> Result<Option<Row>, AdapterError>;
}

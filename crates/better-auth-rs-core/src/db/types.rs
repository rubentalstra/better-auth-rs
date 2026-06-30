//! Upstream reference: db/type.ts  (renamed `type.ts` → `types.rs`; `type` is a Rust keyword)
//!
//! The dynamic field/value vocabulary the adapter layer is built on. About half of `type.ts` is
//! compile-time TypeScript type inference (`InferDBValueType`, `InferDBField*`,
//! `InferDBFieldsFromOptions/Plugins`) — it has **no runtime analog** and is dropped; the runtime
//! surface ported here is the field-type enum, the value type (`DBPrimitive`), the field-attribute
//! config, the table-schema map, the model names, and the `SecondaryStorage` trait.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use time::OffsetDateTime;

/// The four base model names (`BaseModelNames`). `"rate-limit"` is also a model but not a *base*
/// model (it mirrors upstream's `ModelNames = BaseModelNames | "rate-limit" | custom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseModel {
    /// `"user"`
    User,
    /// `"account"`
    Account,
    /// `"session"`
    Session,
    /// `"verification"`
    Verification,
}

impl BaseModel {
    /// All base models, in declaration order.
    pub const ALL: [BaseModel; 4] = [
        BaseModel::User,
        BaseModel::Account,
        BaseModel::Session,
        BaseModel::Verification,
    ];

    /// The model's string name as used for the DB table key.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            BaseModel::User => "user",
            BaseModel::Account => "account",
            BaseModel::Session => "session",
            BaseModel::Verification => "verification",
        }
    }
}

/// A database field's logical type (`DBFieldType`). The string-literal-array case
/// (`Array<LiteralString>`) — a column constrained to a fixed set of string values — becomes
/// [`DbFieldType::Enum`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbFieldType {
    /// `"string"`
    String,
    /// `"number"`
    Number,
    /// `"boolean"`
    Boolean,
    /// `"date"`
    Date,
    /// `"json"`
    Json,
    /// `"string[]"`
    StringArray,
    /// `"number[]"`
    NumberArray,
    /// `Array<LiteralString>` — an enum-like column constrained to these string values.
    Enum(Vec<String>),
}

/// A database field value (`DBPrimitive`). `null`/`undefined` collapse to [`DbValue::Null`].
#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    /// SQL `NULL` / JS `null`|`undefined`.
    Null,
    /// A string.
    String(String),
    /// A boolean.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A floating-point number.
    Float(f64),
    /// A date/time.
    Date(OffsetDateTime),
    /// A list of strings (`string[]`).
    StringArray(Vec<String>),
    /// A list of numbers (`number[]`).
    NumberArray(Vec<f64>),
    /// Arbitrary JSON (`json`).
    Json(serde_json::Value),
}

impl DbValue {
    /// `true` if this is [`DbValue::Null`].
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, DbValue::Null)
    }
    /// The string value, if this is a [`DbValue::String`].
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DbValue::String(s) => Some(s),
            _ => None,
        }
    }
    /// The integer value, if this is a [`DbValue::Int`].
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            DbValue::Int(n) => Some(*n),
            _ => None,
        }
    }
    /// The float value, if this is a [`DbValue::Float`].
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            DbValue::Float(n) => Some(*n),
            _ => None,
        }
    }
    /// The boolean value, if this is a [`DbValue::Bool`].
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DbValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    /// The date value, if this is a [`DbValue::Date`].
    #[must_use]
    pub fn as_date(&self) -> Option<OffsetDateTime> {
        match self {
            DbValue::Date(d) => Some(*d),
            _ => None,
        }
    }
}

impl From<&str> for DbValue {
    fn from(v: &str) -> Self {
        DbValue::String(v.to_owned())
    }
}
impl From<String> for DbValue {
    fn from(v: String) -> Self {
        DbValue::String(v)
    }
}
impl From<bool> for DbValue {
    fn from(v: bool) -> Self {
        DbValue::Bool(v)
    }
}
impl From<i64> for DbValue {
    fn from(v: i64) -> Self {
        DbValue::Int(v)
    }
}
impl From<f64> for DbValue {
    fn from(v: f64) -> Self {
        DbValue::Float(v)
    }
}
impl From<OffsetDateTime> for DbValue {
    fn from(v: OffsetDateTime) -> Self {
        DbValue::Date(v)
    }
}
impl<T: Into<DbValue>> From<Option<T>> for DbValue {
    fn from(v: Option<T>) -> Self {
        v.map_or(DbValue::Null, Into::into)
    }
}

/// The referential action when a referenced row is deleted (`references.onDelete`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReferentialAction {
    /// `"no action"`
    NoAction,
    /// `"restrict"`
    Restrict,
    /// `"cascade"` — the upstream default.
    #[default]
    Cascade,
    /// `"set null"`
    SetNull,
    /// `"set default"`
    SetDefault,
}

/// A foreign-key reference to another model (`references`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The model being referenced.
    pub model: String,
    /// The field on the referenced model.
    pub field: String,
    /// The action on delete (default [`ReferentialAction::Cascade`]).
    pub on_delete: ReferentialAction,
}

/// A field default (`defaultValue`): a static value or a generator invoked per inserted record.
/// Applied only when creating a record (never as a DB-level default).
#[derive(Clone)]
pub enum DefaultValue {
    /// A fixed value.
    Static(DbValue),
    /// A generator, called to produce the value for each new record (e.g. an id or timestamp).
    Generator(Arc<dyn Fn() -> DbValue + Send + Sync>),
}

impl DefaultValue {
    /// Resolve the default value (clone the static value or invoke the generator).
    #[must_use]
    pub fn resolve(&self) -> DbValue {
        match self {
            DefaultValue::Static(v) => v.clone(),
            DefaultValue::Generator(f) => f(),
        }
    }
}

impl std::fmt::Debug for DefaultValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultValue::Static(v) => f.debug_tuple("Static").field(v).finish(),
            DefaultValue::Generator(_) => f.write_str("Generator(<fn>)"),
        }
    }
}

/// A synchronous value transform (`transform.input` / `transform.output`).
///
/// Upstream allows the transform to be async (`Awaitable`); we use a synchronous transform here —
/// field-level transforms are normalizations (e.g. lowercasing), and async work belongs in hooks,
/// not in a per-field value transform.
pub type TransformFn = Arc<dyn Fn(DbValue) -> DbValue + Send + Sync>;

/// Input/output value transforms for a field (`transform`).
#[derive(Clone, Default)]
pub struct Transform {
    /// Applied before storing.
    pub input: Option<TransformFn>,
    /// Applied after reading.
    pub output: Option<TransformFn>,
}

impl std::fmt::Debug for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transform")
            .field("input", &self.input.as_ref().map(|_| "<fn>"))
            .field("output", &self.output.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

/// A field validator closure (the Rust analog of a `StandardSchemaV1`): returns `Err(message)` on
/// invalid input. We use closures / `garde` rather than the JS "standard schema" interface.
pub type ValidatorFn = Arc<dyn Fn(&DbValue) -> Result<(), String> + Send + Sync>;

/// Input/output validators for a field (`validator`).
#[derive(Clone, Default)]
pub struct Validator {
    /// Validates the value on the way in.
    pub input: Option<ValidatorFn>,
    /// Validates the value on the way out.
    pub output: Option<ValidatorFn>,
}

impl std::fmt::Debug for Validator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Validator")
            .field("input", &self.input.as_ref().map(|_| "<fn>"))
            .field("output", &self.output.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

/// The configuration of a database field (`DBFieldAttributeConfig`) — everything except its `type`.
/// Closure-valued options (`default_value` generator, `on_update`, `transform`, `validator`) make
/// this non-`PartialEq`; it derives `Clone`/`Default` and has a hand-written redacting `Debug`.
#[derive(Clone, Default)]
pub struct DbFieldAttributeConfig {
    /// Required on a new record. `@default true` (i.e. `None` is treated as required).
    pub required: Option<bool>,
    /// Returned in a response body. `@default true`.
    pub returned: Option<bool>,
    /// Accepted as input when creating a record. `@default true`.
    pub input: Option<bool>,
    /// Default value, applied when creating a record (not a DB-level default).
    pub default_value: Option<DefaultValue>,
    /// Generator invoked on update (an `onUpdate` trigger for supported adapters).
    pub on_update: Option<Arc<dyn Fn() -> DbValue + Send + Sync>>,
    /// Input/output value transforms.
    pub transform: Option<Transform>,
    /// Foreign-key reference to another model.
    pub references: Option<Reference>,
    /// Whether the column is unique.
    pub unique: Option<bool>,
    /// Whether the column is a bigint rather than an integer.
    pub bigint: Option<bool>,
    /// Input/output validators.
    pub validator: Option<Validator>,
    /// The column name in the database (defaults to the field key).
    pub field_name: Option<String>,
    /// Whether the column is sortable (varchar vs text). `text`-type fields only.
    pub sortable: Option<bool>,
    /// Whether the column is indexed. `@default false`.
    pub index: Option<bool>,
}

impl std::fmt::Debug for DbFieldAttributeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbFieldAttributeConfig")
            .field("required", &self.required)
            .field("returned", &self.returned)
            .field("input", &self.input)
            .field("default_value", &self.default_value)
            .field("on_update", &self.on_update.as_ref().map(|_| "<fn>"))
            .field("transform", &self.transform)
            .field("references", &self.references)
            .field("unique", &self.unique)
            .field("bigint", &self.bigint)
            .field("validator", &self.validator)
            .field("field_name", &self.field_name)
            .field("sortable", &self.sortable)
            .field("index", &self.index)
            .finish()
    }
}

/// A database field: its [`DbFieldType`] plus its [`DbFieldAttributeConfig`]
/// (`DBFieldAttribute = { type } & DBFieldAttributeConfig`).
#[derive(Debug, Clone)]
pub struct DbFieldAttribute {
    /// The field's logical type.
    pub field_type: DbFieldType,
    /// The field's configuration.
    pub config: DbFieldAttributeConfig,
}

impl DbFieldAttribute {
    /// A field of the given type with default configuration.
    #[must_use]
    pub fn new(field_type: DbFieldType) -> Self {
        Self {
            field_type,
            config: DbFieldAttributeConfig::default(),
        }
    }
}

/// One table in a [`BetterAuthDbSchema`] (the value of a `BetterAuthDBSchema` entry).
#[derive(Debug, Clone)]
pub struct DbTableSchema {
    /// The table name in the database.
    pub model_name: String,
    /// The table's fields, keyed by field name.
    pub fields: BTreeMap<String, DbFieldAttribute>,
    /// Whether to skip migrations for this table. `@default false`.
    pub disable_migrations: Option<bool>,
    /// Explicit ordering of the table relative to others.
    pub order: Option<i64>,
}

/// The full better-auth database schema (`BetterAuthDBSchema`): model key → [`DbTableSchema`].
pub type BetterAuthDbSchema = BTreeMap<String, DbTableSchema>;

/// Error from a [`SecondaryStorage`] backend.
#[derive(Debug, thiserror::Error)]
pub enum SecondaryStorageError {
    /// The backend reported an error.
    #[error("secondary storage backend error: {0}")]
    Backend(String),
}

/// A key/value store used for sessions, rate-limit counters, and similar ephemeral data
/// (`SecondaryStorage`). TTLs are in **seconds**.
#[async_trait]
pub trait SecondaryStorage: Send + Sync {
    /// Get the value stored at `key`, or `None` if absent.
    async fn get(&self, key: &str) -> Result<Option<String>, SecondaryStorageError>;

    /// Store `value` at `key` with an optional TTL (seconds).
    async fn set(
        &self,
        key: &str,
        value: &str,
        ttl: Option<i64>,
    ) -> Result<(), SecondaryStorageError>;

    /// Delete `key`.
    async fn delete(&self, key: &str) -> Result<(), SecondaryStorageError>;

    /// Atomically get a value and delete it. The default is a non-atomic read-then-delete;
    /// backends that can do this atomically should override it to avoid the read/delete race
    /// (single-use credential consumers rely on the atomic form when available).
    async fn get_and_delete(&self, key: &str) -> Result<Option<String>, SecondaryStorageError> {
        let value = self.get(key).await?;
        if value.is_some() {
            self.delete(key).await?;
        }
        Ok(value)
    }

    /// Atomically increment the counter at `key`, returning the post-increment value. On first
    /// creation the key is set to `1` with the given `ttl` (seconds), applied only on creation.
    ///
    /// Returns `Ok(None)` by default, signalling the backend has no atomic increment — callers fall
    /// back to a get/set counter. Backends that support it should override and return `Ok(Some(n))`.
    async fn increment(&self, key: &str, ttl: i64) -> Result<Option<i64>, SecondaryStorageError> {
        let _ = (key, ttl);
        Ok(None)
    }
}

#[cfg(test)]
#[path = "types.test.rs"]
mod types_tests;

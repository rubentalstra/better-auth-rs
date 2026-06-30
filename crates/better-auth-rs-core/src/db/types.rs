//! Upstream source: `db/type.ts` (co-located in this directory).
//!
//! This file is named `types.rs`, NOT `type.rs`, because `type` is a reserved keyword in Rust
//! (`mod type;` is illegal and `r#type` is avoided). The convention is to pluralize a
//! reserved-keyword stem (`type` → `types`); the manifest records the `db/type.ts → db/types.rs`
//! mapping.
//!
//! The field/value type system (port of `@better-auth/core` `db/type.ts`).
//!
//! better-auth describes each model as a record of `DBFieldAttribute`s and carries row data
//! as dynamic `Record<string, any>`. In Rust the dynamic row is a [`Row`] of named [`DbValue`]s,
//! and a table is described by an ordered set of [`FieldAttribute`]s ([`TableSchema`]).

use std::collections::BTreeMap;
use std::fmt::Debug;

use async_trait::async_trait;
use serde_json::Value as Json;
use time::OffsetDateTime;

/// A dynamic database row: field name → value. Ordered for deterministic output.
pub type Row = BTreeMap<String, DbValue>;

/// A runtime database value. Mirrors better-auth's `DBPrimitive` set, but typed.
#[derive(Clone, Debug, PartialEq)]
pub enum DbValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    DateTime(OffsetDateTime),
    Json(Json),
    StringArray(Vec<String>),
    IntArray(Vec<i64>),
}

impl DbValue {
    /// Whether this value is SQL/JSON null.
    pub fn is_null(&self) -> bool {
        matches!(self, DbValue::Null)
    }

    /// Borrow the value as a string slice, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DbValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Convert to a `serde_json::Value` for transport (e.g. HTTP responses).
    /// Dates are rendered as RFC 3339 strings to match better-auth's JSON shape.
    pub fn to_json(&self) -> Json {
        match self {
            DbValue::Null => Json::Null,
            DbValue::Bool(b) => Json::Bool(*b),
            DbValue::Int(i) => Json::from(*i),
            DbValue::Float(f) => serde_json::Number::from_f64(*f).map_or(Json::Null, Json::Number),
            DbValue::String(s) => Json::String(s.clone()),
            DbValue::DateTime(dt) => Json::String(
                dt.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            ),
            DbValue::Json(v) => v.clone(),
            DbValue::StringArray(a) => Json::Array(a.iter().cloned().map(Json::String).collect()),
            DbValue::IntArray(a) => Json::Array(a.iter().copied().map(Json::from).collect()),
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
        DbValue::DateTime(v)
    }
}
impl From<Json> for DbValue {
    fn from(v: Json) -> Self {
        DbValue::Json(v)
    }
}
impl<T: Into<DbValue>> From<Option<T>> for DbValue {
    fn from(v: Option<T>) -> Self {
        v.map_or(DbValue::Null, Into::into)
    }
}

/// The declared column type of a field (port of `DBFieldType`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DbFieldType {
    String,
    Number,
    Boolean,
    Date,
    Json,
    StringArray,
    NumberArray,
    /// `Array<LiteralString>` upstream — a fixed set of allowed string values (enum column).
    Enum(Vec<String>),
}

/// Referential action when a referenced row is deleted (port of `references.onDelete`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OnDelete {
    NoAction,
    Restrict,
    #[default]
    Cascade,
    SetNull,
    SetDefault,
}

/// A foreign-key reference to another model's field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldReference {
    pub model: String,
    pub field: String,
    pub on_delete: OnDelete,
}

/// A field's default applied when creating a record (port of `defaultValue`).
///
/// Static values are carried directly; the common dynamic defaults are represented as variants
/// (better-auth uses `() => new Date()` and id generators). Other function defaults are applied
/// in code at the point of creation rather than described as data.
#[derive(Clone, Debug, PartialEq)]
pub enum DefaultValue {
    Value(DbValue),
    /// `() => new Date()` — current timestamp (e.g. `createdAt`/`updatedAt`).
    Now,
    /// The instance's id generator.
    GenerateId,
}

/// Attributes describing a single field/column (port of `DBFieldAttribute`).
///
/// The data-shaped attributes are modeled here (used for schema/migration generation and
/// input/output projection). Function-valued attributes (`transform`, `onUpdate`, `validator`)
/// are handled in the adapter factory port rather than as plain data.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldAttribute {
    pub r#type: DbFieldType,
    /// Required on a new record. Default `true`.
    pub required: bool,
    /// Returned in responses. Default `true`.
    pub returned: bool,
    /// Accepted as input on create. Default `true`.
    pub input: bool,
    pub unique: bool,
    /// Store as bigint rather than integer.
    pub bigint: bool,
    pub sortable: bool,
    pub index: bool,
    /// Override the column name in the database.
    pub field_name: Option<String>,
    pub references: Option<FieldReference>,
    pub default_value: Option<DefaultValue>,
}

impl FieldAttribute {
    /// A required, returned, input field of the given type with default flags.
    pub fn new(r#type: DbFieldType) -> Self {
        Self {
            r#type,
            required: true,
            returned: true,
            input: true,
            unique: false,
            bigint: false,
            sortable: false,
            index: false,
            field_name: None,
            references: None,
            default_value: None,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }
    pub fn not_returned(mut self) -> Self {
        self.returned = false;
        self
    }
    pub fn not_input(mut self) -> Self {
        self.input = false;
        self
    }
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
    pub fn indexed(mut self) -> Self {
        self.index = true;
        self
    }
    pub fn bigint(mut self) -> Self {
        self.bigint = true;
        self
    }
    pub fn default(mut self, value: DefaultValue) -> Self {
        self.default_value = Some(value);
        self
    }
    pub fn references(mut self, model: impl Into<String>, field: impl Into<String>) -> Self {
        self.references = Some(FieldReference {
            model: model.into(),
            field: field.into(),
            on_delete: OnDelete::Cascade,
        });
        self
    }
}

/// A table's schema (port of `BetterAuthDBSchema` entries): its DB model name plus an ordered
/// list of `(field_key, attributes)`. Field order is preserved (better-auth relies on it).
#[derive(Clone, Debug)]
pub struct TableSchema {
    pub model_name: String,
    pub fields: Vec<(String, FieldAttribute)>,
    pub disable_migrations: bool,
    pub order: Option<u32>,
}

impl TableSchema {
    pub fn new(model_name: impl Into<String>, fields: Vec<(String, FieldAttribute)>) -> Self {
        Self {
            model_name: model_name.into(),
            fields,
            disable_migrations: false,
            order: None,
        }
    }

    /// Look up a field's attributes by its better-auth field key.
    pub fn field(&self, key: &str) -> Option<&FieldAttribute> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, a)| a)
    }
}

/// The full better-auth schema: model key → table (port of `BetterAuthDBSchema`).
pub type BetterAuthDbSchema = BTreeMap<String, TableSchema>;

/// A pluggable key/value store for sessions, verifications, rate-limit counters, etc. (port of the
/// `SecondaryStorage` interface). Implemented by the `redis-storage` crate; `None` in
/// [`AuthContext`](crate) means database-only.
///
/// `get_and_delete` and `increment` are optional upstream (`getAndDelete?`/`increment?`); a backend
/// advertises support via the `supports_*` flags, and callers fall back to read-then-delete /
/// database counters when unsupported. Stored values are always strings (better-auth JSON-encodes).
#[async_trait]
pub trait SecondaryStorage: Send + Sync + Debug {
    /// Get the value stored at `key`, or `None` if absent.
    async fn get(&self, key: &str) -> Option<String>;

    /// Store `value` at `key`, optionally expiring after `ttl` seconds.
    async fn set(&self, key: &str, value: &str, ttl: Option<u64>);

    /// Delete `key`.
    async fn delete(&self, key: &str);

    /// Whether this backend implements the atomic [`get_and_delete`](Self::get_and_delete).
    fn supports_get_and_delete(&self) -> bool {
        false
    }

    /// Atomically get the value at `key` and delete it (single-use consume). Returns `None` if the
    /// key was absent. Only meaningful when [`supports_get_and_delete`](Self::supports_get_and_delete)
    /// is `true`; the default is a no-op so callers fall back to read-then-delete.
    async fn get_and_delete(&self, _key: &str) -> Option<String> {
        None
    }

    /// Whether this backend implements the atomic [`increment`](Self::increment).
    fn supports_increment(&self) -> bool {
        false
    }

    /// Atomically increment the counter at `key`, returning the post-increment value. On creation
    /// the key is set to `1` with `ttl` seconds (TTL applied only on creation). Returns `None` when
    /// unsupported.
    async fn increment(&self, _key: &str, _ttl: u64) -> Option<i64> {
        None
    }
}

//! Interim aggregator for `db/`. The full port of `db/index.ts` — which also re-exports
//! `getAuthTables` (`get-tables.ts`) — lands when that module is built. For now this wires the
//! entity schemas, the field/value type vocabulary, and the plugin schema.

pub mod plugin;
pub mod schema;
pub mod types;

pub use plugin::{BetterAuthPluginDbSchema, PluginTableSchema};
pub use schema::{Account, CoreFields, RateLimit, Session, User, Verification};
pub use types::{
    BaseModel, BetterAuthDbSchema, DbFieldAttribute, DbFieldAttributeConfig, DbFieldType,
    DbTableSchema, DbValue, DefaultValue, Reference, ReferentialAction, SecondaryStorage,
    SecondaryStorageError, Transform, TransformFn, Validator, ValidatorFn,
};

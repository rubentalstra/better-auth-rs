//! Database layer: the field/value type system, the model-keyed adapter interface, and the core
//! data model (port of `@better-auth/core` `db/`).

pub mod adapter;
pub mod schema;
pub mod types;

pub use adapter::{
    AdapterError, Connector, CountArgs, CreateArgs, DatabaseAdapter, DeleteArgs, FindManyArgs,
    FindOneArgs, IncrementArgs, JoinConfig, JoinEntry, JoinOn, MatchMode, RelationType, SortBy,
    SortDirection, UpdateArgs, Where, WhereOperator,
};
pub use schema::{Account, RateLimit, Session, User, Verification, core_tables};
pub use types::{
    BetterAuthDbSchema, DbFieldType, DbValue, DefaultValue, FieldAttribute, FieldReference,
    OnDelete, Row, SecondaryStorage, TableSchema,
};

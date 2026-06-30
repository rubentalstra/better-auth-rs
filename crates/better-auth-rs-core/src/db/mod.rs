//! Interim aggregator for `db/`. The full port of `db/index.ts` — which also re-exports the
//! field/value types from `type.ts`, the plugin schema (`plugin.ts`), and `getAuthTables`
//! (`get-tables.ts`) — lands as those modules are built. For now this wires the entity schemas.

pub mod schema;

pub use schema::{Account, CoreFields, RateLimit, Session, User, Verification};

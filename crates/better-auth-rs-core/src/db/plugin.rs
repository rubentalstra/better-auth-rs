//! Upstream reference: db/plugin.ts
//!
//! A plugin's contribution to the database schema (`BetterAuthPluginDBSchema`): table name →
//! `{ fields, disableMigration?, modelName? }`. Note the singular `disableMigration` here, distinct
//! from the core schema's plural `disableMigrations`.

use std::collections::BTreeMap;

use super::types::DbFieldAttribute;

/// One table contributed by a plugin.
#[derive(Debug, Clone)]
pub struct PluginTableSchema {
    /// The table's fields, keyed by field name.
    pub fields: BTreeMap<String, DbFieldAttribute>,
    /// Whether to skip migrations for this table.
    pub disable_migration: Option<bool>,
    /// The table name in the database (defaults to the schema key).
    pub model_name: Option<String>,
}

/// A plugin's database schema (`BetterAuthPluginDBSchema`): table name → [`PluginTableSchema`].
pub type BetterAuthPluginDbSchema = BTreeMap<String, PluginTableSchema>;

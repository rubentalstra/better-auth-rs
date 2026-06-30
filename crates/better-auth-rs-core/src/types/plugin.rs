//! Upstream reference: types/plugin.ts
//!
//! `BetterAuthPlugin` — the server-plugin contract. Implemented per plugin and registered via
//! `options.plugins`; held by [`AuthContext`](crate::types::context::AuthContext) and
//! [`BetterAuthOptions`](crate::types::init_options::BetterAuthOptions) as `Arc<dyn BetterAuthPlugin>`.
//!
//! Every capability is a trait method with a no-op default, so a plugin implements only what it
//! needs. The trait is object-safe (all methods take `&self`, return owned/`'static` types, no
//! generics) and `#[async_trait]` (for the async `init`).
//!
//! ## Dropped / deferred from the TS (documented, no method)
//!
//! - `$Infer` — TypeScript type-only inference; no runtime analog. Dropped.
//! - `migrations: Record<string, Migration>` — Kysely migrations; deferred to the ORM adapter
//!   crates (Diesel/SeaORM/SQLx), which own migration running. Schema-based migration is expressed
//!   via [`schema`](BetterAuthPlugin::schema).
//! - `adapter: { [k]: (...args) => any }` — the untyped per-plugin DB-op override map. Deferred: a
//!   typed plugin-adapter trait will model it when a concrete plugin needs one (an untyped
//!   `Fn(Vec<Value>) -> Value` bag is neither safe nor idiomatic).
//! - `on_request` / `on_response` — the outer-pipeline HTTP hooks. Deferred to the api/integration
//!   batch, where the request/response representation is settled (they are invoked there, not in
//!   core; see the plan). Added to this trait then.
//! - `options: Record<string, any>` — a plugin's own config lives in its concrete struct's fields;
//!   there is no runtime need for an untyped bag on the trait.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::api::{AuthMiddleware, Endpoint, HookEndpointContext, Middleware};
use crate::db::BetterAuthPluginDbSchema;
use crate::error::RawError;
use crate::types::context::AuthContext;
use crate::types::init_options::{BetterAuthOptions, BetterAuthRateLimitRule};

/// A conditional hook entry (`{ matcher, handler }`).
///
/// The `matcher` is a sync predicate over the (read-only) hook context deciding whether `handler`
/// runs for a given request.
pub struct HookEntry {
    /// Decides whether the handler runs for this request.
    pub matcher: Arc<dyn Fn(&HookEndpointContext) -> bool + Send + Sync>,
    /// The middleware to run when the matcher returns `true`.
    pub handler: AuthMiddleware,
}

impl core::fmt::Debug for HookEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HookEntry")
            .field("matcher", &"<fn>")
            .field("handler", &"<fn>")
            .finish()
    }
}

/// Context overrides a plugin's [`init`](BetterAuthPlugin::init) may contribute.
///
/// The TS `DeepPartial<Omit<AuthContext, "options">>` is modeled as an explicit set of the
/// resolved context fields plugins realistically extend at init (e.g. SSO/organization adding
/// trusted origins/providers). The set grows as concrete plugins land; `None` leaves a field
/// unchanged.
#[derive(Debug, Clone, Default)]
pub struct AuthContextPatch {
    /// Append/replace trusted origins.
    pub trusted_origins: Option<Vec<String>>,
    /// Append/replace trusted providers for account linking.
    pub trusted_providers: Option<Vec<String>>,
}

/// What [`init`](BetterAuthPlugin::init) returns (`{ context?, options? } | void`).
#[derive(Debug, Clone, Default)]
pub struct InitResult {
    /// Context overrides this plugin contributes (`None` = no change).
    pub context: Option<AuthContextPatch>,
    /// Options overrides this plugin contributes (`None` = no change; a partial `BetterAuthOptions`
    /// whose unset fields are left as-is by the init layer's merge).
    pub options: Option<BetterAuthOptions>,
}

/// A plugin's per-path rate-limit rule (`rateLimit[]`): a [`BetterAuthRateLimitRule`] plus a path
/// matcher.
pub struct PluginRateLimitRule {
    /// The window/max for matching paths.
    pub rule: BetterAuthRateLimitRule,
    /// Decides which paths the rule applies to.
    pub path_matcher: Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl core::fmt::Debug for PluginRateLimitRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PluginRateLimitRule")
            .field("rule", &self.rule)
            .field("path_matcher", &"<fn>")
            .finish()
    }
}

/// A server-side plugin (`BetterAuthPlugin`). Usable as `Arc<dyn BetterAuthPlugin>`.
#[async_trait]
pub trait BetterAuthPlugin: Send + Sync {
    /// The plugin's stable identifier (`id`). The registry key — the one required method.
    fn id(&self) -> &str;

    /// The plugin's version (`version`), if any.
    fn version(&self) -> Option<&str> {
        None
    }

    /// Initialize the plugin against the built context (`init`), optionally contributing context /
    /// options overrides. Default: no overrides.
    async fn init(&self, _ctx: &AuthContext) -> InitResult {
        InitResult::default()
    }

    /// Endpoints this plugin registers (`endpoints`), keyed by endpoint name. Default: none.
    fn endpoints(&self) -> BTreeMap<String, Endpoint> {
        BTreeMap::new()
    }

    /// Path-scoped middlewares this plugin registers (`middlewares`). Default: none.
    fn middlewares(&self) -> Vec<Middleware> {
        Vec::new()
    }

    /// Hooks run before matching endpoints (`hooks.before`). Default: none.
    fn hooks_before(&self) -> Vec<HookEntry> {
        Vec::new()
    }

    /// Hooks run after matching endpoints (`hooks.after`). Default: none.
    fn hooks_after(&self) -> Vec<HookEntry> {
        Vec::new()
    }

    /// The database schema this plugin needs (`schema`), used for migrations. Default: none.
    fn schema(&self) -> Option<BetterAuthPluginDbSchema> {
        None
    }

    /// Per-path rate-limit rules (`rateLimit`). Default: none.
    fn rate_limit(&self) -> Vec<PluginRateLimitRule> {
        Vec::new()
    }

    /// The plugin's error-code set (`$ERROR_CODES`), keyed by error name. Default: none.
    fn error_codes(&self) -> BTreeMap<&'static str, RawError> {
        BTreeMap::new()
    }
}

#[cfg(test)]
#[path = "plugin.test.rs"]
mod plugin_tests;

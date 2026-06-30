//! Upstream reference: types/init-options.ts  (in progress — sub-types ported; see note)
//!
//! The option vocabulary for configuring an instance. This file ports the **option sub-types**
//! (id generation, base-URL, rate-limit, per-model db options, advanced options). The top-level
//! `BetterAuthOptions` aggregate is part of the mutually-recursive type hub (it references
//! `BetterAuthPlugin` and `AuthContext`) and lands with that batch, so `init-options.ts` stays
//! `building`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::db::schema::RateLimit;
use crate::db::types::DbFieldAttribute;
use crate::types::cookie::CookieOptions;

/// A boxed, `Send` future (used for async config closures).
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Input to a [`GenerateIdFn`].
#[derive(Debug, Clone)]
pub struct GenerateIdInput {
    /// The model the id is for.
    pub model: String,
    /// An optional requested size.
    pub size: Option<usize>,
}

/// A custom id generator (`GenerateIdFn`). Returns `None` for the upstream `false` (let the database
/// generate the id).
pub type GenerateIdFn = Arc<dyn Fn(GenerateIdInput) -> Option<String> + Send + Sync>;

/// How ids are generated for new records (`advanced.database.generateId`).
#[derive(Clone, Default)]
pub enum GenerateIdConfig {
    /// Generate random ids (the default).
    #[default]
    Default,
    /// Let the database auto-generate ids (`false`).
    Database,
    /// Use a database serial/auto-increment id (`"serial"`).
    Serial,
    /// Generate a UUID (`"uuid"`).
    Uuid,
    /// A user-provided generator.
    Custom(GenerateIdFn),
}

impl core::fmt::Debug for GenerateIdConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => f.write_str("Default"),
            Self::Database => f.write_str("Database"),
            Self::Serial => f.write_str("Serial"),
            Self::Uuid => f.write_str("Uuid"),
            Self::Custom(_) => f.write_str("Custom(<fn>)"),
        }
    }
}

/// How a single-use identifier is stored (`StoreIdentifierOption`).
#[derive(Clone, Default)]
pub enum StoreIdentifierOption {
    /// Store the identifier as-is (the default).
    #[default]
    Plain,
    /// Store a hash of the identifier.
    Hashed,
    /// Store a custom async hash of the identifier.
    Custom(Arc<dyn Fn(String) -> BoxFuture<String> + Send + Sync>),
}

impl core::fmt::Debug for StoreIdentifierOption {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Plain => f.write_str("Plain"),
            Self::Hashed => f.write_str("Hashed"),
            Self::Custom(_) => f.write_str("Custom(<fn>)"),
        }
    }
}

/// Protocol selection for [`DynamicBaseUrlConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaseUrlProtocol {
    /// Always HTTP.
    Http,
    /// Always HTTPS.
    Https,
    /// Derive from `x-forwarded-proto`, defaulting to HTTPS (the default).
    #[default]
    Auto,
}

/// Dynamic base-URL configuration for multi-domain deployments (`DynamicBaseURLConfig`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicBaseUrlConfig {
    /// Allowed hostnames (supports wildcard patterns, like `trustedOrigins`).
    pub allowed_hosts: Vec<String>,
    /// Fallback URL when the derived host matches none of `allowed_hosts`.
    pub fallback: Option<String>,
    /// Protocol to use when constructing the URL.
    pub protocol: Option<BaseUrlProtocol>,
}

/// Base-URL configuration (`BaseURLConfig`): a static URL or a dynamic config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseUrlConfig {
    /// A fixed base URL.
    Static(String),
    /// A dynamic, multi-domain config.
    Dynamic(DynamicBaseUrlConfig),
}

/// A rate-limit rule (`BetterAuthRateLimitRule`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BetterAuthRateLimitRule {
    /// Window in seconds (default 10).
    pub window: u64,
    /// Max requests per window (default 100).
    pub max: u64,
}

impl Default for BetterAuthRateLimitRule {
    fn default() -> Self {
        Self {
            window: 10,
            max: 100,
        }
    }
}

/// Per-model database options (`BetterAuthDBOptions`): rename the table, map fields to columns, and
/// add extra fields.
#[derive(Debug, Clone, Default)]
pub struct BetterAuthDbOptions {
    /// Override the table name.
    pub model_name: Option<String>,
    /// Map field keys to database column names.
    pub fields: Option<std::collections::BTreeMap<String, String>>,
    /// Additional fields for the model.
    pub additional_fields: Option<std::collections::BTreeMap<String, DbFieldAttribute>>,
}

/// Error from a [`BetterAuthRateLimitStorage`] backend.
pub type RateLimitStorageError = Box<dyn std::error::Error + Send + Sync>;

/// Pluggable storage for rate-limit counters (`BetterAuthRateLimitStorage`).
#[async_trait::async_trait]
pub trait BetterAuthRateLimitStorage: Send + Sync {
    /// Get the counter for `key`.
    async fn get(&self, key: &str) -> Result<Option<RateLimit>, RateLimitStorageError>;
    /// Set the counter for `key`.
    async fn set(
        &self,
        key: &str,
        value: RateLimit,
        update: bool,
    ) -> Result<(), RateLimitStorageError>;
    /// Atomically record one request against `key` within `rule.window` (seconds) and report
    /// whether it is allowed. The default falls back to the non-atomic get/set path
    /// (`Ok(None)` signalling "not natively supported").
    async fn consume(
        &self,
        _key: &str,
        _rule: BetterAuthRateLimitRule,
    ) -> Result<Option<RateLimitConsumeResult>, RateLimitStorageError> {
        Ok(None)
    }
}

/// The result of an atomic [`BetterAuthRateLimitStorage::consume`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConsumeResult {
    /// Whether the request was allowed within the window.
    pub allowed: bool,
    /// Seconds until the window frees up (when not allowed).
    pub retry_after: Option<u64>,
}

/// Where rate-limit state lives (`storage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateLimitStorageKind {
    /// In-memory (the default).
    #[default]
    Memory,
    /// In the primary database.
    Database,
    /// In the configured secondary storage.
    SecondaryStorage,
}

/// Rate-limiting options (`BetterAuthRateLimitOptions`).
///
/// The `customRules` map's function-rule form (a closure deciding the rule per request) is deferred
/// with the request/api layer; the static rule form is modeled here.
#[derive(Clone, Default)]
pub struct BetterAuthRateLimitOptions {
    /// Default window (seconds).
    pub window: Option<u64>,
    /// Default max requests.
    pub max: Option<u64>,
    /// Override the rate-limit table name.
    pub model_name: Option<String>,
    /// Map rate-limit fields to columns.
    pub fields: Option<std::collections::BTreeMap<String, String>>,
    /// Whether rate limiting is enabled (default: production only).
    pub enabled: Option<bool>,
    /// Static custom rules per path (the function-rule form is deferred).
    pub custom_rules: Option<std::collections::BTreeMap<String, BetterAuthRateLimitRule>>,
    /// Where state is stored.
    pub storage: Option<RateLimitStorageKind>,
    /// Custom storage backend (overrides `storage`).
    pub custom_storage: Option<Arc<dyn BetterAuthRateLimitStorage>>,
}

impl core::fmt::Debug for BetterAuthRateLimitOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BetterAuthRateLimitOptions")
            .field("window", &self.window)
            .field("max", &self.max)
            .field("model_name", &self.model_name)
            .field("fields", &self.fields)
            .field("enabled", &self.enabled)
            .field("custom_rules", &self.custom_rules)
            .field("storage", &self.storage)
            .field(
                "custom_storage",
                &self.custom_storage.as_ref().map(|_| "<storage>"),
            )
            .finish()
    }
}

/// IP-address handling (`advanced.ipAddress`).
#[derive(Debug, Clone, Default)]
pub struct IpAddressOptions {
    /// Headers to read the client IP from, in order.
    pub ip_address_headers: Option<Vec<String>>,
    /// Disable IP tracking entirely (a security risk).
    pub disable_ip_tracking: Option<bool>,
    /// IPv6 prefix length used to collapse addresses before rate-limit keying (default 64).
    pub ipv6_subnet: Option<u8>,
    /// Trusted reverse-proxy IPs / CIDR ranges for forwarded-chain walking.
    pub trusted_proxies: Option<Vec<String>>,
}

/// Cross-subdomain cookie configuration (`advanced.crossSubDomainCookies`).
#[derive(Debug, Clone, Default)]
pub struct CrossSubDomainCookies {
    /// Enable cross-subdomain cookies.
    pub enabled: bool,
    /// Additional cookies to share across subdomains.
    pub additional_cookies: Option<Vec<String>>,
    /// The cookie domain (defaults to the root domain of the base URL).
    pub domain: Option<String>,
}

/// A single cookie override (`advanced.cookies[name]`).
#[derive(Debug, Clone, Default)]
pub struct CookieOverride {
    /// Override the cookie name.
    pub name: Option<String>,
    /// Override the cookie attributes.
    pub attributes: Option<CookieOptions>,
}

/// Database tuning (`advanced.database`).
#[derive(Debug, Clone, Default)]
pub struct AdvancedDatabaseOptions {
    /// Default `findMany` limit (default 100).
    pub default_find_many_limit: Option<u64>,
    /// How ids are generated.
    pub generate_id: Option<GenerateIdConfig>,
}

/// A background-task handler (`advanced.backgroundTasks.handler`) — runs a deferred future.
pub type BackgroundTaskHandler = Arc<dyn Fn(BoxFuture<()>) + Send + Sync>;

/// Advanced options (`BetterAuthAdvancedOptions`).
#[derive(Clone, Default)]
pub struct BetterAuthAdvancedOptions {
    /// IP-address handling.
    pub ip_address: Option<IpAddressOptions>,
    /// Always set the `Secure` cookie attribute.
    pub use_secure_cookies: Option<bool>,
    /// Disable all CSRF protection (dangerous).
    pub disable_csrf_check: Option<bool>,
    /// Disable URL validation against `trustedOrigins` (dangerous).
    pub disable_origin_check: Option<bool>,
    /// Cross-subdomain cookie configuration.
    pub cross_sub_domain_cookies: Option<CrossSubDomainCookies>,
    /// Per-cookie name/attribute overrides.
    pub cookies: Option<std::collections::BTreeMap<String, CookieOverride>>,
    /// Default attributes applied to all cookies.
    pub default_cookie_attributes: Option<CookieOptions>,
    /// Cookie name prefix (defaults to the app name).
    pub cookie_prefix: Option<String>,
    /// Database tuning.
    pub database: Option<AdvancedDatabaseOptions>,
    /// Infer the base URL from `x-forwarded-*` proxy headers.
    pub trusted_proxy_headers: Option<bool>,
    /// Handler for deferred background tasks.
    pub background_tasks: Option<BackgroundTaskHandler>,
    /// Treat trailing-slash routes the same as non-trailing.
    pub skip_trailing_slashes: Option<bool>,
}

impl core::fmt::Debug for BetterAuthAdvancedOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BetterAuthAdvancedOptions")
            .field("ip_address", &self.ip_address)
            .field("use_secure_cookies", &self.use_secure_cookies)
            .field("disable_csrf_check", &self.disable_csrf_check)
            .field("disable_origin_check", &self.disable_origin_check)
            .field("cross_sub_domain_cookies", &self.cross_sub_domain_cookies)
            .field("cookies", &self.cookies)
            .field("default_cookie_attributes", &self.default_cookie_attributes)
            .field("cookie_prefix", &self.cookie_prefix)
            .field("database", &self.database)
            .field("trusted_proxy_headers", &self.trusted_proxy_headers)
            .field(
                "background_tasks",
                &self.background_tasks.as_ref().map(|_| "<fn>"),
            )
            .field("skip_trailing_slashes", &self.skip_trailing_slashes)
            .finish()
    }
}

#[cfg(test)]
#[path = "init_options.test.rs"]
mod init_options_tests;

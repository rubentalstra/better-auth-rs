//! Upstream reference: types/init-options.ts  (in progress — sub-types ported; see note)
//!
//! The option vocabulary for configuring an instance. This file ports the **option sub-types**
//! (id generation, base-URL, rate-limit, per-model db options, advanced options). The top-level
//! `BetterAuthOptions` aggregate is part of the mutually-recursive type hub (it references
//! `BetterAuthPlugin` and `AuthContext`) and lands with that batch, so `init-options.ts` stays
//! `building`.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::{Map, Value};
use time::OffsetDateTime;

use crate::api::AuthMiddleware;
use crate::db::SecondaryStorage;
use crate::db::adapter::DatabaseAdapter;
use crate::db::schema::{Account, RateLimit, Session, User, Verification};
use crate::db::types::DbFieldAttribute;
use crate::env::logger::Logger;
use crate::oauth2::DynOAuthProvider;
use crate::types::context::{
    AuthContext, CallbackError, GenericEndpointContext, PasswordHashFn, PasswordVerifyFn,
    StoreStateStrategy,
};
use crate::types::cookie::CookieOptions;
use crate::types::plugin::BetterAuthPlugin;

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

/// A per-path custom rate-limit entry (`customRules[path]`).
///
/// Upstream's value is `BetterAuthRateLimitRule | false | ((request, rule) => Awaitable<rule | false>)`:
/// a rule applies a custom window/max, while `false` **exempts** the path from rate limiting (which
/// is distinct from omitting the entry — that falls back to the default rule). The per-request
/// function form is deferred with the request/api layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomRateLimitRule {
    /// Apply this window/max to the path.
    Rule(BetterAuthRateLimitRule),
    /// Exempt this path from rate limiting entirely (the upstream `false`).
    Disabled,
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
/// The `customRules` map's value is a [`CustomRateLimitRule`] (a per-path rule or an explicit
/// exemption); the per-request function-rule form (a closure deciding the rule per request) is
/// deferred with the request/api layer.
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
    /// Custom rules per path: apply a rule or exempt the path (`customRules`). The per-request
    /// function-rule form is deferred.
    pub custom_rules: Option<BTreeMap<String, CustomRateLimitRule>>,
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

// ─────────────────────────────────────────────────────────────────────────────
// BetterAuthOptions — the full configuration aggregate.
//
// Deviations from the TS (idiomatic + secure Rust):
// - The `database` Kysely/pool/dialect union collapses to "provide a `DatabaseAdapter`": the storage
//   is chosen by depending on an adapter crate (memory/sqlx/seaorm/…), so options just hold the
//   adapter instance.
// - `socialProviders` (a name→factory map upstream) holds the instantiated providers directly; the
//   social-provider registry that builds them is a later batch.
// - Lifecycle callbacks that upstream hands a raw Fetch `Request` (e.g. `sendResetPassword`) and the
//   database hooks (handed a `GenericEndpointContext`) are unified to take
//   `Option<Arc<GenericEndpointContext>>` — the per-request context carries both the auth context
//   and (via the api layer) the request metadata, and async Rust closures take owned/`Arc` values.
// - The top-level `hooks` (before/after `AuthMiddleware`) is added with the api anchors.
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a user-provided lifecycle callback: success, or any error (which aborts the operation).
pub type CallbackResult = Result<(), CallbackError>;

/// A per-request resolver returning a list of strings (trusted origins / providers). Must tolerate a
/// `None` context (called during init before any request).
pub type RequestStringListFn =
    Arc<dyn Fn(Option<Arc<GenericEndpointContext>>) -> BoxFuture<Vec<String>> + Send + Sync>;

/// A versioned secret entry (`secrets[]`). Secret material is never logged.
#[derive(Clone)]
pub struct SecretEntry {
    /// The key version (the highest is the current/encryption key).
    pub version: u32,
    /// The secret value.
    pub value: String,
}

impl core::fmt::Debug for SecretEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecretEntry")
            .field("version", &self.version)
            .field("value", &"<redacted>")
            .finish()
    }
}

/// `{ user, url, token }` passed to email-sending callbacks (verification / reset / delete).
#[derive(Debug, Clone)]
pub struct UserUrlToken {
    /// The target user.
    pub user: User,
    /// The action URL (carries the token).
    pub url: String,
    /// The raw token (in case the app wants a custom route instead of `url`).
    pub token: String,
}

/// `{ user }` passed to user-event callbacks (`onPasswordReset` / `onExistingUserSignUp`).
#[derive(Debug, Clone)]
pub struct UserData {
    /// The user.
    pub user: User,
}

/// `{ user, newEmail, url, token }` passed to `sendChangeEmailConfirmation`.
#[derive(Debug, Clone)]
pub struct ChangeEmailData {
    /// The user changing their email.
    pub user: User,
    /// The requested new email.
    pub new_email: String,
    /// The confirmation URL (carries the token).
    pub url: String,
    /// The raw token.
    pub token: String,
}

/// Send an email about a `{ user, url, token }` action (`sendVerificationEmail`,
/// `sendResetPassword`, `sendDeleteAccountVerification`).
pub type SendUserEmailFn = Arc<
    dyn Fn(UserUrlToken, Option<Arc<GenericEndpointContext>>) -> BoxFuture<CallbackResult>
        + Send
        + Sync,
>;

/// A user-lifecycle callback (`beforeEmailVerification`, `afterEmailVerification`, `beforeDelete`,
/// `afterDelete`).
pub type UserEventFn = Arc<
    dyn Fn(User, Option<Arc<GenericEndpointContext>>) -> BoxFuture<CallbackResult> + Send + Sync,
>;

/// A `{ user }` callback (`onPasswordReset`, `onExistingUserSignUp`).
pub type UserDataFn = Arc<
    dyn Fn(UserData, Option<Arc<GenericEndpointContext>>) -> BoxFuture<CallbackResult>
        + Send
        + Sync,
>;

/// The change-email confirmation callback (`sendChangeEmailConfirmation`).
pub type ChangeEmailFn = Arc<
    dyn Fn(ChangeEmailData, Option<Arc<GenericEndpointContext>>) -> BoxFuture<CallbackResult>
        + Send
        + Sync,
>;

/// Core fields handed to [`CustomSyntheticUserFn`].
#[derive(Debug, Clone)]
pub struct SyntheticCoreFields {
    /// Display name.
    pub name: String,
    /// Email.
    pub email: String,
    /// Verified flag.
    pub email_verified: bool,
    /// Avatar URL.
    pub image: Option<String>,
    /// Creation timestamp.
    pub created_at: OffsetDateTime,
    /// Update timestamp.
    pub updated_at: OffsetDateTime,
}

/// Parameters for [`CustomSyntheticUserFn`] (`customSyntheticUser`).
#[derive(Debug, Clone)]
pub struct SyntheticUserParams {
    /// The core user fields.
    pub core_fields: SyntheticCoreFields,
    /// Processed additional fields (with defaults applied).
    pub additional_fields: Map<String, Value>,
    /// The generated user id.
    pub id: String,
}

/// Build a synthetic user for email-enumeration protection (`customSyntheticUser`). Synchronous,
/// returning the fake user record as a column map.
pub type CustomSyntheticUserFn =
    Arc<dyn Fn(SyntheticUserParams) -> Map<String, Value> + Send + Sync>;

/// User-overridable password hashing/verification (`emailAndPassword.password`).
#[derive(Clone, Default)]
pub struct PasswordHashVerify {
    /// Hash a password.
    pub hash: Option<PasswordHashFn>,
    /// Verify a password against a hash.
    pub verify: Option<PasswordVerifyFn>,
}

/// Email-verification configuration (`emailVerification`).
#[derive(Clone, Default)]
pub struct EmailVerificationOptions {
    /// Send a verification email.
    pub send_verification_email: Option<SendUserEmailFn>,
    /// Send a verification email automatically after sign up.
    pub send_on_sign_up: Option<bool>,
    /// Send a verification email on sign in when the email is unverified.
    pub send_on_sign_in: Option<bool>,
    /// Auto sign-in the user after they verify.
    pub auto_sign_in_after_verification: Option<bool>,
    /// Verification token lifetime, in seconds (default 3600).
    pub expires_in: Option<u64>,
    /// Called before a user verifies their email.
    pub before_email_verification: Option<UserEventFn>,
    /// Called after a user's email becomes verified.
    pub after_email_verification: Option<UserEventFn>,
}

/// Email-and-password authentication configuration (`emailAndPassword`).
#[derive(Clone, Default)]
pub struct EmailAndPasswordOptions {
    /// Enable email/password authentication.
    pub enabled: bool,
    /// Disable email/password sign up.
    pub disable_sign_up: Option<bool>,
    /// Require email verification before a session can be created.
    pub require_email_verification: Option<bool>,
    /// Maximum password length (default 128).
    pub max_password_length: Option<usize>,
    /// Minimum password length (default 8).
    pub min_password_length: Option<usize>,
    /// Send a reset-password email.
    pub send_reset_password: Option<SendUserEmailFn>,
    /// Reset-password token lifetime, in seconds (default 3600).
    pub reset_password_token_expires_in: Option<u64>,
    /// Called after a password is reset.
    pub on_password_reset: Option<UserDataFn>,
    /// Custom password hashing/verification (default: argon2id).
    pub password: Option<PasswordHashVerify>,
    /// Auto sign-in after sign up (default true).
    pub auto_sign_in: Option<bool>,
    /// Revoke all other sessions on password reset (default false).
    pub revoke_sessions_on_password_reset: Option<bool>,
    /// Called when a sign-up is attempted with an existing email (enumeration-protection paths).
    pub on_existing_user_sign_up: Option<UserDataFn>,
    /// Build the synthetic user for enumeration-protection responses.
    pub custom_synthetic_user: Option<CustomSyntheticUserFn>,
}

/// Change-email configuration (`user.changeEmail`).
#[derive(Clone, Default)]
pub struct ChangeEmailOptions {
    /// Enable changing email.
    pub enabled: bool,
    /// Send a confirmation to the old address when the email changes.
    pub send_change_email_confirmation: Option<ChangeEmailFn>,
    /// Update the email without verification when the user is unverified (default false).
    pub update_email_without_verification: Option<bool>,
}

/// User-deletion configuration (`user.deleteUser`).
#[derive(Clone, Default)]
pub struct DeleteUserOptions {
    /// Enable user deletion.
    pub enabled: Option<bool>,
    /// Send a verification email before deleting (otherwise the user is deleted immediately).
    pub send_delete_account_verification: Option<SendUserEmailFn>,
    /// Called before a user is deleted.
    pub before_delete: Option<UserEventFn>,
    /// Called after a user is deleted.
    pub after_delete: Option<UserEventFn>,
    /// Delete-token lifetime, in seconds (default 1 day).
    pub delete_token_expires_in: Option<u64>,
}

/// User configuration (`user`).
#[derive(Clone, Default)]
pub struct UserOptions {
    /// Per-model database options (table/field/extra-field mapping).
    pub db: BetterAuthDbOptions,
    /// Change-email configuration.
    pub change_email: Option<ChangeEmailOptions>,
    /// User-deletion configuration.
    pub delete_user: Option<DeleteUserOptions>,
}

/// Cookie-cache encoding strategy (`session.cookieCache.strategy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CookieCacheStrategy {
    /// base64url + HMAC-SHA256 (the default; compact).
    #[default]
    Compact,
    /// JWT with HMAC signature (no encryption).
    Jwt,
    /// JWE (encrypted) with A256CBC-HS512 + HKDF.
    Jwe,
}

/// Stateless cookie-cache refresh policy (`session.cookieCache.refreshCache`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookieCacheRefresh {
    /// Disable automatic refresh (`false`).
    Disabled,
    /// Refresh before expiry; `update_age` (seconds) defaults to 20% of `max_age` when `None`.
    Enabled {
        /// Seconds before expiry at which to refresh.
        update_age: Option<u64>,
    },
}

/// Cookie-cache version (`session.cookieCache.version`): a static string or a per-session function.
#[derive(Clone)]
pub enum CookieCacheVersion {
    /// A fixed version string.
    Static(String),
    /// Derive the version from the session + user.
    Dynamic(Arc<dyn Fn(Session, User) -> BoxFuture<String> + Send + Sync>),
}

/// Cookie session-cache configuration (`session.cookieCache`).
#[derive(Clone, Default)]
pub struct CookieCacheOptions {
    /// Max age of the cookie, in seconds (default 300).
    pub max_age: Option<u64>,
    /// Enable cookie session caching (default false).
    pub enabled: Option<bool>,
    /// Encoding strategy.
    pub strategy: Option<CookieCacheStrategy>,
    /// Stateless refresh policy.
    pub refresh_cache: Option<CookieCacheRefresh>,
    /// Cache version (changing it invalidates existing caches).
    pub version: Option<CookieCacheVersion>,
}

/// Session configuration (`session`).
#[derive(Clone, Default)]
pub struct SessionOptions {
    /// Per-model database options.
    pub db: BetterAuthDbOptions,
    /// Session lifetime, in seconds (default 7 days).
    pub expires_in: Option<u64>,
    /// Refresh interval, in seconds (default 1 day; `0` refreshes every use).
    pub update_age: Option<u64>,
    /// Disable session refresh entirely.
    pub disable_session_refresh: Option<bool>,
    /// Defer refresh writes to POST requests (read-replica setups).
    pub defer_session_refresh: Option<bool>,
    /// Also store the session in the database when secondary storage is configured.
    pub store_session_in_database: Option<bool>,
    /// Preserve database session rows even when revoked from secondary storage.
    pub preserve_session_in_database: Option<bool>,
    /// Cookie session-cache configuration.
    pub cookie_cache: Option<CookieCacheOptions>,
    /// Freshness window for sensitive operations, in seconds (default 1 day).
    pub fresh_age: Option<u64>,
}

/// Trusted providers for account linking (`account.accountLinking.trustedProviders`): a static list
/// or a per-request function.
#[derive(Clone)]
pub enum TrustedProviders {
    /// A static allow-list.
    Static(Vec<String>),
    /// Resolved per request (must tolerate a `None` context during init).
    Dynamic(RequestStringListFn),
}

/// Account-linking configuration (`account.accountLinking`).
#[derive(Clone, Default)]
pub struct AccountLinkingOptions {
    /// Enable account linking (default true).
    pub enabled: Option<bool>,
    /// Disable implicit linking on OAuth sign-in (default false).
    pub disable_implicit_linking: Option<bool>,
    /// Require the local row to be `email_verified` before implicit linking trusts the IdP claim
    /// (default true; guards against pre-registration takeover).
    pub require_local_email_verified: Option<bool>,
    /// Trusted providers.
    pub trusted_providers: Option<TrustedProviders>,
    /// Allow linking accounts with different emails (default false; takeover risk).
    pub allow_different_emails: Option<bool>,
    /// Allow unlinking the last account (default false).
    pub allow_unlinking_all: Option<bool>,
    /// Copy provider profile onto the local user on link (never changes email; default false).
    pub update_user_info_on_link: Option<bool>,
}

/// Account configuration (`account`).
#[derive(Clone, Default)]
pub struct AccountOptions {
    /// Per-model database options.
    pub db: BetterAuthDbOptions,
    /// Refresh stored token data from the provider on sign in (default true).
    pub update_account_on_sign_in: Option<bool>,
    /// Account-linking configuration.
    pub account_linking: Option<AccountLinkingOptions>,
    /// Encrypt OAuth tokens at rest with AES-256-GCM (default false).
    pub encrypt_oauth_tokens: Option<bool>,
    /// Skip the OAuth state-cookie check (dangerous; default false).
    pub skip_state_cookie_check: Option<bool>,
    /// Strategy for storing OAuth `state`.
    pub store_state_strategy: Option<StoreStateStrategy>,
    /// Store post-OAuth account data in an encrypted cookie (database-less flows; default false).
    pub store_account_cookie: Option<bool>,
}

/// How a verification identifier is stored (`verification.storeIdentifier`): one policy, or a
/// default with per-type overrides.
#[derive(Clone)]
pub enum StoreIdentifierConfig {
    /// A single storage policy for all identifiers.
    Single(StoreIdentifierOption),
    /// A default policy plus per-type overrides.
    PerType {
        /// The default policy.
        default: StoreIdentifierOption,
        /// Per-type overrides, keyed by verification type.
        overrides: Option<BTreeMap<String, StoreIdentifierOption>>,
    },
}

/// Verification configuration (`verification`).
#[derive(Clone, Default)]
pub struct VerificationOptions {
    /// Per-model database options.
    pub db: BetterAuthDbOptions,
    /// Disable cleaning up expired values on fetch.
    pub disable_cleanup: Option<bool>,
    /// How to store identifiers (tokens, OTPs, …) — defaults to plain.
    pub store_identifier: Option<StoreIdentifierConfig>,
    /// Store verification data in the database even with secondary storage (default false).
    pub store_in_database: Option<bool>,
}

/// Additional trusted origins (`trustedOrigins`): a static list (wildcards allowed) or a per-request
/// function.
#[derive(Clone)]
pub enum TrustedOrigins {
    /// A static list of origins (supports wildcard patterns like `https://*.example.com`).
    Static(Vec<String>),
    /// Resolved per request (must tolerate a `None` context).
    Dynamic(RequestStringListFn),
}

/// The outcome of a `before` database hook (`boolean | void | { data }`).
#[derive(Debug, Clone)]
pub enum BeforeHookOutcome<T> {
    /// Proceed unchanged (the `void`/`true` case).
    Continue,
    /// Abort the operation (the `false` case).
    Abort,
    /// Proceed with replaced data (the `{ data }` case).
    Replace(T),
}

/// A `before` create/update database hook.
pub type BeforeHook<T> = Arc<
    dyn Fn(
            T,
            Option<Arc<GenericEndpointContext>>,
        ) -> BoxFuture<Result<BeforeHookOutcome<T>, CallbackError>>
        + Send
        + Sync,
>;

/// An `after` database hook (create/update/delete).
pub type AfterHook<T> =
    Arc<dyn Fn(T, Option<Arc<GenericEndpointContext>>) -> BoxFuture<CallbackResult> + Send + Sync>;

/// A `before` delete database hook (`boolean | void`): `Ok(true)` proceeds, `Ok(false)` aborts.
pub type BeforeDeleteHook<T> = Arc<
    dyn Fn(T, Option<Arc<GenericEndpointContext>>) -> BoxFuture<Result<bool, CallbackError>>
        + Send
        + Sync,
>;

/// Create/update hooks for an entity (`{ before?, after? }`).
#[derive(Clone)]
pub struct CreateUpdateHooks<T> {
    /// Runs before the operation; may abort or replace the data.
    pub before: Option<BeforeHook<T>>,
    /// Runs after the operation.
    pub after: Option<AfterHook<T>>,
}

impl<T> Default for CreateUpdateHooks<T> {
    fn default() -> Self {
        Self {
            before: None,
            after: None,
        }
    }
}

/// Delete hooks for an entity (`{ before?, after? }`).
#[derive(Clone)]
pub struct DeleteHooks<T> {
    /// Runs before the delete; may abort.
    pub before: Option<BeforeDeleteHook<T>>,
    /// Runs after the delete.
    pub after: Option<AfterHook<T>>,
}

impl<T> Default for DeleteHooks<T> {
    fn default() -> Self {
        Self {
            before: None,
            after: None,
        }
    }
}

/// Create/update/delete hooks for one entity.
#[derive(Clone)]
pub struct EntityHooks<T> {
    /// Create hooks.
    pub create: Option<CreateUpdateHooks<T>>,
    /// Update hooks.
    pub update: Option<CreateUpdateHooks<T>>,
    /// Delete hooks.
    pub delete: Option<DeleteHooks<T>>,
}

impl<T> Default for EntityHooks<T> {
    fn default() -> Self {
        Self {
            create: None,
            update: None,
            delete: None,
        }
    }
}

/// Custom hooks around core database operations (`databaseHooks`).
#[derive(Clone, Default)]
pub struct DatabaseHooks {
    /// User hooks.
    pub user: Option<EntityHooks<User>>,
    /// Session hooks.
    pub session: Option<EntityHooks<Session>>,
    /// Account hooks.
    pub account: Option<EntityHooks<Account>>,
    /// Verification hooks.
    pub verification: Option<EntityHooks<Verification>>,
}

/// Custom API-error handler (`onAPIError.onError`).
pub type OnApiErrorFn = Arc<dyn Fn(CallbackError, Arc<AuthContext>) -> BoxFuture<()> + Send + Sync>;

/// Color overrides for the default error page (`onAPIError.customizeDefaultErrorPage.colors`).
#[derive(Debug, Clone, Default)]
pub struct ErrorPageColors {
    /// `background`
    pub background: Option<String>,
    /// `foreground`
    pub foreground: Option<String>,
    /// `primary`
    pub primary: Option<String>,
    /// `primaryForeground`
    pub primary_foreground: Option<String>,
    /// `mutedForeground`
    pub muted_foreground: Option<String>,
    /// `border`
    pub border: Option<String>,
    /// `destructive`
    pub destructive: Option<String>,
    /// `titleBorder`
    pub title_border: Option<String>,
    /// `titleColor`
    pub title_color: Option<String>,
    /// `gridColor`
    pub grid_color: Option<String>,
    /// `cardBackground`
    pub card_background: Option<String>,
    /// `cornerBorder`
    pub corner_border: Option<String>,
}

/// Size overrides for the default error page (`onAPIError.customizeDefaultErrorPage.size`).
#[derive(Debug, Clone, Default)]
pub struct ErrorPageSize {
    /// `radiusSm`
    pub radius_sm: Option<String>,
    /// `radiusMd`
    pub radius_md: Option<String>,
    /// `radiusLg`
    pub radius_lg: Option<String>,
    /// `textSm`
    pub text_sm: Option<String>,
    /// `text2xl`
    pub text_2xl: Option<String>,
    /// `text4xl`
    pub text_4xl: Option<String>,
    /// `text6xl`
    pub text_6xl: Option<String>,
}

/// Font overrides for the default error page (`onAPIError.customizeDefaultErrorPage.font`).
#[derive(Debug, Clone, Default)]
pub struct ErrorPageFont {
    /// `defaultFamily`
    pub default_family: Option<String>,
    /// `monoFamily`
    pub mono_family: Option<String>,
}

/// Customization of the default error page (`onAPIError.customizeDefaultErrorPage`).
#[derive(Debug, Clone, Default)]
pub struct ErrorPageCustomization {
    /// Color overrides.
    pub colors: Option<ErrorPageColors>,
    /// Size overrides.
    pub size: Option<ErrorPageSize>,
    /// Font overrides.
    pub font: Option<ErrorPageFont>,
    /// Hide the title border.
    pub disable_title_border: Option<bool>,
    /// Hide the corner decorations.
    pub disable_corner_decorations: Option<bool>,
    /// Hide the background grid.
    pub disable_background_grid: Option<bool>,
}

/// API error-handling configuration (`onAPIError`).
#[derive(Clone, Default)]
pub struct OnApiErrorOptions {
    /// Throw on API error instead of returning it (default false).
    pub throw: Option<bool>,
    /// Custom error handler.
    pub on_error: Option<OnApiErrorFn>,
    /// URL to redirect to on error (default `/api/auth/error`).
    pub error_url: Option<String>,
    /// Default-error-page customization.
    pub customize_default_error_page: Option<ErrorPageCustomization>,
}

/// Telemetry configuration (`telemetry`).
#[derive(Debug, Clone, Default)]
pub struct TelemetryOptions {
    /// Enable telemetry collection (default false).
    pub enabled: Option<bool>,
    /// Enable telemetry debug mode (default false).
    pub debug: Option<bool>,
}

/// Experimental feature flags (`experimental`).
#[derive(Debug, Clone, Default)]
pub struct ExperimentalOptions {
    /// Enable experimental adapter joins (default false; not all adapters support them).
    pub joins: Option<bool>,
}

/// Request-pipeline hooks (`hooks`): middleware run before/after every request.
#[derive(Clone, Default)]
pub struct AuthHooks {
    /// Run before a request is processed.
    pub before: Option<AuthMiddleware>,
    /// Run after a request is processed.
    pub after: Option<AuthMiddleware>,
}

impl core::fmt::Debug for AuthHooks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AuthHooks")
            .field("before", &self.before.as_ref().map(|_| "<fn>"))
            .field("after", &self.after.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

/// The top-level configuration for an instance (`BetterAuthOptions`).
///
/// Every field is optional/defaulted so a minimal `BetterAuthOptions::default()` is valid; the
/// published crate's init layer resolves the effective configuration onto an
/// [`AuthContext`](crate::types::context::AuthContext). The top-level `hooks` (before/after
/// `AuthMiddleware`) is added with the api anchors batch.
#[derive(Clone, Default)]
pub struct BetterAuthOptions {
    /// Application display name (`appName`; default "Better Auth").
    pub app_name: Option<String>,
    /// Server root URL (`baseURL`).
    pub base_url: Option<BaseUrlConfig>,
    /// Auth API mount prefix (`basePath`; default `/api/auth`).
    pub base_path: Option<String>,
    /// Single signing/encryption secret (`secret`). Never logged.
    pub secret: Option<String>,
    /// Versioned secrets for rotation (`secrets`). Never logged.
    pub secrets: Option<Vec<SecretEntry>>,
    /// The database adapter (`database`). Provided by a storage-adapter crate.
    pub database: Option<Arc<dyn DatabaseAdapter>>,
    /// Secondary storage for sessions/rate-limits (`secondaryStorage`).
    pub secondary_storage: Option<Arc<dyn SecondaryStorage>>,
    /// Email-verification configuration (`emailVerification`).
    pub email_verification: Option<EmailVerificationOptions>,
    /// Email-and-password configuration (`emailAndPassword`).
    pub email_and_password: Option<EmailAndPasswordOptions>,
    /// Instantiated social providers (`socialProviders`).
    pub social_providers: Vec<DynOAuthProvider>,
    /// Loaded server plugins (`plugins`).
    pub plugins: Vec<Arc<dyn BetterAuthPlugin>>,
    /// User configuration (`user`).
    pub user: Option<UserOptions>,
    /// Session configuration (`session`).
    pub session: Option<SessionOptions>,
    /// Account configuration (`account`).
    pub account: Option<AccountOptions>,
    /// Verification configuration (`verification`).
    pub verification: Option<VerificationOptions>,
    /// Additional trusted origins (`trustedOrigins`).
    pub trusted_origins: Option<TrustedOrigins>,
    /// Rate-limiting configuration (`rateLimit`).
    pub rate_limit: Option<BetterAuthRateLimitOptions>,
    /// Advanced options (`advanced`).
    pub advanced: Option<BetterAuthAdvancedOptions>,
    /// Custom logger (`logger`).
    pub logger: Option<Logger>,
    /// Lifecycle hooks around core database operations (`databaseHooks`).
    pub database_hooks: Option<DatabaseHooks>,
    /// API error handling (`onAPIError`).
    pub on_api_error: Option<OnApiErrorOptions>,
    /// Request-pipeline hooks run before/after every request (`hooks`).
    pub hooks: Option<AuthHooks>,
    /// Paths to disable (`disabledPaths`).
    pub disabled_paths: Option<Vec<String>>,
    /// Telemetry configuration (`telemetry`).
    pub telemetry: Option<TelemetryOptions>,
    /// Experimental feature flags (`experimental`).
    pub experimental: Option<ExperimentalOptions>,
}

impl core::fmt::Debug for BetterAuthOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Print plain-data fields; redact secrets and placeholder closure-bearing groups.
        f.debug_struct("BetterAuthOptions")
            .field("app_name", &self.app_name)
            .field("base_url", &self.base_url)
            .field("base_path", &self.base_path)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .field(
                "secrets",
                &self
                    .secrets
                    .as_ref()
                    .map(|s| format!("<{} secret(s) redacted>", s.len())),
            )
            .field(
                "database",
                &self.database.as_ref().map(|_| "<dyn DatabaseAdapter>"),
            )
            .field(
                "secondary_storage",
                &self
                    .secondary_storage
                    .as_ref()
                    .map(|_| "<dyn SecondaryStorage>"),
            )
            .field(
                "email_verification",
                &self.email_verification.as_ref().map(|_| "<options>"),
            )
            .field(
                "email_and_password",
                &self.email_and_password.as_ref().map(|_| "<options>"),
            )
            .field(
                "social_providers",
                &format_args!("[{} provider(s)]", self.social_providers.len()),
            )
            .field(
                "plugins",
                &format_args!("[{} plugin(s)]", self.plugins.len()),
            )
            .field("user", &self.user.as_ref().map(|_| "<options>"))
            .field("session", &self.session.as_ref().map(|_| "<options>"))
            .field("account", &self.account.as_ref().map(|_| "<options>"))
            .field(
                "verification",
                &self.verification.as_ref().map(|_| "<options>"),
            )
            .field(
                "trusted_origins",
                &self.trusted_origins.as_ref().map(|_| "<origins>"),
            )
            .field("rate_limit", &self.rate_limit)
            .field("advanced", &self.advanced)
            .field("logger", &self.logger)
            .field(
                "database_hooks",
                &self.database_hooks.as_ref().map(|_| "<hooks>"),
            )
            .field(
                "on_api_error",
                &self.on_api_error.as_ref().map(|_| "<options>"),
            )
            .field("hooks", &self.hooks)
            .field("disabled_paths", &self.disabled_paths)
            .field("telemetry", &self.telemetry)
            .field("experimental", &self.experimental)
            .finish()
    }
}

// Redacting / placeholdering `Debug` impls for the closure-bearing option groups (the crate denies
// `missing_debug_implementations`; closures and secret material are never printed).

impl core::fmt::Debug for EmailVerificationOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EmailVerificationOptions")
            .field(
                "send_verification_email",
                &self.send_verification_email.as_ref().map(|_| "<fn>"),
            )
            .field("send_on_sign_up", &self.send_on_sign_up)
            .field("send_on_sign_in", &self.send_on_sign_in)
            .field(
                "auto_sign_in_after_verification",
                &self.auto_sign_in_after_verification,
            )
            .field("expires_in", &self.expires_in)
            .field(
                "before_email_verification",
                &self.before_email_verification.as_ref().map(|_| "<fn>"),
            )
            .field(
                "after_email_verification",
                &self.after_email_verification.as_ref().map(|_| "<fn>"),
            )
            .finish()
    }
}

impl core::fmt::Debug for EmailAndPasswordOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EmailAndPasswordOptions")
            .field("enabled", &self.enabled)
            .field("disable_sign_up", &self.disable_sign_up)
            .field(
                "require_email_verification",
                &self.require_email_verification,
            )
            .field("max_password_length", &self.max_password_length)
            .field("min_password_length", &self.min_password_length)
            .field(
                "send_reset_password",
                &self.send_reset_password.as_ref().map(|_| "<fn>"),
            )
            .field(
                "reset_password_token_expires_in",
                &self.reset_password_token_expires_in,
            )
            .field(
                "on_password_reset",
                &self.on_password_reset.as_ref().map(|_| "<fn>"),
            )
            .field("password", &self.password)
            .field("auto_sign_in", &self.auto_sign_in)
            .field(
                "revoke_sessions_on_password_reset",
                &self.revoke_sessions_on_password_reset,
            )
            .field(
                "on_existing_user_sign_up",
                &self.on_existing_user_sign_up.as_ref().map(|_| "<fn>"),
            )
            .field(
                "custom_synthetic_user",
                &self.custom_synthetic_user.as_ref().map(|_| "<fn>"),
            )
            .finish()
    }
}

impl core::fmt::Debug for PasswordHashVerify {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PasswordHashVerify")
            .field("hash", &self.hash.as_ref().map(|_| "<fn>"))
            .field("verify", &self.verify.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl core::fmt::Debug for UserOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UserOptions")
            .field("db", &self.db)
            .field("change_email", &self.change_email)
            .field("delete_user", &self.delete_user)
            .finish()
    }
}

impl core::fmt::Debug for ChangeEmailOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ChangeEmailOptions")
            .field("enabled", &self.enabled)
            .field(
                "send_change_email_confirmation",
                &self.send_change_email_confirmation.as_ref().map(|_| "<fn>"),
            )
            .field(
                "update_email_without_verification",
                &self.update_email_without_verification,
            )
            .finish()
    }
}

impl core::fmt::Debug for DeleteUserOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DeleteUserOptions")
            .field("enabled", &self.enabled)
            .field(
                "send_delete_account_verification",
                &self
                    .send_delete_account_verification
                    .as_ref()
                    .map(|_| "<fn>"),
            )
            .field(
                "before_delete",
                &self.before_delete.as_ref().map(|_| "<fn>"),
            )
            .field("after_delete", &self.after_delete.as_ref().map(|_| "<fn>"))
            .field("delete_token_expires_in", &self.delete_token_expires_in)
            .finish()
    }
}

impl core::fmt::Debug for CookieCacheVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Static(v) => f.debug_tuple("Static").field(v).finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(<fn>)"),
        }
    }
}

impl core::fmt::Debug for CookieCacheOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CookieCacheOptions")
            .field("max_age", &self.max_age)
            .field("enabled", &self.enabled)
            .field("strategy", &self.strategy)
            .field("refresh_cache", &self.refresh_cache)
            .field("version", &self.version)
            .finish()
    }
}

impl core::fmt::Debug for SessionOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SessionOptions")
            .field("db", &self.db)
            .field("expires_in", &self.expires_in)
            .field("update_age", &self.update_age)
            .field("disable_session_refresh", &self.disable_session_refresh)
            .field("defer_session_refresh", &self.defer_session_refresh)
            .field("store_session_in_database", &self.store_session_in_database)
            .field(
                "preserve_session_in_database",
                &self.preserve_session_in_database,
            )
            .field("cookie_cache", &self.cookie_cache)
            .field("fresh_age", &self.fresh_age)
            .finish()
    }
}

impl core::fmt::Debug for TrustedProviders {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Static(v) => f.debug_tuple("Static").field(v).finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(<fn>)"),
        }
    }
}

impl core::fmt::Debug for AccountLinkingOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccountLinkingOptions")
            .field("enabled", &self.enabled)
            .field("disable_implicit_linking", &self.disable_implicit_linking)
            .field(
                "require_local_email_verified",
                &self.require_local_email_verified,
            )
            .field("trusted_providers", &self.trusted_providers)
            .field("allow_different_emails", &self.allow_different_emails)
            .field("allow_unlinking_all", &self.allow_unlinking_all)
            .field("update_user_info_on_link", &self.update_user_info_on_link)
            .finish()
    }
}

impl core::fmt::Debug for AccountOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AccountOptions")
            .field("db", &self.db)
            .field("update_account_on_sign_in", &self.update_account_on_sign_in)
            .field("account_linking", &self.account_linking)
            .field("encrypt_oauth_tokens", &self.encrypt_oauth_tokens)
            .field("skip_state_cookie_check", &self.skip_state_cookie_check)
            .field("store_state_strategy", &self.store_state_strategy)
            .field("store_account_cookie", &self.store_account_cookie)
            .finish()
    }
}

impl core::fmt::Debug for StoreIdentifierConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Single(o) => f.debug_tuple("Single").field(o).finish(),
            Self::PerType { default, overrides } => f
                .debug_struct("PerType")
                .field("default", default)
                .field("overrides", overrides)
                .finish(),
        }
    }
}

impl core::fmt::Debug for VerificationOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VerificationOptions")
            .field("db", &self.db)
            .field("disable_cleanup", &self.disable_cleanup)
            .field("store_identifier", &self.store_identifier)
            .field("store_in_database", &self.store_in_database)
            .finish()
    }
}

impl core::fmt::Debug for TrustedOrigins {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Static(v) => f.debug_tuple("Static").field(v).finish(),
            Self::Dynamic(_) => f.write_str("Dynamic(<fn>)"),
        }
    }
}

impl<T> core::fmt::Debug for CreateUpdateHooks<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CreateUpdateHooks")
            .field("before", &self.before.as_ref().map(|_| "<fn>"))
            .field("after", &self.after.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl<T> core::fmt::Debug for DeleteHooks<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DeleteHooks")
            .field("before", &self.before.as_ref().map(|_| "<fn>"))
            .field("after", &self.after.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl<T> core::fmt::Debug for EntityHooks<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EntityHooks")
            .field("create", &self.create)
            .field("update", &self.update)
            .field("delete", &self.delete)
            .finish()
    }
}

impl core::fmt::Debug for DatabaseHooks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DatabaseHooks")
            .field("user", &self.user)
            .field("session", &self.session)
            .field("account", &self.account)
            .field("verification", &self.verification)
            .finish()
    }
}

impl core::fmt::Debug for OnApiErrorOptions {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OnApiErrorOptions")
            .field("throw", &self.throw)
            .field("on_error", &self.on_error.as_ref().map(|_| "<fn>"))
            .field("error_url", &self.error_url)
            .field(
                "customize_default_error_page",
                &self.customize_default_error_page,
            )
            .finish()
    }
}

#[cfg(test)]
#[path = "init_options.test.rs"]
mod init_options_tests;

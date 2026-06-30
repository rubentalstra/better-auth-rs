//! Behavior tests for the core type hub: `AuthContext`, `GenericEndpointContext`, the
//! `InternalAdapter` trait surface, and `BetterAuthPlugin` wiring through the context.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use time::{Duration, OffsetDateTime};

use super::*;
use crate::api::HookEndpointContext;
use crate::db::adapter::{
    AdapterResult, CountArgs, CreateArgs, DatabaseAdapter, DatabaseTransaction, DeleteArgs,
    FindManyArgs, FindOneArgs, IncrementOneArgs, Row, TransactionAdapter, UpdateArgs,
};
use crate::db::schema::{CoreFields, User};
use crate::env::logger::create_logger;
use crate::types::cookie::{BetterAuthCookie, BetterAuthCookies, CookieOptions};
use crate::types::init_options::{
    BetterAuthOptions, GenerateIdConfig, GenerateIdFn, GenerateIdInput,
};
use crate::types::plugin::BetterAuthPlugin;

// ── stubs ────────────────────────────────────────────────────────────────────

/// A database adapter that panics on use — present only so an `AuthContext` can be constructed.
/// None of the context-hub tests below touch the data layer.
struct StubAdapter;

#[async_trait::async_trait]
impl TransactionAdapter for StubAdapter {
    async fn create(&self, _args: CreateArgs) -> AdapterResult<Row> {
        unimplemented!()
    }
    async fn find_one(&self, _args: FindOneArgs) -> AdapterResult<Option<Row>> {
        unimplemented!()
    }
    async fn find_many(&self, _args: FindManyArgs) -> AdapterResult<Vec<Row>> {
        unimplemented!()
    }
    async fn count(&self, _args: CountArgs) -> AdapterResult<u64> {
        unimplemented!()
    }
    async fn update(&self, _args: UpdateArgs) -> AdapterResult<Option<Row>> {
        unimplemented!()
    }
    async fn update_many(&self, _args: UpdateArgs) -> AdapterResult<u64> {
        unimplemented!()
    }
    async fn delete(&self, _args: DeleteArgs) -> AdapterResult<()> {
        unimplemented!()
    }
    async fn delete_many(&self, _args: DeleteArgs) -> AdapterResult<u64> {
        unimplemented!()
    }
    async fn consume_one(&self, _args: DeleteArgs) -> AdapterResult<Option<Row>> {
        unimplemented!()
    }
    async fn increment_one(&self, _args: IncrementOneArgs) -> AdapterResult<Option<Row>> {
        unimplemented!()
    }
}

#[async_trait::async_trait]
impl DatabaseAdapter for StubAdapter {
    fn id(&self) -> &str {
        "stub"
    }
    async fn begin_transaction(&self) -> AdapterResult<Box<dyn DatabaseTransaction>> {
        unimplemented!()
    }
}

/// An internal adapter that panics on use — present only so an `AuthContext` can be constructed.
struct StubInternalAdapter;

#[async_trait::async_trait]
impl InternalAdapter for StubInternalAdapter {
    async fn create_oauth_user(
        &self,
        _user: NewUser,
        _account: NewOAuthAccount,
    ) -> InternalAdapterResult<OAuthUserCreated> {
        unimplemented!()
    }
    async fn create_user(&self, _user: NewUser) -> InternalAdapterResult<WithExtra<User>> {
        unimplemented!()
    }
    async fn list_users(&self, _options: ListUsersOptions) -> InternalAdapterResult<Vec<User>> {
        unimplemented!()
    }
    async fn count_total_users(
        &self,
        _conditions: Vec<crate::db::adapter::Where>,
    ) -> InternalAdapterResult<u64> {
        unimplemented!()
    }
    async fn find_user_by_email(
        &self,
        _email: &str,
        _options: FindUserByEmailOptions,
    ) -> InternalAdapterResult<Option<UserWithAccounts>> {
        unimplemented!()
    }
    async fn find_user_by_id(&self, _user_id: &str) -> InternalAdapterResult<Option<User>> {
        unimplemented!()
    }
    async fn update_user(
        &self,
        _user_id: &str,
        _data: UserUpdate,
    ) -> InternalAdapterResult<WithExtra<User>> {
        unimplemented!()
    }
    async fn update_user_by_email(
        &self,
        _email: &str,
        _data: UserUpdate,
    ) -> InternalAdapterResult<WithExtra<User>> {
        unimplemented!()
    }
    async fn update_password(&self, _user_id: &str, _password: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn delete_user(&self, _user_id: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn refresh_user_sessions(&self, _user: &User) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn create_account(
        &self,
        _account: NewAccount,
    ) -> InternalAdapterResult<WithExtra<Account>> {
        unimplemented!()
    }
    async fn link_account(&self, _account: NewAccount) -> InternalAdapterResult<Account> {
        unimplemented!()
    }
    async fn find_accounts(&self, _user_id: &str) -> InternalAdapterResult<Vec<Account>> {
        unimplemented!()
    }
    async fn find_account_by_provider_id(
        &self,
        _account_id: &str,
        _provider_id: &str,
    ) -> InternalAdapterResult<Option<Account>> {
        unimplemented!()
    }
    async fn find_account_by_user_id(&self, _user_id: &str) -> InternalAdapterResult<Vec<Account>> {
        unimplemented!()
    }
    async fn find_oauth_user(
        &self,
        _email: &str,
        _account_id: &str,
        _provider_id: &str,
    ) -> InternalAdapterResult<Option<OAuthUserResult>> {
        unimplemented!()
    }
    async fn update_account(
        &self,
        _id: &str,
        _data: AccountUpdate,
    ) -> InternalAdapterResult<Account> {
        unimplemented!()
    }
    async fn delete_account(&self, _id: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn delete_accounts(&self, _user_id: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn create_session(
        &self,
        _user_id: &str,
        _options: CreateSessionOptions,
    ) -> InternalAdapterResult<Session> {
        unimplemented!()
    }
    async fn list_sessions(
        &self,
        _user_id: &str,
        _options: ListSessionsOptions,
    ) -> InternalAdapterResult<Vec<Session>> {
        unimplemented!()
    }
    async fn find_session(&self, _token: &str) -> InternalAdapterResult<Option<SessionWithUser>> {
        unimplemented!()
    }
    async fn find_sessions(
        &self,
        _session_tokens: &[String],
        _options: ListSessionsOptions,
    ) -> InternalAdapterResult<Vec<SessionWithUser>> {
        unimplemented!()
    }
    async fn update_session(
        &self,
        _session_token: &str,
        _session: SessionUpdate,
    ) -> InternalAdapterResult<Option<Session>> {
        unimplemented!()
    }
    async fn delete_session(&self, _token: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn delete_user_sessions(&self, _user_id: &str) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn delete_sessions(&self, _session_tokens: &[String]) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn create_verification_value(
        &self,
        _data: NewVerification,
    ) -> InternalAdapterResult<Verification> {
        unimplemented!()
    }
    async fn find_verification_value(
        &self,
        _identifier: &str,
    ) -> InternalAdapterResult<Option<Verification>> {
        unimplemented!()
    }
    async fn update_verification_by_identifier(
        &self,
        _identifier: &str,
        _data: VerificationUpdate,
    ) -> InternalAdapterResult<Verification> {
        unimplemented!()
    }
    async fn delete_verification_by_identifier(
        &self,
        _identifier: &str,
    ) -> InternalAdapterResult<()> {
        unimplemented!()
    }
    async fn consume_verification_value(
        &self,
        _identifier: &str,
    ) -> InternalAdapterResult<Option<Verification>> {
        unimplemented!()
    }
    async fn reserve_verification_value(
        &self,
        _data: ReserveVerification,
    ) -> InternalAdapterResult<bool> {
        unimplemented!()
    }
}

/// A minimal plugin used to exercise `get_plugin` / `has_plugin`.
struct TestPlugin {
    id: &'static str,
}

impl BetterAuthPlugin for TestPlugin {
    fn id(&self) -> &str {
        self.id
    }
}

// ── builders ─────────────────────────────────────────────────────────────────

fn cookie(name: &str) -> BetterAuthCookie {
    BetterAuthCookie {
        name: name.to_owned(),
        attributes: CookieOptions::default(),
    }
}

const TEST_SECRET: &str = "do-not-leak-this-secret-value";

fn test_context(
    plugins: Vec<Arc<dyn BetterAuthPlugin>>,
    generate_id: GenerateIdConfig,
) -> AuthContext {
    let options = BetterAuthOptions {
        plugins: plugins.clone(),
        ..BetterAuthOptions::default()
    };
    AuthContext {
        app_name: "Test App".to_owned(),
        base_url: "https://example.test".to_owned(),
        version: "0.0.0".to_owned(),
        options,
        trusted_origins: vec!["https://example.test".to_owned()],
        trusted_providers: Vec::new(),
        oauth_config: OAuthConfig::default(),
        social_providers: Vec::new(),
        auth_cookies: BetterAuthCookies {
            session_token: cookie("session_token"),
            session_data: cookie("session_data"),
            account_data: cookie("account_data"),
            dont_remember_token: cookie("dont_remember"),
        },
        logger: Arc::new(create_logger(None)),
        rate_limit: ResolvedRateLimit {
            enabled: false,
            window: 10,
            max: 100,
            storage: crate::types::init_options::RateLimitStorageKind::Memory,
            model_name: None,
            fields: None,
            custom_rules: None,
            custom_storage: None,
        },
        adapter: Arc::new(StubAdapter),
        internal_adapter: Arc::new(StubInternalAdapter),
        secret: TEST_SECRET.to_owned(),
        secret_config: SecretSource::Single(TEST_SECRET.to_owned()),
        session_config: SessionConfig {
            update_age: Duration::days(1),
            expires_in: Duration::days(7),
            fresh_age: Duration::days(1),
            cookie_refresh_cache: CookieRefreshCache::Disabled,
        },
        generate_id,
        secondary_storage: None,
        password: PasswordContext {
            hash: Arc::new(|p| Box::pin(async move { Ok(p) })),
            verify: Arc::new(|_| Box::pin(async move { Ok(true) })),
            config: PasswordConfig {
                min_password_length: 8,
                max_password_length: 128,
            },
        },
        tables: BetterAuthDbSchema::new(),
        skip_origin_check: SkipOriginCheck::None,
        skip_csrf_check: false,
        publish_telemetry: None,
        background_tasks: None,
    }
}

fn sample_session_with_user(id: &str) -> SessionWithUser {
    let now = OffsetDateTime::now_utc();
    SessionWithUser {
        session: Session {
            core: CoreFields::new(format!("sess-{id}")),
            user_id: format!("user-{id}"),
            expires_at: now + Duration::days(7),
            token: format!("token-{id}"),
            ip_address: None,
            user_agent: None,
        },
        user: User::new(format!("user-{id}"), format!("{id}@example.test"), "Tester"),
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[test]
fn get_and_has_plugin_resolve_by_id() {
    let plugins: Vec<Arc<dyn BetterAuthPlugin>> = vec![
        Arc::new(TestPlugin { id: "two-factor" }),
        Arc::new(TestPlugin { id: "organization" }),
    ];
    let ctx = test_context(plugins, GenerateIdConfig::Default);

    assert!(ctx.has_plugin("two-factor"));
    assert!(ctx.has_plugin("organization"));
    assert!(!ctx.has_plugin("passkey"));

    assert_eq!(
        ctx.get_plugin("organization").map(|p| p.id().to_owned()),
        Some("organization".to_owned())
    );
    assert!(ctx.get_plugin("passkey").is_none());
}

#[test]
fn generate_id_dispatches_on_config() {
    // Default -> a generated id.
    let ctx = test_context(Vec::new(), GenerateIdConfig::Default);
    let id = ctx.generate_id("user", None);
    assert!(id.is_some());
    assert!(!id.unwrap().is_empty());

    // Database / Serial / Uuid -> None (the adapter generates the id).
    for cfg in [
        GenerateIdConfig::Database,
        GenerateIdConfig::Serial,
        GenerateIdConfig::Uuid,
    ] {
        let ctx = test_context(Vec::new(), cfg);
        assert_eq!(ctx.generate_id("user", None), None);
    }

    // Custom -> the user's function decides (and sees the model).
    let f: GenerateIdFn = Arc::new(|input: GenerateIdInput| Some(format!("{}-42", input.model)));
    let ctx = test_context(Vec::new(), GenerateIdConfig::Custom(f));
    assert_eq!(
        ctx.generate_id("session", None),
        Some("session-42".to_owned())
    );
}

#[tokio::test]
async fn run_in_background_or_await_runs_inline_without_handler() {
    let ctx = test_context(Vec::new(), GenerateIdConfig::Default);
    let ran = Arc::new(AtomicBool::new(false));
    let flag = ran.clone();
    ctx.run_in_background_or_await(Box::pin(async move {
        flag.store(true, Ordering::SeqCst);
    }))
    .await;
    // No handler is configured, so the future must have been awaited inline (never dropped).
    assert!(ran.load(Ordering::SeqCst));
}

#[test]
fn debug_never_leaks_the_secret() {
    let ctx = test_context(Vec::new(), GenerateIdConfig::Default);
    let rendered = format!("{ctx:?}");
    assert!(
        !rendered.contains(TEST_SECRET),
        "AuthContext Debug leaked the secret"
    );
    assert!(rendered.contains("<redacted>"));
}

#[test]
fn generic_endpoint_context_session_state_is_independent() {
    let ctx = Arc::new(test_context(Vec::new(), GenerateIdConfig::Default));
    let ep = GenericEndpointContext::new(ctx);

    // Both start empty.
    assert!(ep.session().is_none());
    assert!(ep.new_session().is_none());

    // Setting `new_session` does not touch `session`, and vice versa.
    ep.set_new_session(Some(sample_session_with_user("a")));
    assert!(ep.session().is_none());
    assert_eq!(
        ep.new_session().map(|s| s.session.token),
        Some("token-a".to_owned())
    );

    ep.set_session(Some(sample_session_with_user("b")));
    assert_eq!(
        ep.session().map(|s| s.session.token),
        Some("token-b".to_owned())
    );
    assert_eq!(
        ep.new_session().map(|s| s.session.token),
        Some("token-a".to_owned())
    );

    // Clearing works.
    ep.set_new_session(None);
    assert!(ep.new_session().is_none());
    assert_eq!(
        ep.session().map(|s| s.user.email),
        Some("b@example.test".to_owned())
    );
}

#[tokio::test]
async fn plugin_init_default_returns_no_overrides() {
    let ctx = test_context(Vec::new(), GenerateIdConfig::Default);
    let plugin = TestPlugin { id: "x" };
    let result = plugin.init(&ctx).await;
    assert!(result.context.is_none());
    assert!(result.options.is_none());
}

#[test]
fn hook_endpoint_context_starts_empty() {
    let ctx = Arc::new(test_context(Vec::new(), GenerateIdConfig::Default));
    let hook = HookEndpointContext::new(ctx);
    assert!(hook.path.is_none());
    assert!(hook.body.is_none());
    assert!(hook.returned.is_none());
    assert!(hook.response_headers.is_empty());
}

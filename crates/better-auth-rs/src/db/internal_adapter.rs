//! Upstream source: `db/internal-adapter.ts`.
//!
//! The internal (domain) adapter: the typed CRUD surface the auth routes call (`create_user`,
//! `find_user_by_email`, `create_session`, `consume_verification_value`, …), built on the
//! schema-agnostic [`DatabaseAdapter`] via the [`mapping`](super::mapping) layer.
//!
//! Scope notes (kept faithful where it matters; deferred parts are behaviorally inert today):
//! - **`with_hooks`**: upstream routes create/update/delete through `getWithHooks`, which runs
//!   `options.databaseHooks` before/after and queues after-transaction hooks. That lifecycle is
//!   coupled to the context/options/`RequestState` layer and is a no-op with zero hooks registered,
//!   so it is wired in the context step; here we call the adapter directly at the marked seams.
//! - **secondary storage** (Redis): upstream mirrors sessions/verifications into `secondaryStorage`.
//!   We have no secondary storage configured, so only the database path is ported.
//! - **schema defaults**: upstream's adapter *factory* applies declared defaults (e.g. `createdAt`,
//!   `emailVerified=false`). The factory isn't ported yet, so the create methods apply the few
//!   defaults needed for a valid entity inline.
//! - **joins**: `find_user_by_email` returns the user; account joining is a separate call
//!   ([`find_accounts`](InternalAdapter::find_accounts)) until adapter joins land.

use std::sync::Arc;

use better_auth_rs_core::db::{
    Account, AdapterError, BetterAuthDbSchema, CreateArgs, DatabaseAdapter, DbValue, DeleteArgs,
    FindManyArgs, FindOneArgs, Row, Session, SortBy, SortDirection, UpdateArgs, User, Verification,
    Where,
};
use time::OffsetDateTime;

use super::mapping::{MappingError, row_to_entity};
use crate::crypto::random::{Alphabet, generate_random_string_with};

/// Default session lifetime: 7 days (matches upstream `session.expiresIn` default).
const DEFAULT_SESSION_EXPIRATION_SECS: i64 = 60 * 60 * 24 * 7;
/// "Don't remember me" session lifetime: 1 day.
const DONT_REMEMBER_EXPIRATION_SECS: i64 = 60 * 60 * 24;
/// Session token length (upstream `generateId(32)`).
const SESSION_TOKEN_LEN: usize = 32;

/// Errors from the internal adapter.
#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    /// The underlying database adapter failed.
    #[error(transparent)]
    Adapter(#[from] AdapterError),
    /// A row could not be mapped to/from its typed entity.
    #[error(transparent)]
    Mapping(#[from] MappingError),
}

/// The internal (domain) adapter (port of `createInternalAdapter`).
#[derive(Clone)]
pub struct InternalAdapter {
    adapter: Arc<dyn DatabaseAdapter>,
    #[allow(dead_code)] // used by the schema-default/factory work (and SQL migration) as it lands
    tables: Arc<BetterAuthDbSchema>,
    session_expiration_secs: i64,
}

impl std::fmt::Debug for InternalAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalAdapter")
            .field("adapter", &self.adapter.id())
            .field("session_expiration_secs", &self.session_expiration_secs)
            .finish_non_exhaustive()
    }
}

fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

/// Insert `createdAt`/`updatedAt = now` unless the caller already set them (upstream spreads the
/// caller's object *after* the defaults, so caller values win).
fn default_timestamps(data: &mut Row) {
    let ts = DbValue::DateTime(now());
    data.entry("createdAt".into()).or_insert_with(|| ts.clone());
    data.entry("updatedAt".into()).or_insert(ts);
}

/// Lowercase the `email` field in place if present (upstream `email.toLowerCase()`).
fn lowercase_email(data: &mut Row) {
    if let Some(DbValue::String(email)) = data.get("email") {
        let lowered = email.to_lowercase();
        data.insert("email".into(), DbValue::String(lowered));
    }
}

impl InternalAdapter {
    /// Build an internal adapter over a database adapter and the resolved table schema.
    pub fn new(adapter: Arc<dyn DatabaseAdapter>, tables: Arc<BetterAuthDbSchema>) -> Self {
        Self {
            adapter,
            tables,
            session_expiration_secs: DEFAULT_SESSION_EXPIRATION_SECS,
        }
    }

    /// Override the session expiration (seconds). Mirrors `options.session.expiresIn`.
    #[must_use]
    pub fn with_session_expiration(mut self, secs: i64) -> Self {
        self.session_expiration_secs = secs;
        self
    }

    // --- users ------------------------------------------------------------

    /// Create a user (port of `createUser`). Applies `createdAt`/`updatedAt` defaults, lowercases
    /// `email`, and defaults `emailVerified` to `false` when absent.
    pub async fn create_user(&self, mut data: Row) -> Result<User, InternalError> {
        default_timestamps(&mut data);
        lowercase_email(&mut data);
        data.entry("emailVerified".into())
            .or_insert(DbValue::Bool(false));
        let row = self.adapter.create(CreateArgs::new("user", data)).await?;
        Ok(row_to_entity(&row)?)
    }

    /// Find a user by email (lower-cased), or `None` (port of `findUserByEmail`, without the
    /// account join — see [`find_accounts`](Self::find_accounts)).
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, InternalError> {
        let row = self
            .adapter
            .find_one(FindOneArgs::new(
                "user",
                vec![Where::eq("email", email.to_lowercase())],
            ))
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Find a user by id, or `None` (port of `findUserById`).
    pub async fn find_user_by_id(&self, user_id: &str) -> Result<Option<User>, InternalError> {
        if user_id.is_empty() {
            return Ok(None);
        }
        let row = self
            .adapter
            .find_one(FindOneArgs::new("user", vec![Where::eq("id", user_id)]))
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Update a user by id (port of `updateUser`). Lowercases `email` if present.
    pub async fn update_user(
        &self,
        user_id: &str,
        mut data: Row,
    ) -> Result<Option<User>, InternalError> {
        lowercase_email(&mut data);
        let row = self
            .adapter
            .update(UpdateArgs {
                model: "user".into(),
                r#where: vec![Where::eq("id", user_id)],
                update: data,
            })
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    // --- accounts ---------------------------------------------------------

    /// Create an account (port of `createAccount`/`linkAccount`).
    pub async fn create_account(&self, mut data: Row) -> Result<Account, InternalError> {
        default_timestamps(&mut data);
        let row = self
            .adapter
            .create(CreateArgs::new("account", data))
            .await?;
        Ok(row_to_entity(&row)?)
    }

    /// List a user's accounts (port of `findAccounts`).
    pub async fn find_accounts(&self, user_id: &str) -> Result<Vec<Account>, InternalError> {
        let rows = self
            .adapter
            .find_many(FindManyArgs::new("account").filter(vec![Where::eq("userId", user_id)]))
            .await?;
        rows.iter()
            .map(row_to_entity)
            .collect::<Result<_, _>>()
            .map_err(Into::into)
    }

    /// Find an account by `(accountId, providerId)` (port of `findAccountByProviderId`).
    pub async fn find_account_by_provider_id(
        &self,
        account_id: &str,
        provider_id: &str,
    ) -> Result<Option<Account>, InternalError> {
        let row = self
            .adapter
            .find_one(FindOneArgs::new(
                "account",
                vec![
                    Where::eq("accountId", account_id),
                    Where::eq("providerId", provider_id),
                ],
            ))
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Update an account by id (port of `updateAccount`).
    pub async fn update_account(
        &self,
        id: &str,
        data: Row,
    ) -> Result<Option<Account>, InternalError> {
        let row = self
            .adapter
            .update(UpdateArgs {
                model: "account".into(),
                r#where: vec![Where::eq("id", id)],
                update: data,
            })
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Set the password on the credential account(s) for a user (port of `updatePassword`).
    pub async fn update_password(
        &self,
        user_id: &str,
        password: &str,
    ) -> Result<u64, InternalError> {
        let mut update = Row::new();
        update.insert("password".into(), DbValue::String(password.to_string()));
        let affected = self
            .adapter
            .update_many(UpdateArgs {
                model: "account".into(),
                r#where: vec![
                    Where::eq("userId", user_id),
                    Where::eq("providerId", "credential"),
                ],
                update,
            })
            .await?;
        Ok(affected)
    }

    // --- sessions ---------------------------------------------------------

    /// Create a session for a user (port of `createSession`, database path). Generates a 32-char
    /// token and sets expiry (1 day if `dont_remember_me`, else the configured expiration). IP and
    /// user-agent come from the request context (empty here until the context layer threads them).
    pub async fn create_session(
        &self,
        user_id: &str,
        dont_remember_me: bool,
        override_data: Row,
    ) -> Result<Session, InternalError> {
        let secs = if dont_remember_me {
            DONT_REMEMBER_EXPIRATION_SECS
        } else {
            self.session_expiration_secs
        };
        let expires = now() + time::Duration::seconds(secs);

        let mut data = override_data;
        data.remove("id"); // new sessions always get a fresh id
        // defaults the caller's override may replace
        data.entry("ipAddress".into())
            .or_insert(DbValue::String(String::new()));
        data.entry("userAgent".into())
            .or_insert(DbValue::String(String::new()));
        // these always win over the override
        data.insert("expiresAt".into(), DbValue::DateTime(expires));
        data.insert("userId".into(), DbValue::String(user_id.to_string()));
        // Upstream uses `generateId(32)` (core utils/id.ts), whose charset is a-z A-Z 0-9 —
        // NOT `generateRandomString` (which also includes `-`/`_`). Match it exactly.
        data.insert(
            "token".into(),
            DbValue::String(generate_random_string_with(
                SESSION_TOKEN_LEN,
                &[Alphabet::LowerAlpha, Alphabet::UpperAlpha, Alphabet::Digits],
            )),
        );
        data.insert("createdAt".into(), DbValue::DateTime(now()));
        data.insert("updatedAt".into(), DbValue::DateTime(now()));

        let row = self
            .adapter
            .create(CreateArgs::new("session", data))
            .await?;
        Ok(row_to_entity(&row)?)
    }

    /// Find a session by its token (port of `findSession`, database path; returns the session only,
    /// not the joined user).
    pub async fn find_session(&self, token: &str) -> Result<Option<Session>, InternalError> {
        let row = self
            .adapter
            .find_one(FindOneArgs::new("session", vec![Where::eq("token", token)]))
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Update a session by token (port of `updateSession`, database path).
    pub async fn update_session(
        &self,
        token: &str,
        data: Row,
    ) -> Result<Option<Session>, InternalError> {
        let row = self
            .adapter
            .update(UpdateArgs {
                model: "session".into(),
                r#where: vec![Where::eq("token", token)],
                update: data,
            })
            .await?;
        row.as_ref()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Delete a session by token (port of `deleteSession`, database path).
    pub async fn delete_session(&self, token: &str) -> Result<(), InternalError> {
        self.adapter
            .delete(DeleteArgs {
                model: "session".into(),
                r#where: vec![Where::eq("token", token)],
            })
            .await?;
        Ok(())
    }

    /// Delete all sessions for a user (port of `deleteSessions(userId)`).
    pub async fn delete_user_sessions(&self, user_id: &str) -> Result<u64, InternalError> {
        let removed = self
            .adapter
            .delete_many(DeleteArgs {
                model: "session".into(),
                r#where: vec![Where::eq("userId", user_id)],
            })
            .await?;
        Ok(removed)
    }

    // --- verification values ---------------------------------------------

    /// Create a verification value (port of `createVerificationValue`, database path).
    pub async fn create_verification_value(
        &self,
        mut data: Row,
    ) -> Result<Verification, InternalError> {
        default_timestamps(&mut data);
        let row = self
            .adapter
            .create(CreateArgs::new("verification", data))
            .await?;
        Ok(row_to_entity(&row)?)
    }

    /// Find the most recent verification value for an identifier (port of `findVerificationValue`,
    /// database path).
    pub async fn find_verification_value(
        &self,
        identifier: &str,
    ) -> Result<Option<Verification>, InternalError> {
        // Upstream sorts by createdAt desc, limit 1 — the MOST RECENT row when several share an
        // identifier; `find_one` would return the oldest (insertion order).
        let rows = self
            .adapter
            .find_many(
                FindManyArgs::new("verification")
                    .filter(vec![Where::eq("identifier", identifier)])
                    .sort_by(SortBy {
                        field: "createdAt".into(),
                        direction: SortDirection::Desc,
                    })
                    .limit(1),
            )
            .await?;
        rows.first()
            .map(row_to_entity)
            .transpose()
            .map_err(Into::into)
    }

    /// Atomically consume (delete-and-return) a single verification value by id (the database path
    /// of `consumeVerificationValue`). The first caller wins; a second call returns `None`.
    ///
    /// Expired rows are treated as already-invalid: the row is still deleted (so it cannot be
    /// replayed) but `None` is returned — callers don't need their own expiry gate.
    pub async fn consume_verification_value(
        &self,
        id: &str,
    ) -> Result<Option<Verification>, InternalError> {
        let row = self
            .adapter
            .consume_one(DeleteArgs {
                model: "verification".into(),
                r#where: vec![Where::eq("id", id)],
            })
            .await?;
        let consumed: Option<Verification> = row.as_ref().map(row_to_entity).transpose()?;
        Ok(match consumed {
            Some(v) if v.expires_at < now() => None,
            other => other,
        })
    }

    /// Delete all verification values for an identifier (port of `deleteVerificationByIdentifier`).
    pub async fn delete_verification_by_identifier(
        &self,
        identifier: &str,
    ) -> Result<u64, InternalError> {
        let removed = self
            .adapter
            .delete_many(DeleteArgs {
                model: "verification".into(),
                r#where: vec![Where::eq("identifier", identifier)],
            })
            .await?;
        Ok(removed)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use better_auth_rs_core::db::core_tables;
    use better_auth_rs_memory_adapter::MemoryAdapter;

    fn adapter() -> InternalAdapter {
        InternalAdapter::new(Arc::new(MemoryAdapter::new()), Arc::new(core_tables()))
    }

    fn row(pairs: &[(&str, DbValue)]) -> Row {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[tokio::test]
    async fn user_create_find_update() {
        let ia = adapter();
        let created = ia
            .create_user(row(&[
                ("email", "Alice@Example.com".into()),
                ("name", "Alice".into()),
            ]))
            .await
            .unwrap();
        // email lowercased, emailVerified defaulted, id + timestamps present
        assert_eq!(created.email, "alice@example.com");
        assert!(!created.email_verified);
        assert!(!created.id.is_empty());

        let by_email = ia.find_user_by_email("ALICE@example.com").await.unwrap();
        assert_eq!(by_email.unwrap().id, created.id);
        let by_id = ia.find_user_by_id(&created.id).await.unwrap();
        assert_eq!(by_id.unwrap().name, "Alice");
        assert!(ia.find_user_by_id("").await.unwrap().is_none());

        let updated = ia
            .update_user(&created.id, row(&[("name", "Alice B".into())]))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "Alice B");
    }

    #[tokio::test]
    async fn account_create_find_password() {
        let ia = adapter();
        let user = ia
            .create_user(row(&[("email", "u@e.com".into()), ("name", "U".into())]))
            .await
            .unwrap();
        ia.create_account(row(&[
            ("userId", user.id.clone().into()),
            ("providerId", "credential".into()),
            ("accountId", user.id.clone().into()),
            ("password", "hash1".into()),
        ]))
        .await
        .unwrap();

        let accounts = ia.find_accounts(&user.id).await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].provider_id, "credential");

        let found = ia
            .find_account_by_provider_id(&user.id, "credential")
            .await
            .unwrap();
        assert!(found.is_some());

        let n = ia.update_password(&user.id, "hash2").await.unwrap();
        assert_eq!(n, 1);
        let after = ia.find_accounts(&user.id).await.unwrap();
        assert_eq!(after[0].password.as_deref(), Some("hash2"));
    }

    #[tokio::test]
    async fn session_lifecycle() {
        let ia = adapter();
        let session = ia
            .create_session("user-1", false, Row::new())
            .await
            .unwrap();
        assert_eq!(session.user_id, "user-1");
        assert_eq!(session.token.len(), 32);
        // token charset matches upstream generateId(32): a-z A-Z 0-9 only (no '-'/'_')
        assert!(
            session.token.chars().all(|c| c.is_ascii_alphanumeric()),
            "token {} must be alphanumeric",
            session.token
        );
        assert!(session.expires_at > now());

        let found = ia.find_session(&session.token).await.unwrap().unwrap();
        assert_eq!(found.id, session.id);

        ia.update_session(&session.token, row(&[("ipAddress", "1.2.3.4".into())]))
            .await
            .unwrap();
        assert_eq!(
            ia.find_session(&session.token)
                .await
                .unwrap()
                .unwrap()
                .ip_address
                .as_deref(),
            Some("1.2.3.4")
        );

        ia.delete_session(&session.token).await.unwrap();
        assert!(ia.find_session(&session.token).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn dont_remember_me_shortens_expiry() {
        let ia = adapter();
        let short = ia.create_session("u", true, Row::new()).await.unwrap();
        let long = ia.create_session("u", false, Row::new()).await.unwrap();
        assert!(short.expires_at < long.expires_at);
    }

    #[tokio::test]
    async fn delete_user_sessions_removes_all() {
        let ia = adapter();
        ia.create_session("u1", false, Row::new()).await.unwrap();
        ia.create_session("u1", false, Row::new()).await.unwrap();
        ia.create_session("u2", false, Row::new()).await.unwrap();
        assert_eq!(ia.delete_user_sessions("u1").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn verification_consume_is_atomic() {
        let ia = adapter();
        let v = ia
            .create_verification_value(row(&[
                ("identifier", "email-verify".into()),
                ("value", "token-123".into()),
                (
                    "expiresAt",
                    DbValue::DateTime(now() + time::Duration::hours(1)),
                ),
            ]))
            .await
            .unwrap();
        assert_eq!(v.identifier, "email-verify");

        let found = ia.find_verification_value("email-verify").await.unwrap();
        assert_eq!(found.unwrap().value, "token-123");

        // first consume returns it; second returns None (consumed)
        let first = ia.consume_verification_value(&v.id).await.unwrap();
        assert_eq!(first.unwrap().value, "token-123");
        assert!(
            ia.consume_verification_value(&v.id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn find_verification_returns_most_recent() {
        let ia = adapter();
        // two rows sharing an identifier; the later createdAt must win
        ia.create_verification_value(row(&[
            ("identifier", "id".into()),
            ("value", "old".into()),
            (
                "expiresAt",
                DbValue::DateTime(now() + time::Duration::hours(1)),
            ),
            (
                "createdAt",
                DbValue::DateTime(now() - time::Duration::hours(2)),
            ),
        ]))
        .await
        .unwrap();
        ia.create_verification_value(row(&[
            ("identifier", "id".into()),
            ("value", "new".into()),
            (
                "expiresAt",
                DbValue::DateTime(now() + time::Duration::hours(1)),
            ),
            (
                "createdAt",
                DbValue::DateTime(now() - time::Duration::hours(1)),
            ),
        ]))
        .await
        .unwrap();
        assert_eq!(
            ia.find_verification_value("id")
                .await
                .unwrap()
                .unwrap()
                .value,
            "new"
        );
    }

    #[tokio::test]
    async fn consume_expired_returns_none_but_deletes() {
        let ia = adapter();
        let v = ia
            .create_verification_value(row(&[
                ("identifier", "expired".into()),
                ("value", "x".into()),
                (
                    "expiresAt",
                    DbValue::DateTime(now() - time::Duration::minutes(1)),
                ),
            ]))
            .await
            .unwrap();
        // expired row: returns None even on the first consume, and the row is gone
        assert!(
            ia.consume_verification_value(&v.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            ia.find_verification_value("expired")
                .await
                .unwrap()
                .is_none()
        );
    }
}

//! Upstream reference: oauth2/oauth-provider.ts  (in progress — see note)
//!
//! The OAuth2 provider abstraction. The token/userinfo value types and the `OAuthProvider` trait
//! (the methods every social provider implements) are ported here. `ProviderOptions`' async-closure
//! *override* fields (`getUserInfo`/`refreshAccessToken`/`mapProfileToUser`/`verifyIdToken`) are
//! deferred — they need the oauth2 fetch layer — so `oauth-provider.ts` stays `building`. The
//! generic profile `T` is modeled as `serde_json::Value` (raw provider profile).

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use url::Url;

/// An error from an OAuth2 provider operation.
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    /// The provider operation failed.
    #[error("oauth2 provider error: {0}")]
    Provider(String),
}

/// Normalized OAuth2 token response (`OAuth2Tokens`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Tokens {
    /// The token type (e.g. `"bearer"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    /// The access token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// The refresh token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// When the access token expires.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub access_token_expires_at: Option<OffsetDateTime>,
    /// When the refresh token expires.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub refresh_token_expires_at: Option<OffsetDateTime>,
    /// Granted scopes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
    /// The OIDC id token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// The raw provider token response (provider-specific fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw: Option<Value>,
}

/// Normalized user info from a provider (`OAuth2UserInfo`). The upstream `id: string | number` is
/// modeled as a `String` (numeric ids are stringified).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2UserInfo {
    /// The provider's user id.
    pub id: String,
    /// Display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Avatar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Whether the email is verified.
    pub email_verified: bool,
}

/// The result of [`OAuthProvider::get_user_info`]: the normalized user plus the raw provider
/// profile (`{ user, data }`).
#[derive(Debug, Clone)]
pub struct UserInfoResult {
    /// The normalized user.
    pub user: OAuth2UserInfo,
    /// The raw provider profile.
    pub data: Value,
}

/// Parameters for [`OAuthProvider::create_authorization_url`].
#[derive(Debug, Clone, Default)]
pub struct AuthorizationUrlParams {
    /// The CSRF state.
    pub state: String,
    /// The PKCE code verifier.
    pub code_verifier: String,
    /// Requested scopes.
    pub scopes: Option<Vec<String>>,
    /// The redirect URI.
    pub redirect_uri: String,
    /// The `display` parameter.
    pub display: Option<String>,
    /// The `login_hint` parameter.
    pub login_hint: Option<String>,
}

/// Parameters for [`OAuthProvider::validate_authorization_code`].
#[derive(Debug, Clone, Default)]
pub struct ValidateCodeParams {
    /// The authorization code.
    pub code: String,
    /// The redirect URI.
    pub redirect_uri: String,
    /// The PKCE code verifier.
    pub code_verifier: Option<String>,
    /// The device id (device-flow).
    pub device_id: Option<String>,
}

/// The `select_account`/`consent`/… prompt for the authorization request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prompt {
    /// `select_account`
    SelectAccount,
    /// `consent`
    Consent,
    /// `login`
    Login,
    /// `none`
    None,
    /// `select_account consent`
    SelectAccountConsent,
}

/// The response mode for the authorization request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseMode {
    /// `query`
    Query,
    /// `form_post`
    FormPost,
}

/// Configuration for a provider (`ProviderOptions`). The async-closure override fields
/// (`getUserInfo`/`refreshAccessToken`/`mapProfileToUser`/`verifyIdToken`) are deferred until the
/// oauth2 fetch layer is ported.
#[derive(Debug, Clone, Default)]
pub struct ProviderOptions {
    /// The OAuth client id.
    pub client_id: Option<String>,
    /// The OAuth client secret.
    pub client_secret: Option<String>,
    /// Requested scopes.
    pub scope: Option<Vec<String>>,
    /// Don't add the provider's default scopes.
    pub disable_default_scope: Option<bool>,
    /// Override redirect URI.
    pub redirect_uri: Option<String>,
    /// Override the authorization endpoint.
    pub authorization_endpoint: Option<String>,
    /// Client key (used by TikTok instead of `client_id`).
    pub client_key: Option<String>,
    /// Disable id-token sign-in from the client.
    pub disable_id_token_sign_in: Option<bool>,
    /// Disable implicit sign-up.
    pub disable_implicit_sign_up: Option<bool>,
    /// Disable sign-up entirely.
    pub disable_sign_up: Option<bool>,
    /// The authorization prompt.
    pub prompt: Option<Prompt>,
    /// The authorization response mode.
    pub response_mode: Option<ResponseMode>,
    /// Override the stored user info with the provider's on each sign-in.
    pub override_user_info_on_sign_in: Option<bool>,
}

/// An OAuth2 / social provider (`OAuthProvider`). Each social provider implements this. The raw
/// provider profile is a `serde_json::Value`.
#[async_trait::async_trait]
pub trait OAuthProvider: Send + Sync {
    /// The provider id (e.g. `"google"`).
    fn id(&self) -> &str;
    /// The provider's display name.
    fn name(&self) -> &str;
    /// Provider options, if any.
    fn options(&self) -> Option<&ProviderOptions> {
        None
    }
    /// Whether implicit sign-up is disabled.
    fn disable_implicit_sign_up(&self) -> bool {
        false
    }
    /// Whether sign-up is disabled.
    fn disable_sign_up(&self) -> bool {
        false
    }

    /// Build the authorization URL to redirect the user to.
    async fn create_authorization_url(
        &self,
        params: AuthorizationUrlParams,
    ) -> Result<Url, OAuthError>;

    /// Exchange an authorization code for tokens (`None` if the exchange yields no tokens).
    async fn validate_authorization_code(
        &self,
        params: ValidateCodeParams,
    ) -> Result<Option<OAuth2Tokens>, OAuthError>;

    /// Fetch the user info for a token. `user_hint` carries the provider-supplied user object some
    /// providers (e.g. Apple) include on first sign-in.
    async fn get_user_info(
        &self,
        token: OAuth2Tokens,
        user_hint: Option<Value>,
    ) -> Result<Option<UserInfoResult>, OAuthError>;

    /// Refresh an access token. Not supported by default.
    async fn refresh_access_token(&self, _refresh_token: &str) -> Result<OAuth2Tokens, OAuthError> {
        Err(OAuthError::Provider(
            "refresh_access_token is not supported by this provider".to_owned(),
        ))
    }

    /// Revoke a token. No-op by default.
    async fn revoke_token(&self, _token: &str) -> Result<(), OAuthError> {
        Ok(())
    }

    /// Verify an id token. Returns `false` by default.
    async fn verify_id_token(
        &self,
        _token: &str,
        _nonce: Option<&str>,
    ) -> Result<bool, OAuthError> {
        Ok(false)
    }
}

/// A boxed, shareable [`OAuthProvider`].
pub type DynOAuthProvider = Arc<dyn OAuthProvider>;

#[cfg(test)]
#[path = "oauth_provider.test.rs"]
mod oauth_provider_tests;

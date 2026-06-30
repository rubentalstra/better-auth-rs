//! Aggregator for `oauth2/`. The full `oauth2/index.ts` barrel (authorization-url, validate-code,
//! refresh, verify, social-provider helpers) is ported in the oauth2 phase; for now this wires the
//! provider abstraction that `AuthContext.social_providers` holds.

pub mod oauth_provider;

pub use oauth_provider::{
    AuthorizationUrlParams, DynOAuthProvider, OAuth2Tokens, OAuth2UserInfo, OAuthError,
    OAuthProvider, Prompt, ProviderOptions, ResponseMode, UserInfoResult, ValidateCodeParams,
};

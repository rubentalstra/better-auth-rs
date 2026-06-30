//! Behavior tests for the OAuth provider abstraction.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use super::*;

#[test]
fn oauth2_tokens_serde_camel_case_and_omits_none() {
    let t: OAuth2Tokens = serde_json::from_value(serde_json::json!({
        "tokenType": "bearer",
        "accessToken": "at",
        "scopes": ["a", "b"]
    }))
    .unwrap();
    assert_eq!(t.token_type.as_deref(), Some("bearer"));
    assert_eq!(t.access_token.as_deref(), Some("at"));
    assert_eq!(t.scopes, Some(vec!["a".to_owned(), "b".to_owned()]));

    let v = serde_json::to_value(&t).unwrap();
    assert_eq!(v["accessToken"], "at");
    assert!(v.get("refreshToken").is_none()); // None omitted
    assert!(v.get("access_token").is_none()); // camelCase, not snake
}

struct Dummy;

#[async_trait::async_trait]
impl OAuthProvider for Dummy {
    fn id(&self) -> &str {
        "dummy"
    }
    fn name(&self) -> &str {
        "Dummy"
    }
    async fn create_authorization_url(
        &self,
        params: AuthorizationUrlParams,
    ) -> Result<url::Url, OAuthError> {
        url::Url::parse(&format!("https://example.com/auth?state={}", params.state))
            .map_err(|e| OAuthError::Provider(e.to_string()))
    }
    async fn validate_authorization_code(
        &self,
        _params: ValidateCodeParams,
    ) -> Result<Option<OAuth2Tokens>, OAuthError> {
        Ok(Some(OAuth2Tokens {
            access_token: Some("at".to_owned()),
            ..Default::default()
        }))
    }
    async fn get_user_info(
        &self,
        _token: OAuth2Tokens,
        _user_hint: Option<serde_json::Value>,
    ) -> Result<Option<UserInfoResult>, OAuthError> {
        Ok(None)
    }
}

#[tokio::test]
async fn defaults_and_object_safety() {
    let provider: DynOAuthProvider = Arc::new(Dummy);
    assert_eq!(provider.id(), "dummy");
    assert_eq!(provider.name(), "Dummy");
    assert!(!provider.disable_sign_up());
    assert!(!provider.disable_implicit_sign_up());
    assert!(provider.options().is_none());

    // default optional methods
    assert!(provider.refresh_access_token("rt").await.is_err());
    assert!(!provider.verify_id_token("tok", None).await.unwrap());
    assert!(provider.revoke_token("tok").await.is_ok());

    let url = provider
        .create_authorization_url(AuthorizationUrlParams {
            state: "xyz".to_owned(),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(url.query(), Some("state=xyz"));
}

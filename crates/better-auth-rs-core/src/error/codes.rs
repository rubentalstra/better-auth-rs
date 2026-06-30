//! Upstream source: error/codes.ts
//!
//! `BASE_ERROR_CODES` — a flat UPPER_SNAKE → message map built via `defineErrorCodes` — becomes a
//! closed enum: one variant per code, `code()` returns the UPPER_SNAKE key, `message()` the human
//! string (verbatim). `Display` mirrors the entries' `toString()` (which returns the key).
//!
//! The `defineErrorCodes` factory and its compile-time key-validation types
//! (`ValidateErrorCodes`/`IsValidUpperSnakeCase`/`InvalidKeyError`) have no runtime analog — the
//! enum *is* their realization. The `declare module "@better-auth/core"` plugin-registry
//! augmentation (the `$internal:base` entry) is likewise compile-time-only and drops.

use core::fmt;

/// The base authentication error codes (upstream `BASE_ERROR_CODES`). Each variant maps to a
/// stable machine-readable [`code`](Self::code) (the UPPER_SNAKE key) and a human-readable
/// [`message`](Self::message).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseErrorCode {
    /// "User not found"
    UserNotFound,
    /// "Failed to create user"
    FailedToCreateUser,
    /// "Failed to create session"
    FailedToCreateSession,
    /// "Failed to update user"
    FailedToUpdateUser,
    /// "Failed to get session"
    FailedToGetSession,
    /// "Invalid password"
    InvalidPassword,
    /// "Invalid email"
    InvalidEmail,
    /// "Invalid email or password"
    InvalidEmailOrPassword,
    /// "Invalid user"
    InvalidUser,
    /// "Social account already linked"
    SocialAccountAlreadyLinked,
    /// "Provider not found"
    ProviderNotFound,
    /// "Invalid token"
    InvalidToken,
    /// "Token expired"
    TokenExpired,
    /// "id_token not supported"
    IdTokenNotSupported,
    /// "Failed to get user info"
    FailedToGetUserInfo,
    /// "User email not found"
    UserEmailNotFound,
    /// "Email not verified"
    EmailNotVerified,
    /// "Password too short"
    PasswordTooShort,
    /// "Password too long"
    PasswordTooLong,
    /// "User already exists."
    UserAlreadyExists,
    /// "User already exists. Use another email."
    UserAlreadyExistsUseAnotherEmail,
    /// "Email can not be updated"
    EmailCanNotBeUpdated,
    /// "Change email is disabled"
    ChangeEmailDisabled,
    /// "Credential account not found"
    CredentialAccountNotFound,
    /// "Session expired. Re-authenticate to perform this action."
    SessionExpired,
    /// "You can't unlink your last account"
    FailedToUnlinkLastAccount,
    /// "Account not found"
    AccountNotFound,
    /// "User already has a password. Provide that to delete the account."
    UserAlreadyHasPassword,
    /// "Cross-site navigation login blocked. This request appears to be a CSRF attack."
    CrossSiteNavigationLoginBlocked,
    /// "Verification email isn't enabled"
    VerificationEmailNotEnabled,
    /// "Email is already verified"
    EmailAlreadyVerified,
    /// "Email mismatch"
    EmailMismatch,
    /// "Session is not fresh"
    SessionNotFresh,
    /// "Linked account already exists"
    LinkedAccountAlreadyExists,
    /// "Invalid origin"
    InvalidOrigin,
    /// "Invalid callbackURL"
    InvalidCallbackUrl,
    /// "Invalid redirectURL"
    InvalidRedirectUrl,
    /// "Invalid errorCallbackURL"
    InvalidErrorCallbackUrl,
    /// "Invalid newUserCallbackURL"
    InvalidNewUserCallbackUrl,
    /// "Missing or null Origin"
    MissingOrNullOrigin,
    /// "callbackURL is required"
    CallbackUrlRequired,
    /// "Unable to create verification"
    FailedToCreateVerification,
    /// "Field not allowed to be set"
    FieldNotAllowed,
    /// "Async validation is not supported"
    AsyncValidationNotSupported,
    /// "Validation Error"
    ValidationError,
    /// "Field is required"
    MissingField,
    /// "POST method requires deferSessionRefresh to be enabled in session config"
    MethodNotAllowedDeferSessionRequired,
    /// "Body must be an object"
    BodyMustBeAnObject,
    /// "User already has a password set"
    PasswordAlreadySet,
}

impl BaseErrorCode {
    /// The machine-readable code — the upstream UPPER_SNAKE key.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::FailedToCreateUser => "FAILED_TO_CREATE_USER",
            Self::FailedToCreateSession => "FAILED_TO_CREATE_SESSION",
            Self::FailedToUpdateUser => "FAILED_TO_UPDATE_USER",
            Self::FailedToGetSession => "FAILED_TO_GET_SESSION",
            Self::InvalidPassword => "INVALID_PASSWORD",
            Self::InvalidEmail => "INVALID_EMAIL",
            Self::InvalidEmailOrPassword => "INVALID_EMAIL_OR_PASSWORD",
            Self::InvalidUser => "INVALID_USER",
            Self::SocialAccountAlreadyLinked => "SOCIAL_ACCOUNT_ALREADY_LINKED",
            Self::ProviderNotFound => "PROVIDER_NOT_FOUND",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::IdTokenNotSupported => "ID_TOKEN_NOT_SUPPORTED",
            Self::FailedToGetUserInfo => "FAILED_TO_GET_USER_INFO",
            Self::UserEmailNotFound => "USER_EMAIL_NOT_FOUND",
            Self::EmailNotVerified => "EMAIL_NOT_VERIFIED",
            Self::PasswordTooShort => "PASSWORD_TOO_SHORT",
            Self::PasswordTooLong => "PASSWORD_TOO_LONG",
            Self::UserAlreadyExists => "USER_ALREADY_EXISTS",
            Self::UserAlreadyExistsUseAnotherEmail => "USER_ALREADY_EXISTS_USE_ANOTHER_EMAIL",
            Self::EmailCanNotBeUpdated => "EMAIL_CAN_NOT_BE_UPDATED",
            Self::ChangeEmailDisabled => "CHANGE_EMAIL_DISABLED",
            Self::CredentialAccountNotFound => "CREDENTIAL_ACCOUNT_NOT_FOUND",
            Self::SessionExpired => "SESSION_EXPIRED",
            Self::FailedToUnlinkLastAccount => "FAILED_TO_UNLINK_LAST_ACCOUNT",
            Self::AccountNotFound => "ACCOUNT_NOT_FOUND",
            Self::UserAlreadyHasPassword => "USER_ALREADY_HAS_PASSWORD",
            Self::CrossSiteNavigationLoginBlocked => "CROSS_SITE_NAVIGATION_LOGIN_BLOCKED",
            Self::VerificationEmailNotEnabled => "VERIFICATION_EMAIL_NOT_ENABLED",
            Self::EmailAlreadyVerified => "EMAIL_ALREADY_VERIFIED",
            Self::EmailMismatch => "EMAIL_MISMATCH",
            Self::SessionNotFresh => "SESSION_NOT_FRESH",
            Self::LinkedAccountAlreadyExists => "LINKED_ACCOUNT_ALREADY_EXISTS",
            Self::InvalidOrigin => "INVALID_ORIGIN",
            Self::InvalidCallbackUrl => "INVALID_CALLBACK_URL",
            Self::InvalidRedirectUrl => "INVALID_REDIRECT_URL",
            Self::InvalidErrorCallbackUrl => "INVALID_ERROR_CALLBACK_URL",
            Self::InvalidNewUserCallbackUrl => "INVALID_NEW_USER_CALLBACK_URL",
            Self::MissingOrNullOrigin => "MISSING_OR_NULL_ORIGIN",
            Self::CallbackUrlRequired => "CALLBACK_URL_REQUIRED",
            Self::FailedToCreateVerification => "FAILED_TO_CREATE_VERIFICATION",
            Self::FieldNotAllowed => "FIELD_NOT_ALLOWED",
            Self::AsyncValidationNotSupported => "ASYNC_VALIDATION_NOT_SUPPORTED",
            Self::ValidationError => "VALIDATION_ERROR",
            Self::MissingField => "MISSING_FIELD",
            Self::MethodNotAllowedDeferSessionRequired => {
                "METHOD_NOT_ALLOWED_DEFER_SESSION_REQUIRED"
            }
            Self::BodyMustBeAnObject => "BODY_MUST_BE_AN_OBJECT",
            Self::PasswordAlreadySet => "PASSWORD_ALREADY_SET",
        }
    }

    /// The human-readable default message (verbatim from upstream).
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::UserNotFound => "User not found",
            Self::FailedToCreateUser => "Failed to create user",
            Self::FailedToCreateSession => "Failed to create session",
            Self::FailedToUpdateUser => "Failed to update user",
            Self::FailedToGetSession => "Failed to get session",
            Self::InvalidPassword => "Invalid password",
            Self::InvalidEmail => "Invalid email",
            Self::InvalidEmailOrPassword => "Invalid email or password",
            Self::InvalidUser => "Invalid user",
            Self::SocialAccountAlreadyLinked => "Social account already linked",
            Self::ProviderNotFound => "Provider not found",
            Self::InvalidToken => "Invalid token",
            Self::TokenExpired => "Token expired",
            Self::IdTokenNotSupported => "id_token not supported",
            Self::FailedToGetUserInfo => "Failed to get user info",
            Self::UserEmailNotFound => "User email not found",
            Self::EmailNotVerified => "Email not verified",
            Self::PasswordTooShort => "Password too short",
            Self::PasswordTooLong => "Password too long",
            Self::UserAlreadyExists => "User already exists.",
            Self::UserAlreadyExistsUseAnotherEmail => "User already exists. Use another email.",
            Self::EmailCanNotBeUpdated => "Email can not be updated",
            Self::ChangeEmailDisabled => "Change email is disabled",
            Self::CredentialAccountNotFound => "Credential account not found",
            Self::SessionExpired => "Session expired. Re-authenticate to perform this action.",
            Self::FailedToUnlinkLastAccount => "You can't unlink your last account",
            Self::AccountNotFound => "Account not found",
            Self::UserAlreadyHasPassword => {
                "User already has a password. Provide that to delete the account."
            }
            Self::CrossSiteNavigationLoginBlocked => {
                "Cross-site navigation login blocked. This request appears to be a CSRF attack."
            }
            Self::VerificationEmailNotEnabled => "Verification email isn't enabled",
            Self::EmailAlreadyVerified => "Email is already verified",
            Self::EmailMismatch => "Email mismatch",
            Self::SessionNotFresh => "Session is not fresh",
            Self::LinkedAccountAlreadyExists => "Linked account already exists",
            Self::InvalidOrigin => "Invalid origin",
            Self::InvalidCallbackUrl => "Invalid callbackURL",
            Self::InvalidRedirectUrl => "Invalid redirectURL",
            Self::InvalidErrorCallbackUrl => "Invalid errorCallbackURL",
            Self::InvalidNewUserCallbackUrl => "Invalid newUserCallbackURL",
            Self::MissingOrNullOrigin => "Missing or null Origin",
            Self::CallbackUrlRequired => "callbackURL is required",
            Self::FailedToCreateVerification => "Unable to create verification",
            Self::FieldNotAllowed => "Field not allowed to be set",
            Self::AsyncValidationNotSupported => "Async validation is not supported",
            Self::ValidationError => "Validation Error",
            Self::MissingField => "Field is required",
            Self::MethodNotAllowedDeferSessionRequired => {
                "POST method requires deferSessionRefresh to be enabled in session config"
            }
            Self::BodyMustBeAnObject => "Body must be an object",
            Self::PasswordAlreadySet => "User already has a password set",
        }
    }
}

/// Mirrors the entries' `toString()` in `defineErrorCodes`, which returns the code (the key).
impl fmt::Display for BaseErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// `APIErrorCode = keyof typeof BASE_ERROR_CODES` — the set of base error-code keys. In Rust the
/// enum itself is that set.
pub type ApiErrorCode = BaseErrorCode;

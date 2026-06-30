//! Upstream source: types/cookie.ts
//!
//! `CookieOptions` is from `better-call` (modeled locally — core is driver-light, no `cookie`
//! crate dep). The string-literal unions for `sameSite`/`prefix` become closed enums; `sameSite`'s
//! upstream union accepts either case (`"Strict"`/`"strict"` …) but the value is the same, so the
//! enum carries the value only. `expires?: Date` → `Option<time::OffsetDateTime>`; `maxAge` is in
//! seconds.

use time::OffsetDateTime;

/// The `SameSite` cookie attribute. Upstream accepts both `"Strict"`/`"strict"` (etc.); the value
/// is case-insensitive so only the three semantic options are modeled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    /// `Strict`
    Strict,
    /// `Lax`
    Lax,
    /// `None`
    None,
}

/// The cookie name prefix (`__Host-` / `__Secure-`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookiePrefix {
    /// `__Host-` prefix.
    Host,
    /// `__Secure-` prefix.
    Secure,
}

/// Cookie serialization options (port of `better-call`'s `CookieOptions`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookieOptions {
    /// `domain`
    pub domain: Option<String>,
    /// `expires`
    pub expires: Option<OffsetDateTime>,
    /// `httpOnly`
    pub http_only: Option<bool>,
    /// `maxAge`, in seconds.
    pub max_age: Option<i64>,
    /// `path`
    pub path: Option<String>,
    /// `secure`
    pub secure: Option<bool>,
    /// `sameSite`
    pub same_site: Option<SameSite>,
    /// `partitioned`
    pub partitioned: Option<bool>,
    /// `prefix`
    pub prefix: Option<CookiePrefix>,
}

/// A named cookie plus its serialization attributes (`{ name, attributes }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetterAuthCookie {
    /// The cookie name.
    pub name: String,
    /// The cookie attributes.
    pub attributes: CookieOptions,
}

/// The set of cookies better-auth manages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetterAuthCookies {
    /// The session-token cookie.
    pub session_token: BetterAuthCookie,
    /// The session-data (cookie-cache) cookie.
    pub session_data: BetterAuthCookie,
    /// The account-data cookie.
    pub account_data: BetterAuthCookie,
    /// The "don't remember me" token cookie.
    pub dont_remember_token: BetterAuthCookie,
}

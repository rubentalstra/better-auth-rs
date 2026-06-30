//! Upstream source: types/cookie.ts
//!
//! `CookieOptions` is `better-call`'s cookie type. Rather than hand-roll it, the fields are modeled
//! on standard crates — `cookie::SameSite` for `sameSite`, `time` for `expires`/`maxAge`. `prefix`
//! has no `cookie`-crate analog (it is better-call's `__Host-`/`__Secure-` naming convention), so it
//! stays a small local enum.

use cookie::SameSite;
use time::{Duration, OffsetDateTime};

/// The cookie name prefix (`__Host-` / `__Secure-`). better-call-specific; no `cookie`-crate analog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CookiePrefix {
    /// `__Host-` prefix.
    Host,
    /// `__Secure-` prefix.
    Secure,
}

/// Cookie serialization options (port of `better-call`'s `CookieOptions`), modeled on the `cookie`
/// and `time` crates.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CookieOptions {
    /// `domain`
    pub domain: Option<String>,
    /// `expires`
    pub expires: Option<OffsetDateTime>,
    /// `httpOnly`
    pub http_only: Option<bool>,
    /// `maxAge` (upstream: a number of seconds).
    pub max_age: Option<Duration>,
    /// `path`
    pub path: Option<String>,
    /// `secure`
    pub secure: Option<bool>,
    /// `sameSite` (`cookie::SameSite`; upstream's union accepts either case, same value).
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

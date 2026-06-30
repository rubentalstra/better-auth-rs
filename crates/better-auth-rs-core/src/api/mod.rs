//! Upstream reference: api/index.ts (the anchor types only)
//!
//! The framework-neutral api anchors the recursive hub needs: [`AuthMiddleware`], [`Endpoint`], and
//! [`Middleware`], plus the [`HookEndpointContext`] they operate over. Core defines only the
//! contract — the published crate's api layer (`crates/better-auth-rs/src/api`) builds concrete,
//! routable endpoints on these and owns routing/OpenAPI.
//!
//! Deviations from the TS: `better-call`'s `createEndpoint`/`createMiddleware` machinery has no Rust
//! analog; we model a middleware/handler as a boxed async closure over a mutable
//! [`HookEndpointContext`]. The closure returns a future borrowing the context (a higher-ranked
//! lifetime), so a middleware may hold the context across `.await`. `HookEndpointContext` flattens
//! the TS `context.returned` / `context.responseHeaders` up to its own fields for ergonomics; the
//! `EndpointContext & InputContext` request data is concrete optional fields (no TS `Partial<>`).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use http::{HeaderMap, Method};
use serde_json::Value;

use crate::error::APIError;
use crate::types::context::AuthContext;

/// The request-scoped context handed to every hook/middleware (`HookEndpointContext`).
#[derive(Debug)]
pub struct HookEndpointContext {
    /// The matched path (`/sign-in/email`, …); `None` for path-less server-only endpoints.
    pub path: Option<String>,
    /// Parsed request body (opaque JSON at the anchor layer; the api layer narrows per endpoint).
    pub body: Option<Value>,
    /// Parsed query parameters.
    pub query: Option<Value>,
    /// Parsed path parameters.
    pub params: Option<Value>,
    /// Request headers.
    pub headers: HeaderMap,
    /// The shared application context.
    pub context: Arc<AuthContext>,
    /// What a prior stage / the endpoint returned (post-hooks only).
    pub returned: Option<Value>,
    /// Headers to attach to the response (e.g. `Set-Cookie`).
    pub response_headers: HeaderMap,
}

impl HookEndpointContext {
    /// Build a hook context for `context` with empty request data.
    #[must_use]
    pub fn new(context: Arc<AuthContext>) -> Self {
        Self {
            path: None,
            body: None,
            query: None,
            params: None,
            headers: HeaderMap::new(),
            context,
            returned: None,
            response_headers: HeaderMap::new(),
        }
    }
}

/// What a hook/middleware decides (`{ context? } | void`, plus short-circuit).
#[derive(Debug)]
pub enum MiddlewareOutcome {
    /// Continue the pipeline (the `void` case).
    Continue,
    /// Short-circuit the pipeline with a response value (serialized by the api layer).
    Respond(Value),
}

/// The future returned by an [`AuthMiddleware`] / [`EndpointHandler`]. Borrows the
/// [`HookEndpointContext`] for `'a` so the body may hold it across `.await`.
pub type MiddlewareFuture<'a> =
    Pin<Box<dyn Future<Output = Result<MiddlewareOutcome, APIError>> + Send + 'a>>;

/// A hook/middleware handler (`AuthMiddleware` in `api/index.ts`): a boxed async closure over the
/// mutable [`HookEndpointContext`]. Plugins build these with
/// `Arc::new(|ctx| Box::pin(async move { … }))`. Returns `Err(APIError)` to fail closed.
pub type AuthMiddleware =
    Arc<dyn for<'a> Fn(&'a mut HookEndpointContext) -> MiddlewareFuture<'a> + Send + Sync>;

/// An endpoint's handler — the same shape as an [`AuthMiddleware`] (one context type across the
/// pipeline).
pub type EndpointHandler = AuthMiddleware;

/// Metadata describing an [`Endpoint`].
#[derive(Debug, Clone)]
pub struct EndpointMeta {
    /// The HTTP methods the endpoint accepts.
    pub methods: Vec<Method>,
    /// Server-only endpoints (`metadata.SERVER_ONLY`) are never routed or emitted to OpenAPI.
    pub server_only: bool,
    /// Opaque per-endpoint metadata (`$Infer`/openapi); JSON at the anchor layer.
    pub metadata: Value,
}

/// A registered endpoint (`Endpoint`). The published crate's api layer constructs these; core only
/// carries them.
pub struct Endpoint {
    /// The mount path; `None` means a server-only / path-less endpoint.
    pub path: Option<String>,
    /// Endpoint metadata.
    pub meta: EndpointMeta,
    /// The endpoint handler.
    pub handler: EndpointHandler,
}

impl core::fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Endpoint")
            .field("path", &self.path)
            .field("meta", &self.meta)
            .field("handler", &"<fn>")
            .finish()
    }
}

/// A path-scoped middleware (`{ path, middleware }`).
pub struct Middleware {
    /// The path the middleware applies to.
    pub path: String,
    /// The middleware handler.
    pub middleware: AuthMiddleware,
}

impl core::fmt::Debug for Middleware {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Middleware")
            .field("path", &self.path)
            .field("middleware", &"<fn>")
            .finish()
    }
}

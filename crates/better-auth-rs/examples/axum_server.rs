//! Minimal axum server example (placeholder).
//!
//! This will wire `better-auth-rs` (email/password + sessions, `sqlx-postgres`) into an
//! `axum` router via `Router::nest_service("/api/auth", auth_service)` once the `axum`
//! integration lands in Phase 4. For now it just prints the tracked versions so the
//! example crate compiles and is the smoke target for the differential harness.
//!
//! Run with: `cargo run -p better-auth-rs --example axum_server`

fn main() {
    println!(
        "better-auth-rs v{} (tracking better-auth v{})",
        better_auth_rs::VERSION,
        better_auth_rs::UPSTREAM_VERSION,
    );
    println!(
        "axum integration arrives in Phase 4 — see .claude/phases/phase-4-axum-integration.md"
    );
}

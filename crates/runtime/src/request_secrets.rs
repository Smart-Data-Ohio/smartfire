//! The tests' fixed rendering entropy. The retained pages that rendered tokens and a CSP nonce
//! are gone, so the scope no longer fixes anything; the test harness re-exports it.

/// Run `request` as the tests' renders expect. No HTML inputs or response rewrites;
/// authentication and forgery verification keep their real tokens.
pub async fn with_fixed_render_secrets<T>(request: impl std::future::Future<Output = T>) -> T {
    request.await
}

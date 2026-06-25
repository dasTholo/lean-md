//! Test-only helpers for mutating the process environment.
//!
//! `std::env::set_var` / `std::env::remove_var` became `unsafe` in Rust 2024
//! because they are not thread-safe: a concurrent environment read from another
//! thread is undefined behaviour. Centralising the `unsafe` here documents the
//! invariant once instead of at every call site. Vendored from lean-ctx
//! (`src/test_env.rs`) as part of the standalone-crate decoupling (Task 6).

#![cfg(test)]

use std::ffi::OsStr;

/// Sets `key` to `value` in the process environment (test-only).
pub(crate) fn set_var<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    // SAFETY: env mutations in lean-md tests are not run concurrently with
    // environment reads from other threads (single-threaded test bodies).
    unsafe { std::env::set_var(key, value) };
}

/// Removes `key` from the process environment (test-only).
#[allow(dead_code)]
pub(crate) fn remove_var<K: AsRef<OsStr>>(key: K) {
    // SAFETY: see `set_var`.
    unsafe { std::env::remove_var(key) };
}

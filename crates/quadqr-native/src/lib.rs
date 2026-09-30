//! Android/native bindings for QuadQR.
//!
//! This crate produces one `libquadqr.so` per Android ABI and exposes both:
//! - a C ABI suitable for Dart/Flutter FFI;
//! - JNI entry points suitable for Java/Kotlin.
//!
//! The actual codec/scanner remains in `crates/quadqr`.

mod api;
mod ffi;
mod jni_bridge;

pub use api::*;

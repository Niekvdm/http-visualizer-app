//! Infrastructure layer providing abstractions for external dependencies.
//!
//! This module contains traits and implementations for:
//! - DNS resolution
//! - TLS/SSL connections
//! - Content decompression
//!
//! These abstractions enable dependency injection, easier testing, and
//! the ability to swap implementations without modifying core business logic.

pub mod decompressor;
pub mod dns;
pub mod tls;

pub use decompressor::{decompress_body, Decompressor, MultiDecompressor};
pub use dns::{DnsResolver, HickoryDnsResolver};
pub use tls::{create_tls_config, RustlsTlsProvider, TlsProvider};

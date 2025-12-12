//! Shared utilities used across the HTTP visualizer application.
//!
//! This module contains common functionality extracted from various parts
//! of the codebase to eliminate code duplication and improve maintainability.

pub mod cert_parser;
pub mod status_text;
pub mod timing;

pub use cert_parser::{parse_x509_basic, BasicCertInfo, CapturedCertInfo};
pub use status_text::status_text;
pub use timing::DetailedTiming;

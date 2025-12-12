//! Timing utilities for HTTP request measurements.
//!
//! Provides detailed timing tracking for various phases of an HTTP request.

use crate::proxy::types::TimingInfo;
use std::time::Instant;

/// Detailed timing measurements for HTTP request phases.
///
/// Tracks the start and end time of each phase of an HTTP request:
/// - DNS resolution
/// - TCP connection establishment
/// - TLS handshake (for HTTPS)
/// - Request sending
/// - Time to first byte (TTFB)
/// - Content download
#[derive(Debug)]
pub struct DetailedTiming {
    pub dns_start: Option<Instant>,
    pub dns_end: Option<Instant>,
    pub tcp_start: Option<Instant>,
    pub tcp_end: Option<Instant>,
    pub tls_start: Option<Instant>,
    pub tls_end: Option<Instant>,
    pub request_start: Option<Instant>,
    pub ttfb: Option<Instant>,
    pub download_start: Option<Instant>,
    pub download_end: Option<Instant>,
    pub total_start: Instant,
}

impl DetailedTiming {
    /// Creates a new `DetailedTiming` instance with the total timer started.
    pub fn new() -> Self {
        Self {
            dns_start: None,
            dns_end: None,
            tcp_start: None,
            tcp_end: None,
            tls_start: None,
            tls_end: None,
            request_start: None,
            ttfb: None,
            download_start: None,
            download_end: None,
            total_start: Instant::now(),
        }
    }

    /// Converts the detailed timing measurements into a `TimingInfo` struct
    /// suitable for serialization and API responses.
    pub fn to_timing_info(&self) -> TimingInfo {
        let total = self
            .download_end
            .unwrap_or_else(Instant::now)
            .duration_since(self.total_start)
            .as_millis() as u64;

        let dns = match (self.dns_start, self.dns_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let tcp = match (self.tcp_start, self.tcp_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let tls = match (self.tls_start, self.tls_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let ttfb = match (self.request_start, self.ttfb) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        let download = match (self.download_start, self.download_end) {
            (Some(s), Some(e)) => Some(e.duration_since(s).as_millis() as u64),
            _ => None,
        };

        TimingInfo {
            total,
            dns,
            tcp,
            tls,
            ttfb,
            download,
            blocked: Some(0),
        }
    }

    /// Starts the DNS timing phase.
    pub fn start_dns(&mut self) {
        self.dns_start = Some(Instant::now());
    }

    /// Ends the DNS timing phase.
    pub fn end_dns(&mut self) {
        self.dns_end = Some(Instant::now());
    }

    /// Starts the TCP connection timing phase.
    pub fn start_tcp(&mut self) {
        self.tcp_start = Some(Instant::now());
    }

    /// Ends the TCP connection timing phase.
    pub fn end_tcp(&mut self) {
        self.tcp_end = Some(Instant::now());
    }

    /// Starts the TLS handshake timing phase.
    pub fn start_tls(&mut self) {
        self.tls_start = Some(Instant::now());
    }

    /// Ends the TLS handshake timing phase.
    pub fn end_tls(&mut self) {
        self.tls_end = Some(Instant::now());
    }

    /// Marks the start of sending the request.
    pub fn start_request(&mut self) {
        self.request_start = Some(Instant::now());
    }

    /// Marks the time to first byte (TTFB).
    pub fn mark_ttfb(&mut self) {
        self.ttfb = Some(Instant::now());
    }

    /// Starts the download timing phase.
    pub fn start_download(&mut self) {
        self.download_start = Some(Instant::now());
    }

    /// Ends the download timing phase.
    pub fn end_download(&mut self) {
        self.download_end = Some(Instant::now());
    }
}

impl Default for DetailedTiming {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_new_timing() {
        let timing = DetailedTiming::new();
        assert!(timing.dns_start.is_none());
        assert!(timing.dns_end.is_none());
    }

    #[test]
    fn test_timing_phases() {
        let mut timing = DetailedTiming::new();

        timing.start_dns();
        sleep(Duration::from_millis(1));
        timing.end_dns();

        timing.start_tcp();
        sleep(Duration::from_millis(1));
        timing.end_tcp();

        timing.start_download();
        sleep(Duration::from_millis(1));
        timing.end_download();

        let info = timing.to_timing_info();
        assert!(info.dns.is_some());
        assert!(info.tcp.is_some());
        assert!(info.download.is_some());
        assert!(info.total >= 3);
    }
}

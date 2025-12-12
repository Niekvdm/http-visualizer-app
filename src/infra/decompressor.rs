//! Content decompression infrastructure.
//!
//! Provides trait-based abstractions for HTTP response body decompression,
//! supporting gzip, deflate, and brotli encodings.

use std::io::Read;

/// Result of a decompression operation.
#[derive(Debug)]
pub struct DecompressResult {
    /// The decompressed data.
    pub data: Vec<u8>,
    /// Original compressed size.
    pub compressed_size: usize,
    /// Decompressed size.
    pub decompressed_size: usize,
}

/// Trait for content decompression.
///
/// This abstraction allows for different decompression implementations
/// and makes testing easier.
pub trait Decompressor: Send + Sync {
    /// The content-encoding this decompressor handles (e.g., "gzip", "deflate", "br").
    fn encoding(&self) -> &'static str;

    /// Decompresses the given data.
    ///
    /// # Arguments
    ///
    /// * `data` - The compressed data
    ///
    /// # Returns
    ///
    /// A `Result` containing `DecompressResult` on success, or an error message on failure.
    fn decompress(&self, data: &[u8]) -> Result<DecompressResult, String>;
}

/// Gzip decompressor implementation.
#[derive(Default)]
pub struct GzipDecompressor;

impl Decompressor for GzipDecompressor {
    fn encoding(&self) -> &'static str {
        "gzip"
    }

    fn decompress(&self, data: &[u8]) -> Result<DecompressResult, String> {
        let compressed_size = data.len();
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("Gzip decompression failed: {}", e))?;
        Ok(DecompressResult {
            decompressed_size: decompressed.len(),
            data: decompressed,
            compressed_size,
        })
    }
}

/// Deflate decompressor implementation.
#[derive(Default)]
pub struct DeflateDecompressor;

impl Decompressor for DeflateDecompressor {
    fn encoding(&self) -> &'static str {
        "deflate"
    }

    fn decompress(&self, data: &[u8]) -> Result<DecompressResult, String> {
        let compressed_size = data.len();
        let mut decoder = flate2::read::DeflateDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("Deflate decompression failed: {}", e))?;
        Ok(DecompressResult {
            decompressed_size: decompressed.len(),
            data: decompressed,
            compressed_size,
        })
    }
}

/// Brotli decompressor implementation.
#[derive(Default)]
pub struct BrotliDecompressor;

impl Decompressor for BrotliDecompressor {
    fn encoding(&self) -> &'static str {
        "br"
    }

    fn decompress(&self, data: &[u8]) -> Result<DecompressResult, String> {
        let compressed_size = data.len();
        let mut decompressed = Vec::new();
        brotli::BrotliDecompress(&mut std::io::Cursor::new(data), &mut decompressed)
            .map_err(|e| format!("Brotli decompression failed: {}", e))?;
        Ok(DecompressResult {
            decompressed_size: decompressed.len(),
            data: decompressed,
            compressed_size,
        })
    }
}

/// Multi-format decompressor that selects the appropriate algorithm based on encoding.
#[derive(Default)]
pub struct MultiDecompressor {
    gzip: GzipDecompressor,
    deflate: DeflateDecompressor,
    brotli: BrotliDecompressor,
}

impl MultiDecompressor {
    /// Creates a new `MultiDecompressor` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decompresses data based on the content-encoding header.
    ///
    /// # Arguments
    ///
    /// * `data` - The potentially compressed data
    /// * `encoding` - The content-encoding header value (e.g., "gzip", "deflate", "br")
    ///
    /// # Returns
    ///
    /// A `Result` containing `DecompressResult` on success, or an error message on failure.
    /// If encoding is `None` or unrecognized, returns the data as-is.
    pub fn decompress(&self, data: &[u8], encoding: Option<&str>) -> Result<DecompressResult, String> {
        match encoding {
            Some("gzip") => self.gzip.decompress(data),
            Some("deflate") => self.deflate.decompress(data),
            Some("br") => self.brotli.decompress(data),
            _ => Ok(DecompressResult {
                compressed_size: data.len(),
                decompressed_size: data.len(),
                data: data.to_vec(),
            }),
        }
    }
}

/// Convenience function for decompressing body data.
///
/// # Arguments
///
/// * `body` - The potentially compressed body data
/// * `encoding` - The content-encoding header value
///
/// # Returns
///
/// A `Result` containing the decompressed data on success, or an error message on failure.
pub fn decompress_body(body: &[u8], encoding: Option<&str>) -> Result<Vec<u8>, String> {
    MultiDecompressor::new()
        .decompress(body, encoding)
        .map(|r| r.data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_gzip_decompression() {
        // Create gzip compressed data
        let original = b"Hello, World!";
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressor = GzipDecompressor;
        let result = decompressor.decompress(&compressed).unwrap();
        assert_eq!(result.data, original);
    }

    #[test]
    fn test_deflate_decompression() {
        // Create deflate compressed data
        let original = b"Hello, World!";
        let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decompressor = DeflateDecompressor;
        let result = decompressor.decompress(&compressed).unwrap();
        assert_eq!(result.data, original);
    }

    #[test]
    fn test_no_encoding_passthrough() {
        let data = b"Hello, World!";
        let result = decompress_body(data, None).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_unknown_encoding_passthrough() {
        let data = b"Hello, World!";
        let result = decompress_body(data, Some("unknown")).unwrap();
        assert_eq!(result, data);
    }
}

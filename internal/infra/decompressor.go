package infra

import (
	"bytes"
	"compress/flate"
	"compress/gzip"
	"io"

	"github.com/andybalholm/brotli"
)

// DecompressResult contains the result of a decompression operation.
type DecompressResult struct {
	Data             []byte
	CompressedSize   int
	DecompressedSize int
}

// Decompress decompresses data based on the content-encoding.
func Decompress(data []byte, encoding string) (*DecompressResult, error) {
	switch encoding {
	case "gzip":
		return decompressGzip(data)
	case "deflate":
		return decompressDeflate(data)
	case "br":
		return decompressBrotli(data)
	default:
		// No compression or unknown encoding - return as-is
		return &DecompressResult{
			Data:             data,
			CompressedSize:   len(data),
			DecompressedSize: len(data),
		}, nil
	}
}

func decompressGzip(data []byte) (*DecompressResult, error) {
	reader, err := gzip.NewReader(bytes.NewReader(data))
	if err != nil {
		return nil, err
	}
	defer reader.Close()

	decompressed, err := io.ReadAll(reader)
	if err != nil {
		return nil, err
	}

	return &DecompressResult{
		Data:             decompressed,
		CompressedSize:   len(data),
		DecompressedSize: len(decompressed),
	}, nil
}

func decompressDeflate(data []byte) (*DecompressResult, error) {
	reader := flate.NewReader(bytes.NewReader(data))
	defer reader.Close()

	decompressed, err := io.ReadAll(reader)
	if err != nil {
		return nil, err
	}

	return &DecompressResult{
		Data:             decompressed,
		CompressedSize:   len(data),
		DecompressedSize: len(decompressed),
	}, nil
}

func decompressBrotli(data []byte) (*DecompressResult, error) {
	reader := brotli.NewReader(bytes.NewReader(data))
	decompressed, err := io.ReadAll(reader)
	if err != nil {
		return nil, err
	}

	return &DecompressResult{
		Data:             decompressed,
		CompressedSize:   len(data),
		DecompressedSize: len(decompressed),
	}, nil
}

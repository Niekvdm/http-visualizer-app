// Package infra provides infrastructure components for HTTP proxy operations.
package infra

import (
	"context"
	"net"
	"time"
)

// DNSResult contains DNS resolution results and timing information.
type DNSResult struct {
	IPs        []net.IP
	DurationMs uint64
}

// ResolveDNS resolves a hostname to IP addresses with timing.
func ResolveDNS(ctx context.Context, host string) (*DNSResult, error) {
	start := time.Now()

	// Check if already an IP address
	if ip := net.ParseIP(host); ip != nil {
		return &DNSResult{
			IPs:        []net.IP{ip},
			DurationMs: 0,
		}, nil
	}

	// Perform DNS lookup
	ips, err := net.DefaultResolver.LookupIP(ctx, "ip", host)
	if err != nil {
		return nil, err
	}

	if len(ips) == 0 {
		return nil, &net.DNSError{
			Err:  "no addresses found",
			Name: host,
		}
	}

	return &DNSResult{
		IPs:        ips,
		DurationMs: uint64(time.Since(start).Milliseconds()),
	}, nil
}

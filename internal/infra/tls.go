package infra

import (
	"crypto/tls"
	"crypto/x509"
	"time"
)

// CertInfo contains TLS certificate information.
type CertInfo struct {
	Protocol  string
	Cipher    string
	Issuer    string
	Subject   string
	ValidFrom uint64
	ValidTo   uint64
}

// ExtractCertInfo extracts certificate information from a TLS connection state.
func ExtractCertInfo(state *tls.ConnectionState) *CertInfo {
	if state == nil {
		return nil
	}

	info := &CertInfo{
		Protocol: tlsVersionString(state.Version),
		Cipher:   tls.CipherSuiteName(state.CipherSuite),
	}

	// Get peer certificate
	if len(state.PeerCertificates) > 0 {
		cert := state.PeerCertificates[0]
		info.Subject = extractCN(cert.Subject.String())
		info.Issuer = extractCN(cert.Issuer.String())
		info.ValidFrom = uint64(cert.NotBefore.Unix())
		info.ValidTo = uint64(cert.NotAfter.Unix())
	}

	return info
}

// IsCertValid checks if a certificate is currently valid.
func IsCertValid(validFrom, validTo uint64) bool {
	now := uint64(time.Now().Unix())
	return now >= validFrom && now <= validTo
}

// ParseCertificate parses a DER-encoded X.509 certificate.
func ParseCertificate(der []byte) (*CertInfo, error) {
	cert, err := x509.ParseCertificate(der)
	if err != nil {
		return nil, err
	}

	return &CertInfo{
		Subject:   extractCN(cert.Subject.String()),
		Issuer:    extractCN(cert.Issuer.String()),
		ValidFrom: uint64(cert.NotBefore.Unix()),
		ValidTo:   uint64(cert.NotAfter.Unix()),
	}, nil
}

// tlsVersionString returns a human-readable TLS version string.
func tlsVersionString(version uint16) string {
	switch version {
	case tls.VersionTLS10:
		return "TLS 1.0"
	case tls.VersionTLS11:
		return "TLS 1.1"
	case tls.VersionTLS12:
		return "TLS 1.2"
	case tls.VersionTLS13:
		return "TLS 1.3"
	default:
		return "TLS"
	}
}

// extractCN attempts to extract CN from a certificate subject/issuer string.
func extractCN(dn string) string {
	// Simple extraction - the full DN is returned if CN extraction fails
	// Go's x509 already provides a nice string representation
	return dn
}

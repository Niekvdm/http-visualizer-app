package proxy

import "time"

// DetailedTiming tracks timing measurements for HTTP request phases.
type DetailedTiming struct {
	DNSStart      *time.Time
	DNSEnd        *time.Time
	TCPStart      *time.Time
	TCPEnd        *time.Time
	TLSStart      *time.Time
	TLSEnd        *time.Time
	RequestStart  *time.Time
	TTFB          *time.Time
	DownloadStart *time.Time
	DownloadEnd   *time.Time
	TotalStart    time.Time
}

// NewDetailedTiming creates a new DetailedTiming instance with the total timer started.
func NewDetailedTiming() *DetailedTiming {
	return &DetailedTiming{
		TotalStart: time.Now(),
	}
}

// ToTimingInfo converts the detailed timing measurements into a TimingInfo struct.
func (t *DetailedTiming) ToTimingInfo() TimingInfo {
	endTime := time.Now()
	if t.DownloadEnd != nil {
		endTime = *t.DownloadEnd
	}

	total := uint64(endTime.Sub(t.TotalStart).Milliseconds())

	info := TimingInfo{
		Total: total,
	}

	if t.DNSStart != nil && t.DNSEnd != nil {
		dns := uint64(t.DNSEnd.Sub(*t.DNSStart).Milliseconds())
		info.DNS = &dns
	}

	if t.TCPStart != nil && t.TCPEnd != nil {
		tcp := uint64(t.TCPEnd.Sub(*t.TCPStart).Milliseconds())
		info.TCP = &tcp
	}

	if t.TLSStart != nil && t.TLSEnd != nil {
		tls := uint64(t.TLSEnd.Sub(*t.TLSStart).Milliseconds())
		info.TLS = &tls
	}

	if t.RequestStart != nil && t.TTFB != nil {
		ttfb := uint64(t.TTFB.Sub(*t.RequestStart).Milliseconds())
		info.TTFB = &ttfb
	}

	if t.DownloadStart != nil && t.DownloadEnd != nil {
		download := uint64(t.DownloadEnd.Sub(*t.DownloadStart).Milliseconds())
		info.Download = &download
	}

	blocked := uint64(0)
	info.Blocked = &blocked

	return info
}

// StartDNS starts the DNS timing phase.
func (t *DetailedTiming) StartDNS() {
	now := time.Now()
	t.DNSStart = &now
}

// EndDNS ends the DNS timing phase.
func (t *DetailedTiming) EndDNS() {
	now := time.Now()
	t.DNSEnd = &now
}

// StartTCP starts the TCP connection timing phase.
func (t *DetailedTiming) StartTCP() {
	now := time.Now()
	t.TCPStart = &now
}

// EndTCP ends the TCP connection timing phase.
func (t *DetailedTiming) EndTCP() {
	now := time.Now()
	t.TCPEnd = &now
}

// StartTLS starts the TLS handshake timing phase.
func (t *DetailedTiming) StartTLS() {
	now := time.Now()
	t.TLSStart = &now
}

// EndTLS ends the TLS handshake timing phase.
func (t *DetailedTiming) EndTLS() {
	now := time.Now()
	t.TLSEnd = &now
}

// StartRequest marks the start of sending the request.
func (t *DetailedTiming) StartRequest() {
	now := time.Now()
	t.RequestStart = &now
}

// MarkTTFB marks the time to first byte.
func (t *DetailedTiming) MarkTTFB() {
	now := time.Now()
	t.TTFB = &now
}

// StartDownload starts the download timing phase.
func (t *DetailedTiming) StartDownload() {
	now := time.Now()
	t.DownloadStart = &now
}

// EndDownload ends the download timing phase.
func (t *DetailedTiming) EndDownload() {
	now := time.Now()
	t.DownloadEnd = &now
}

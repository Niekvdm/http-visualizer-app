// Package proxy provides HTTP proxy functionality for the desktop app.
package proxy

// ProxyRequest represents an incoming proxy request from the frontend.
type ProxyRequest struct {
	Method  string            `json:"method"`
	URL     string            `json:"url"`
	Headers map[string]string `json:"headers"`
	Body    *string           `json:"body,omitempty"`
	Timeout *uint64           `json:"timeout,omitempty"` // Timeout in milliseconds
}

// TimingInfo contains detailed timing information for an HTTP request.
type TimingInfo struct {
	Total    uint64  `json:"total"`              // Total request time in milliseconds
	DNS      *uint64 `json:"dns,omitempty"`      // DNS lookup time
	TCP      *uint64 `json:"tcp,omitempty"`      // TCP connection time
	TLS      *uint64 `json:"tls,omitempty"`      // TLS handshake time
	TTFB     *uint64 `json:"ttfb,omitempty"`     // Time to first byte
	Download *uint64 `json:"download,omitempty"` // Content download time
	Blocked  *uint64 `json:"blocked,omitempty"`  // Time blocked/queued
}

// RedirectHop represents information about a redirect in the chain.
type RedirectHop struct {
	URL      string            `json:"url"`
	Status   uint16            `json:"status"`
	Duration uint64            `json:"duration"`
	Headers  map[string]string `json:"headers,omitempty"`
	Opaque   *bool             `json:"opaque,omitempty"`
	Message  *string           `json:"message,omitempty"`
}

// TLSInfo contains TLS/SSL certificate information.
type TLSInfo struct {
	Protocol  *string `json:"protocol,omitempty"`
	Cipher    *string `json:"cipher,omitempty"`
	Issuer    *string `json:"issuer,omitempty"`
	Subject   *string `json:"subject,omitempty"`
	ValidFrom *uint64 `json:"validFrom,omitempty"`
	ValidTo   *uint64 `json:"validTo,omitempty"`
	Valid     *bool   `json:"valid,omitempty"`
}

// SizeBreakdown contains response size information.
type SizeBreakdown struct {
	Headers          int      `json:"headers"`
	Body             int      `json:"body"`
	Total            int      `json:"total"`
	Compressed       *int     `json:"compressed,omitempty"`
	Uncompressed     *int     `json:"uncompressed,omitempty"`
	Encoding         *string  `json:"encoding,omitempty"`
	CompressionRatio *float64 `json:"compressionRatio,omitempty"`
}

// ResponseData contains successful response data matching extension protocol.
type ResponseData struct {
	Status          uint16            `json:"status"`
	StatusText      string            `json:"statusText"`
	Headers         map[string]string `json:"headers"`
	RequestHeaders  map[string]string `json:"requestHeaders,omitempty"`
	Body            string            `json:"body"`
	BodyBase64      *string           `json:"bodyBase64,omitempty"`
	IsBinary        bool              `json:"isBinary"`
	Size            int               `json:"size"`
	Timing          TimingInfo        `json:"timing"`
	URL             string            `json:"url"`
	Redirected      bool              `json:"redirected"`
	RedirectChain   []RedirectHop     `json:"redirectChain,omitempty"`
	TLS             *TLSInfo          `json:"tls,omitempty"`
	SizeBreakdown   *SizeBreakdown    `json:"sizeBreakdown,omitempty"`
	ServerIP        *string           `json:"serverIp,omitempty"`
	Protocol        *string           `json:"protocol,omitempty"`
	FromCache       *bool             `json:"fromCache,omitempty"`
	ResourceType    *string           `json:"resourceType,omitempty"`
	RequestBodySize *int              `json:"requestBodySize,omitempty"`
	Connection      *string           `json:"connection,omitempty"`
	ServerSoftware  *string           `json:"serverSoftware,omitempty"`
}

// ErrorData contains error information matching extension protocol.
type ErrorData struct {
	Message string  `json:"message"`
	Code    string  `json:"code"`
	Name    *string `json:"name,omitempty"`
}

// ProxyResponse is the full proxy response matching extension protocol.
type ProxyResponse struct {
	Success bool          `json:"success"`
	Data    *ResponseData `json:"data,omitempty"`
	Error   *ErrorData    `json:"error,omitempty"`
}

// NewSuccessResponse creates a successful proxy response.
func NewSuccessResponse(data ResponseData) ProxyResponse {
	return ProxyResponse{
		Success: true,
		Data:    &data,
	}
}

// NewErrorResponse creates an error proxy response.
func NewErrorResponse(message, code string) ProxyResponse {
	return ProxyResponse{
		Success: false,
		Error: &ErrorData{
			Message: message,
			Code:    code,
		},
	}
}

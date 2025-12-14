package proxy

import (
	"context"
	"crypto/tls"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"strings"
	"time"

	"zone.digit.tommie/internal/infra"
)

const (
	// MaxRedirects is the maximum number of redirects to follow.
	MaxRedirects = 20
	// DefaultTimeoutMS is the default request timeout in milliseconds.
	DefaultTimeoutMS = 30000
)

// requestContext tracks request state during redirect chain.
type requestContext struct {
	url     string
	host    string
	port    string
	path    string
	isHTTPS bool
}

func newRequestContext(rawURL string) (*requestContext, error) {
	parsed, err := url.Parse(rawURL)
	if err != nil {
		return nil, fmt.Errorf("invalid URL: %w", err)
	}

	if parsed.Host == "" {
		return nil, fmt.Errorf("URL has no host")
	}

	isHTTPS := parsed.Scheme == "https"
	host := parsed.Hostname()
	port := parsed.Port()
	if port == "" {
		if isHTTPS {
			port = "443"
		} else {
			port = "80"
		}
	}

	path := parsed.RequestURI()
	if path == "" {
		path = "/"
	}

	return &requestContext{
		url:     rawURL,
		host:    host,
		port:    port,
		path:    path,
		isHTTPS: isHTTPS,
	}, nil
}

func (c *requestContext) updateFromRedirect(location string) string {
	if strings.HasPrefix(location, "http://") || strings.HasPrefix(location, "https://") {
		// Absolute URL
		parsed, err := url.Parse(location)
		if err != nil {
			c.url = location
			return location
		}

		c.isHTTPS = parsed.Scheme == "https"
		c.host = parsed.Hostname()
		c.port = parsed.Port()
		if c.port == "" {
			if c.isHTTPS {
				c.port = "443"
			} else {
				c.port = "80"
			}
		}
		c.path = parsed.RequestURI()
		if c.path == "" {
			c.path = "/"
		}

		// Rebuild URL
		defaultPort := "80"
		if c.isHTTPS {
			defaultPort = "443"
		}
		scheme := "http"
		if c.isHTTPS {
			scheme = "https"
		}

		hostWithPort := c.host
		if c.port != defaultPort {
			hostWithPort = net.JoinHostPort(c.host, c.port)
		}

		nextURL := fmt.Sprintf("%s://%s%s", scheme, hostWithPort, c.path)
		c.url = nextURL
		return nextURL
	}

	// Relative URL
	scheme := "http"
	if c.isHTTPS {
		scheme = "https"
	}
	defaultPort := "80"
	if c.isHTTPS {
		defaultPort = "443"
	}

	hostWithPort := c.host
	if c.port != defaultPort {
		hostWithPort = net.JoinHostPort(c.host, c.port)
	}

	if strings.HasPrefix(location, "/") {
		c.path = location
	} else {
		c.path = "/" + location
	}

	nextURL := fmt.Sprintf("%s://%s%s", scheme, hostWithPort, c.path)
	c.url = nextURL
	return nextURL
}

func (c *requestContext) addr() string {
	return net.JoinHostPort(c.host, c.port)
}

// ExecuteRequest executes an HTTP request with detailed timing.
func ExecuteRequest(request ProxyRequest) ProxyResponse {
	timing := NewDetailedTiming()

	// Parse initial URL
	ctx, err := newRequestContext(request.URL)
	if err != nil {
		return NewErrorResponse(err.Error(), "INVALID_URL")
	}

	timeoutMS := DefaultTimeoutMS
	if request.Timeout != nil {
		timeoutMS = int(*request.Timeout)
	}
	timeout := time.Duration(timeoutMS) * time.Millisecond

	// DNS Resolution
	timing.StartDNS()
	dnsResult, err := infra.ResolveDNS(context.Background(), ctx.host)
	if err != nil {
		return NewErrorResponse(fmt.Sprintf("DNS lookup failed: %v", err), "DNS_ERROR")
	}
	timing.EndDNS()

	var serverIP string
	var resolvedIPs []string
	for _, ip := range dnsResult.IPs {
		resolvedIPs = append(resolvedIPs, ip.String())
	}
	if len(resolvedIPs) > 0 {
		serverIP = resolvedIPs[0]
	}

	// Track redirect chain
	var redirectChain []RedirectHop
	var tlsInfo *infra.CertInfo
	var httpVersion string

	requestHeaders := request.Headers
	if requestHeaders == nil {
		requestHeaders = make(map[string]string)
	}

	var requestBodySize *int
	if request.Body != nil {
		size := len(*request.Body)
		requestBodySize = &size
	}

	isFirstRequest := true

	for {
		hopStart := time.Now()

		// Create HTTP client with custom transport for timing
		transport := &http.Transport{
			DialContext: func(dialCtx context.Context, network, addr string) (net.Conn, error) {
				if isFirstRequest {
					timing.StartTCP()
				}
				dialer := &net.Dialer{Timeout: timeout}
				conn, err := dialer.DialContext(dialCtx, network, addr)
				if isFirstRequest && err == nil {
					timing.EndTCP()
				}
				return conn, err
			},
			TLSClientConfig: &tls.Config{
				InsecureSkipVerify: false,
			},
			TLSHandshakeTimeout: timeout,
			DisableCompression:  false,
		}

		// Capture TLS info
		if ctx.isHTTPS {
			transport.DialTLSContext = func(dialCtx context.Context, network, addr string) (net.Conn, error) {
				if isFirstRequest {
					timing.StartTCP()
				}
				dialer := &net.Dialer{Timeout: timeout}
				conn, err := dialer.DialContext(dialCtx, network, addr)
				if err != nil {
					return nil, err
				}
				if isFirstRequest {
					timing.EndTCP()
					timing.StartTLS()
				}

				tlsConn := tls.Client(conn, &tls.Config{
					ServerName: ctx.host,
				})
				if err := tlsConn.HandshakeContext(dialCtx); err != nil {
					conn.Close()
					return nil, err
				}
				if isFirstRequest {
					timing.EndTLS()
					state := tlsConn.ConnectionState()
					tlsInfo = infra.ExtractCertInfo(&state)
				}
				return tlsConn, nil
			}
		}

		client := &http.Client{
			Transport: transport,
			Timeout:   timeout,
			CheckRedirect: func(req *http.Request, via []*http.Request) error {
				// Don't follow redirects automatically - we handle them manually
				return http.ErrUseLastResponse
			},
		}

		// Build request
		var bodyReader io.Reader
		if request.Body != nil {
			bodyReader = strings.NewReader(*request.Body)
		}

		httpReq, err := http.NewRequest(request.Method, ctx.url, bodyReader)
		if err != nil {
			return NewErrorResponse(fmt.Sprintf("Failed to create request: %v", err), "REQUEST_BUILD_ERROR")
		}

		// Set headers
		for key, value := range request.Headers {
			httpReq.Header.Set(key, value)
		}

		// Add accept-encoding if not set
		if httpReq.Header.Get("Accept-Encoding") == "" {
			httpReq.Header.Set("Accept-Encoding", "gzip, deflate, br")
		}

		if isFirstRequest {
			timing.StartRequest()
		}

		// Execute request
		resp, err := client.Do(httpReq)
		if err != nil {
			return NewErrorResponse(fmt.Sprintf("Request failed: %v", err), "REQUEST_FAILED")
		}

		if isFirstRequest {
			timing.MarkTTFB()
		}

		// Read response
		timing.StartDownload()
		bodyBytes, err := io.ReadAll(resp.Body)
		resp.Body.Close()
		if err != nil {
			return NewErrorResponse(fmt.Sprintf("Failed to read body: %v", err), "BODY_READ_ERROR")
		}
		timing.EndDownload()

		// Get headers
		headers := make(map[string]string)
		for key, values := range resp.Header {
			if len(values) > 0 {
				headers[strings.ToLower(key)] = values[0]
			}
		}

		httpVersion = resp.Proto

		// Check for redirect
		if resp.StatusCode >= 300 && resp.StatusCode < 400 {
			location := resp.Header.Get("Location")
			if location != "" {
				hopDuration := uint64(time.Since(hopStart).Milliseconds())
				currentURL := ctx.url
				nextURL := ctx.updateFromRedirect(location)

				redirectChain = append(redirectChain, RedirectHop{
					URL:      currentURL,
					Status:   uint16(resp.StatusCode),
					Duration: hopDuration,
					Headers:  headers,
					Message:  strPtr(fmt.Sprintf("Redirect to: %s", nextURL)),
				})

				if len(redirectChain) >= MaxRedirects {
					return NewErrorResponse("Too many redirects", "TOO_MANY_REDIRECTS")
				}

				isFirstRequest = false
				continue
			}
		}

		// Build response
		return buildResponse(responseBuildParams{
			status:          uint16(resp.StatusCode),
			headers:         headers,
			bodyBytes:       bodyBytes,
			timing:          timing,
			finalURL:        ctx.url,
			redirectChain:   redirectChain,
			tlsInfo:         tlsInfo,
			httpVersion:     httpVersion,
			serverIP:        serverIP,
			requestHeaders:  requestHeaders,
			requestBodySize: requestBodySize,
			hostname:        ctx.host,
			port:            ctx.port,
			resolvedIPs:     resolvedIPs,
		})
	}
}

func strPtr(s string) *string {
	return &s
}

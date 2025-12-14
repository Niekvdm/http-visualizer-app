package proxy

import (
	"encoding/base64"
	"fmt"
	"strings"
	"time"

	"zone.digit.tommie/internal/infra"
	"zone.digit.tommie/pkg/statustext"
)

// responseBuildParams contains parameters for building a proxy response.
type responseBuildParams struct {
	status          uint16
	headers         map[string]string
	bodyBytes       []byte
	timing          *DetailedTiming
	finalURL        string
	redirectChain   []RedirectHop
	tlsInfo         *infra.CertInfo
	httpVersion     string
	serverIP        string
	requestHeaders  map[string]string
	requestBodySize *int
	hostname        string
	port            string
	resolvedIPs     []string
}

// isBinaryContent determines if response body is likely binary based on content-type.
func isBinaryContent(contentType string) bool {
	if contentType == "" {
		return false
	}

	ct := strings.ToLower(contentType)

	textTypes := []string{
		"text/",
		"application/json",
		"application/xml",
		"application/javascript",
		"application/x-javascript",
		"application/ecmascript",
		"application/x-www-form-urlencoded",
		"+json",
		"+xml",
	}

	for _, t := range textTypes {
		if strings.Contains(ct, t) {
			return false
		}
	}

	return true
}

// buildResponse builds a ProxyResponse from raw response data.
func buildResponse(params responseBuildParams) ProxyResponse {
	contentType := params.headers["content-type"]
	contentEncoding := params.headers["content-encoding"]
	isBinary := isBinaryContent(contentType)

	// Decompress if needed
	compressedSize := len(params.bodyBytes)
	decompressResult, err := infra.Decompress(params.bodyBytes, contentEncoding)
	if err != nil {
		return NewErrorResponse(fmt.Sprintf("Decompression failed: %v", err), "DECOMPRESSION_ERROR")
	}
	decompressed := decompressResult.Data
	bodySize := len(decompressed)

	// Convert body
	var body string
	var bodyBase64 *string
	if isBinary {
		b64 := base64.StdEncoding.EncodeToString(decompressed)
		bodyBase64 = &b64
	} else {
		body = string(decompressed)
	}

	// Calculate sizes
	statusLine := fmt.Sprintf("%s %d %s", params.httpVersion, params.status, statustext.Get(int(params.status)))
	headerSize := len(statusLine) + 2
	for k, v := range params.headers {
		headerSize += len(k) + 2 + len(v) + 2
	}

	var compressionRatio *float64
	var compressed *int
	var uncompressed *int
	var encoding *string

	if contentEncoding != "" && bodySize > 0 {
		ratio := float64(compressedSize) / float64(bodySize)
		compressionRatio = &ratio
		compressed = &compressedSize
		uncompressed = &bodySize
		encoding = &contentEncoding
	}

	sizeBreakdown := &SizeBreakdown{
		Headers:          headerSize,
		Body:             bodySize,
		Total:            headerSize + bodySize,
		Compressed:       compressed,
		Uncompressed:     uncompressed,
		Encoding:         encoding,
		CompressionRatio: compressionRatio,
	}

	// Build TLS info
	var tlsInfoData *TLSInfo
	if params.tlsInfo != nil {
		valid := infra.IsCertValid(params.tlsInfo.ValidFrom, params.tlsInfo.ValidTo)
		tlsInfoData = &TLSInfo{
			Protocol:  strPtr(params.tlsInfo.Protocol),
			Cipher:    strPtr(params.tlsInfo.Cipher),
			Issuer:    strPtr(params.tlsInfo.Issuer),
			Subject:   strPtr(params.tlsInfo.Subject),
			ValidFrom: &params.tlsInfo.ValidFrom,
			ValidTo:   &params.tlsInfo.ValidTo,
			Valid:     &valid,
			SANs:      params.tlsInfo.SANs,
		}
	}

	serverSoftware := params.headers["server"]
	connection := params.headers["connection"]

	var serverSoftwarePtr, connectionPtr *string
	if serverSoftware != "" {
		serverSoftwarePtr = &serverSoftware
	}
	if connection != "" {
		connectionPtr = &connection
	}

	var serverIPPtr *string
	if params.serverIP != "" {
		serverIPPtr = &params.serverIP
	}

	var hostnamePtr *string
	if params.hostname != "" {
		hostnamePtr = &params.hostname
	}

	var portPtr *string
	if params.port != "" {
		portPtr = &params.port
	}

	fromCache := false
	resourceType := "fetch"

	var redirectChainPtr []RedirectHop
	if len(params.redirectChain) > 0 {
		redirectChainPtr = params.redirectChain
	}

	data := ResponseData{
		Status:          params.status,
		StatusText:      statustext.Get(int(params.status)),
		Headers:         params.headers,
		RequestHeaders:  params.requestHeaders,
		Body:            body,
		BodyBase64:      bodyBase64,
		IsBinary:        isBinary,
		Size:            bodySize,
		Timing:          params.timing.ToTimingInfo(),
		URL:             params.finalURL,
		Redirected:      len(params.redirectChain) > 0,
		RedirectChain:   redirectChainPtr,
		TLS:             tlsInfoData,
		SizeBreakdown:   sizeBreakdown,
		ServerIP:        serverIPPtr,
		Protocol:        &params.httpVersion,
		FromCache:       &fromCache,
		ResourceType:    &resourceType,
		RequestBodySize: params.requestBodySize,
		Connection:      connectionPtr,
		ServerSoftware:  serverSoftwarePtr,
		Hostname:        hostnamePtr,
		Port:            portPtr,
		ResolvedIPs:     params.resolvedIPs,
	}

	return NewSuccessResponse(data)
}

// uint64Ptr creates a pointer to a uint64.
func uint64Ptr(v uint64) *uint64 {
	return &v
}

// nowUnix returns the current Unix timestamp.
func nowUnix() uint64 {
	return uint64(time.Now().Unix())
}

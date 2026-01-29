// Package sdk provides a strongly typed GraphQL SDK for Go.
package sdk

import (
	"fmt"
)

// ErrorCode represents typed error codes for compile-time safety.
type ErrorCode string

const (
	// Network errors
	ErrNetworkError      ErrorCode = "NETWORK_ERROR"
	ErrTimeout           ErrorCode = "TIMEOUT"
	ErrConnectionRefused ErrorCode = "CONNECTION_REFUSED"

	// Protocol errors
	ErrHttpError        ErrorCode = "HTTP_ERROR"
	ErrInvalidUrl       ErrorCode = "INVALID_URL"
	ErrInvalidResponse  ErrorCode = "INVALID_RESPONSE"

	// GraphQL errors
	ErrParseError      ErrorCode = "PARSE_ERROR"
	ErrValidationError ErrorCode = "VALIDATION_ERROR"
	ErrExecutionError  ErrorCode = "EXECUTION_ERROR"
	ErrNoOperation     ErrorCode = "NO_OPERATION"
	ErrNoData          ErrorCode = "NO_DATA"

	// Auth errors
	ErrAuthError    ErrorCode = "AUTH_ERROR"
	ErrUnauthorized ErrorCode = "UNAUTHORIZED"
	ErrForbidden    ErrorCode = "FORBIDDEN"

	// Resource errors
	ErrNotFound ErrorCode = "NOT_FOUND"
	ErrConflict ErrorCode = "CONFLICT"

	// Internal errors
	ErrInternalError ErrorCode = "INTERNAL_ERROR"
)

// IsRetryable returns true if this error code represents a retryable error.
func (c ErrorCode) IsRetryable() bool {
	switch c {
	case ErrNetworkError, ErrTimeout, ErrConnectionRefused:
		return true
	default:
		return false
	}
}

// IsClientError returns true if this is a client error.
func (c ErrorCode) IsClientError() bool {
	switch c {
	case ErrParseError, ErrValidationError, ErrAuthError, ErrUnauthorized,
		ErrForbidden, ErrNotFound, ErrInvalidUrl, ErrNoOperation:
		return true
	default:
		return false
	}
}

// IsServerError returns true if this is a server error.
func (c ErrorCode) IsServerError() bool {
	switch c {
	case ErrInternalError, ErrExecutionError:
		return true
	default:
		return false
	}
}

// SdkError is a strongly typed SDK error.
type SdkError struct {
	Code       ErrorCode
	Message    string
	Cause      error
	Extensions map[string]any
}

// Error implements the error interface.
func (e *SdkError) Error() string {
	if e.Cause != nil {
		return fmt.Sprintf("[%s] %s: %v", e.Code, e.Message, e.Cause)
	}
	return fmt.Sprintf("[%s] %s", e.Code, e.Message)
}

// Unwrap implements errors.Unwrap.
func (e *SdkError) Unwrap() error {
	return e.Cause
}

// Is implements errors.Is.
func (e *SdkError) Is(target error) bool {
	if t, ok := target.(*SdkError); ok {
		return e.Code == t.Code
	}
	return false
}

// WithCause adds a cause to the error.
func (e *SdkError) WithCause(cause error) *SdkError {
	e.Cause = cause
	return e
}

// WithExtension adds an extension to the error.
func (e *SdkError) WithExtension(key string, value any) *SdkError {
	if e.Extensions == nil {
		e.Extensions = make(map[string]any)
	}
	e.Extensions[key] = value
	return e
}

// NewError creates a new SDK error.
func NewError(code ErrorCode, message string) *SdkError {
	return &SdkError{
		Code:    code,
		Message: message,
	}
}

// Error constructors
var (
	ErrNetwork = func(message string) *SdkError {
		return NewError(ErrNetworkError, message)
	}
	ErrTimeoutError = func() *SdkError {
		return NewError(ErrTimeout, "Request timed out")
	}
	ErrParse = func(message string) *SdkError {
		return NewError(ErrParseError, message)
	}
	ErrValidation = func(message string) *SdkError {
		return NewError(ErrValidationError, message)
	}
	ErrAuth = func(message string) *SdkError {
		return NewError(ErrAuthError, message)
	}
	ErrResourceNotFound = func(resource string) *SdkError {
		return NewError(ErrNotFound, fmt.Sprintf("%s not found", resource))
	}
	ErrInternal = func(message string) *SdkError {
		return NewError(ErrInternalError, message)
	}
)

// IsSdkError checks if an error is an SdkError.
func IsSdkError(err error) bool {
	_, ok := err.(*SdkError)
	return ok
}

// AsSdkError attempts to extract an SdkError from an error.
func AsSdkError(err error) (*SdkError, bool) {
	sdkErr, ok := err.(*SdkError)
	return sdkErr, ok
}

// GraphQLError represents a GraphQL error from the server.
type GraphQLError struct {
	Message    string         `json:"message"`
	Path       []any          `json:"path,omitempty"`
	Locations  []Location     `json:"locations,omitempty"`
	Extensions map[string]any `json:"extensions,omitempty"`
}

// Location represents a location in a GraphQL document.
type Location struct {
	Line   int `json:"line"`
	Column int `json:"column"`
}

// Error implements the error interface.
func (e *GraphQLError) Error() string {
	return e.Message
}

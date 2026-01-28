// Package bgql provides Go bindings for Better GraphQL.
//
// This package wraps the C FFI bindings to provide a native Go API.
//
// Example usage:
//
//	ctx := bgql.NewContext()
//	defer ctx.Free()
//
//	result, err := ctx.Parse(`type Query { hello: String }`)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println("Parse successful:", result.Success)
package bgql

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lbgql_ffi
#include "../../crates/bgql_ffi/include/bgql.h"
#include <stdlib.h>
*/
import "C"
import (
	"errors"
	"unsafe"
)

// Context represents a Better GraphQL context.
type Context struct {
	ptr *C.bgql_context_t
}

// ParseResult represents the result of parsing a GraphQL document.
type ParseResult struct {
	Success bool
	Error   string
}

// FormatResult represents the result of formatting a GraphQL document.
type FormatResult struct {
	Success bool
	Output  string
	Error   string
}

// NewContext creates a new Better GraphQL context.
func NewContext() *Context {
	return &Context{
		ptr: C.bgql_context_new(),
	}
}

// Free releases the resources associated with the context.
// The context must not be used after calling Free.
func (c *Context) Free() {
	if c.ptr != nil {
		C.bgql_context_free(c.ptr)
		c.ptr = nil
	}
}

// Parse parses a GraphQL document.
func (c *Context) Parse(source string) (*ParseResult, error) {
	if c.ptr == nil {
		return nil, errors.New("context has been freed")
	}

	cSource := C.CString(source)
	defer C.free(unsafe.Pointer(cSource))

	result := C.bgql_parse(c.ptr, cSource)
	if result == nil {
		return nil, errors.New("failed to parse")
	}
	defer C.bgql_parse_result_free(result)

	success := C.bgql_parse_result_success(result) == 1
	var errMsg string
	if !success {
		if errPtr := C.bgql_parse_result_error(result); errPtr != nil {
			errMsg = C.GoString(errPtr)
		}
	}

	return &ParseResult{
		Success: success,
		Error:   errMsg,
	}, nil
}

// Format formats a GraphQL document.
func Format(source string) (*FormatResult, error) {
	cSource := C.CString(source)
	defer C.free(unsafe.Pointer(cSource))

	result := C.bgql_format(cSource)
	if result == nil {
		return nil, errors.New("failed to format")
	}
	defer C.bgql_format_result_free(result)

	success := C.bgql_format_result_success(result) == 1
	var output, errMsg string

	if success {
		if outPtr := C.bgql_format_result_output(result); outPtr != nil {
			output = C.GoString(outPtr)
		}
	} else {
		if errPtr := C.bgql_format_result_error(result); errPtr != nil {
			errMsg = C.GoString(errPtr)
		}
	}

	return &FormatResult{
		Success: success,
		Output:  output,
		Error:   errMsg,
	}, nil
}

// Version returns the version string of the library.
func Version() string {
	return C.GoString(C.bgql_version())
}

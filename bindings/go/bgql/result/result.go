// Package result provides Result type for type-safe error handling.
// Inspired by Rust's Result<T, E> type.
package result

import (
	"encoding/json"
	"fmt"
)

// Result represents either a success value or an error.
// Use Ok() to create success, Err() to create failure.
type Result[T any] struct {
	value T
	err   error
	ok    bool
}

// Ok creates a successful Result with the given value.
func Ok[T any](value T) Result[T] {
	return Result[T]{value: value, ok: true}
}

// Err creates a failed Result with the given error.
func Err[T any](err error) Result[T] {
	return Result[T]{err: err, ok: false}
}

// ErrMsg creates a failed Result with a string message.
func ErrMsg[T any](message string) Result[T] {
	return Result[T]{err: fmt.Errorf("%s", message), ok: false}
}

// IsOk returns true if the Result is successful.
func (r Result[T]) IsOk() bool {
	return r.ok
}

// IsErr returns true if the Result is an error.
func (r Result[T]) IsErr() bool {
	return !r.ok
}

// Unwrap returns the value if Ok, panics if Err.
// Use sparingly - prefer Match or UnwrapOr.
func (r Result[T]) Unwrap() T {
	if !r.ok {
		panic(fmt.Sprintf("called Unwrap on Err: %v", r.err))
	}
	return r.value
}

// UnwrapOr returns the value if Ok, or the default value if Err.
func (r Result[T]) UnwrapOr(defaultValue T) T {
	if r.ok {
		return r.value
	}
	return defaultValue
}

// UnwrapOrElse returns the value if Ok, or calls the function with the error if Err.
func (r Result[T]) UnwrapOrElse(fn func(error) T) T {
	if r.ok {
		return r.value
	}
	return fn(r.err)
}

// Error returns the error if Err, nil if Ok.
func (r Result[T]) Error() error {
	if r.ok {
		return nil
	}
	return r.err
}

// Value returns the value and a boolean indicating success.
func (r Result[T]) Value() (T, bool) {
	return r.value, r.ok
}

// Map transforms the value if Ok, passes through if Err.
func Map[T, U any](r Result[T], fn func(T) U) Result[U] {
	if r.ok {
		return Ok(fn(r.value))
	}
	return Err[U](r.err)
}

// MapErr transforms the error if Err, passes through if Ok.
func MapErr[T any](r Result[T], fn func(error) error) Result[T] {
	if !r.ok {
		return Err[T](fn(r.err))
	}
	return r
}

// AndThen chains Results together (flatMap).
func AndThen[T, U any](r Result[T], fn func(T) Result[U]) Result[U] {
	if r.ok {
		return fn(r.value)
	}
	return Err[U](r.err)
}

// Match performs pattern matching on a Result.
func Match[T, U any](r Result[T], onOk func(T) U, onErr func(error) U) U {
	if r.ok {
		return onOk(r.value)
	}
	return onErr(r.err)
}

// All combines multiple Results into a single Result containing a slice.
// Returns Err with the first error if any Result is Err.
func All[T any](results ...Result[T]) Result[[]T] {
	values := make([]T, 0, len(results))
	for _, r := range results {
		if !r.ok {
			return Err[[]T](r.err)
		}
		values = append(values, r.value)
	}
	return Ok(values)
}

// Partition separates Results into values and errors.
func Partition[T any](results ...Result[T]) (values []T, errors []error) {
	for _, r := range results {
		if r.ok {
			values = append(values, r.value)
		} else {
			errors = append(errors, r.err)
		}
	}
	return
}

// FromError converts a value and error pair to a Result.
// Common pattern for wrapping Go functions that return (T, error).
func FromError[T any](value T, err error) Result[T] {
	if err != nil {
		return Err[T](err)
	}
	return Ok(value)
}

// MarshalJSON implements json.Marshaler for Result.
func (r Result[T]) MarshalJSON() ([]byte, error) {
	if r.ok {
		return json.Marshal(map[string]any{
			"ok":    true,
			"value": r.value,
		})
	}
	return json.Marshal(map[string]any{
		"ok":    false,
		"error": r.err.Error(),
	})
}

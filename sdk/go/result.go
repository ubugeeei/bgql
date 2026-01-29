package sdk

// Result represents either a success value or an error.
// Uses Go generics for type safety.
type Result[T any] struct {
	value T
	err   error
	ok    bool
}

// Ok creates a successful Result.
func Ok[T any](value T) Result[T] {
	return Result[T]{value: value, ok: true}
}

// Err creates an error Result.
func Err[T any](err error) Result[T] {
	return Result[T]{err: err, ok: false}
}

// IsOk returns true if the Result is successful.
func (r Result[T]) IsOk() bool {
	return r.ok
}

// IsErr returns true if the Result is an error.
func (r Result[T]) IsErr() bool {
	return !r.ok
}

// Unwrap returns the value or panics if error.
func (r Result[T]) Unwrap() T {
	if !r.ok {
		panic("Unwrap called on Err result")
	}
	return r.value
}

// UnwrapOr returns the value or a default.
func (r Result[T]) UnwrapOr(defaultValue T) T {
	if r.ok {
		return r.value
	}
	return defaultValue
}

// UnwrapOrElse returns the value or computes a default.
func (r Result[T]) UnwrapOrElse(fn func(error) T) T {
	if r.ok {
		return r.value
	}
	return fn(r.err)
}

// Error returns the error or nil.
func (r Result[T]) Error() error {
	if r.ok {
		return nil
	}
	return r.err
}

// Value returns the value and ok status.
func (r Result[T]) Value() (T, bool) {
	return r.value, r.ok
}

// Match pattern matches on the Result.
func (r Result[T]) Match(onOk func(T), onErr func(error)) {
	if r.ok {
		onOk(r.value)
	} else {
		onErr(r.err)
	}
}

// Map transforms the success value.
func Map[T, U any](r Result[T], fn func(T) U) Result[U] {
	if r.ok {
		return Ok(fn(r.value))
	}
	return Err[U](r.err)
}

// MapErr transforms the error.
func MapErr[T any](r Result[T], fn func(error) error) Result[T] {
	if r.ok {
		return r
	}
	return Err[T](fn(r.err))
}

// FlatMap chains Result computations.
func FlatMap[T, U any](r Result[T], fn func(T) Result[U]) Result[U] {
	if r.ok {
		return fn(r.value)
	}
	return Err[U](r.err)
}

// All combines multiple Results into one.
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

// Collect collects Results from a slice.
func Collect[T any](items []T, fn func(T) Result[T]) Result[[]T] {
	results := make([]T, 0, len(items))
	for _, item := range items {
		r := fn(item)
		if !r.ok {
			return Err[[]T](r.err)
		}
		results = append(results, r.value)
	}
	return Ok(results)
}

// Try wraps a function that may panic into a Result.
func Try[T any](fn func() T) (result Result[T]) {
	defer func() {
		if r := recover(); r != nil {
			if err, ok := r.(error); ok {
				result = Err[T](err)
			} else {
				result = Err[T](NewError(ErrInternalError, "panic occurred"))
			}
		}
	}()
	return Ok(fn())
}

// FromError creates a Result from a value and error pair (Go idiom).
func FromError[T any](value T, err error) Result[T] {
	if err != nil {
		return Err[T](err)
	}
	return Ok(value)
}

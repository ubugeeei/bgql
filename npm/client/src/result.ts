/**
 * Result type pattern for type-safe error handling.
 * Inspired by Rust's Result<T, E> type.
 */

/**
 * A Result type that represents either success (Ok) or failure (Err).
 * This is the core type for all bgql operations.
 */
export type Result<T, E> = Ok<T> | Err<E>;

/**
 * Represents a successful result.
 */
export interface Ok<T> {
  readonly ok: true;
  readonly value: T;
  readonly error?: never;
}

/**
 * Represents a failed result.
 */
export interface Err<E> {
  readonly ok: false;
  readonly value?: never;
  readonly error: E;
}

/**
 * Creates a successful Result.
 */
export function ok<T>(value: T): Ok<T> {
  return { ok: true, value };
}

/**
 * Creates a failed Result.
 */
export function err<E>(error: E): Err<E> {
  return { ok: false, error };
}

/**
 * Type guard to check if a Result is Ok.
 */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
  return result.ok === true;
}

/**
 * Type guard to check if a Result is Err.
 */
export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
  return result.ok === false;
}

/**
 * Maps the value of a Result if it's Ok.
 */
export function map<T, U, E>(
  result: Result<T, E>,
  fn: (value: T) => U
): Result<U, E> {
  if (isOk(result)) {
    return ok(fn(result.value));
  }
  return result;
}

/**
 * Maps the error of a Result if it's Err.
 */
export function mapErr<T, E, F>(
  result: Result<T, E>,
  fn: (error: E) => F
): Result<T, F> {
  if (isErr(result)) {
    return err(fn(result.error));
  }
  return result;
}

/**
 * Chains Results together (flatMap).
 */
export function andThen<T, U, E>(
  result: Result<T, E>,
  fn: (value: T) => Result<U, E>
): Result<U, E> {
  if (isOk(result)) {
    return fn(result.value);
  }
  return result;
}

/**
 * Unwraps a Result, throwing if it's an Err.
 * Use sparingly - prefer pattern matching.
 */
export function unwrap<T, E>(result: Result<T, E>): T {
  if (isOk(result)) {
    return result.value;
  }
  throw new Error(`Attempted to unwrap an Err: ${JSON.stringify(result.error)}`);
}

/**
 * Unwraps a Result with a default value.
 */
export function unwrapOr<T, E>(result: Result<T, E>, defaultValue: T): T {
  if (isOk(result)) {
    return result.value;
  }
  return defaultValue;
}

/**
 * Unwraps a Result with a lazy default value.
 */
export function unwrapOrElse<T, E>(
  result: Result<T, E>,
  fn: (error: E) => T
): T {
  if (isOk(result)) {
    return result.value;
  }
  return fn(result.error);
}

/**
 * Pattern matching for Results.
 *
 * @example
 * ```typescript
 * const result = await client.getUser({ id: "1" });
 * const message = match(result, {
 *   ok: (user) => `Hello, ${user.name}!`,
 *   err: (error) => `Error: ${error.message}`,
 * });
 * ```
 */
export function match<T, E, U>(
  result: Result<T, E>,
  handlers: {
    ok: (value: T) => U;
    err: (error: E) => U;
  }
): U {
  if (isOk(result)) {
    return handlers.ok(result.value);
  }
  return handlers.err(result.error);
}

/**
 * Converts a Promise<T> to Promise<Result<T, Error>>.
 * Catches any thrown errors and wraps them in Err.
 */
export async function fromPromise<T>(
  promise: Promise<T>
): Promise<Result<T, Error>> {
  try {
    const value = await promise;
    return ok(value);
  } catch (error) {
    return err(error instanceof Error ? error : new Error(String(error)));
  }
}

/**
 * Converts a Result<T, E> to a Promise<T>.
 * Rejects with the error if it's Err.
 */
export function toPromise<T, E>(result: Result<T, E>): Promise<T> {
  if (isOk(result)) {
    return Promise.resolve(result.value);
  }
  return Promise.reject(result.error);
}

/**
 * Combines multiple Results into a single Result containing an array.
 * Returns Err with the first error if any Result is Err.
 */
export function all<T, E>(results: Result<T, E>[]): Result<T[], E> {
  const values: T[] = [];
  for (const result of results) {
    if (isErr(result)) {
      return result;
    }
    values.push(result.value);
  }
  return ok(values);
}

/**
 * Combines multiple Results, collecting all errors.
 */
export function partition<T, E>(
  results: Result<T, E>[]
): { values: T[]; errors: E[] } {
  const values: T[] = [];
  const errors: E[] = [];
  for (const result of results) {
    if (isOk(result)) {
      values.push(result.value);
    } else {
      errors.push(result.error);
    }
  }
  return { values, errors };
}

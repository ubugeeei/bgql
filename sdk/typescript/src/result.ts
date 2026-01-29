/**
 * Strongly typed Result type for error handling.
 *
 * Uses discriminated unions for exhaustive pattern matching.
 */

import type { SdkError } from "./error";

/**
 * Success result.
 */
export interface Ok<T> {
  readonly _tag: "Ok";
  readonly value: T;
}

/**
 * Error result.
 */
export interface Err<E> {
  readonly _tag: "Err";
  readonly error: E;
}

/**
 * Result type - either Ok or Err.
 */
export type Result<T, E = SdkError> = Ok<T> | Err<E>;

/**
 * Creates an Ok result.
 */
export function ok<T>(value: T): Ok<T> {
  return { _tag: "Ok", value };
}

/**
 * Creates an Err result.
 */
export function err<E>(error: E): Err<E> {
  return { _tag: "Err", error };
}

/**
 * Type guard for Ok results.
 */
export function isOk<T, E>(result: Result<T, E>): result is Ok<T> {
  return result._tag === "Ok";
}

/**
 * Type guard for Err results.
 */
export function isErr<T, E>(result: Result<T, E>): result is Err<E> {
  return result._tag === "Err";
}

/**
 * Maps the Ok value.
 */
export function map<T, U, E>(
  result: Result<T, E>,
  fn: (value: T) => U
): Result<U, E> {
  return isOk(result) ? ok(fn(result.value)) : result;
}

/**
 * Maps the Err value.
 */
export function mapErr<T, E, F>(
  result: Result<T, E>,
  fn: (error: E) => F
): Result<T, F> {
  return isErr(result) ? err(fn(result.error)) : result;
}

/**
 * Flat maps the Ok value.
 */
export function flatMap<T, U, E>(
  result: Result<T, E>,
  fn: (value: T) => Result<U, E>
): Result<U, E> {
  return isOk(result) ? fn(result.value) : result;
}

/**
 * Unwraps the Ok value or throws.
 */
export function unwrap<T, E>(result: Result<T, E>): T {
  if (isOk(result)) {
    return result.value;
  }
  throw new Error(`Unwrap called on Err: ${JSON.stringify(result.error)}`);
}

/**
 * Unwraps the Ok value or returns a default.
 */
export function unwrapOr<T, E>(result: Result<T, E>, defaultValue: T): T {
  return isOk(result) ? result.value : defaultValue;
}

/**
 * Unwraps the Ok value or computes a default.
 */
export function unwrapOrElse<T, E>(
  result: Result<T, E>,
  fn: (error: E) => T
): T {
  return isOk(result) ? result.value : fn(result.error);
}

/**
 * Pattern matches on a Result.
 */
export function match<T, E, U>(
  result: Result<T, E>,
  handlers: {
    ok: (value: T) => U;
    err: (error: E) => U;
  }
): U {
  return isOk(result) ? handlers.ok(result.value) : handlers.err(result.error);
}

/**
 * Converts a Promise to a Result.
 */
export async function fromPromise<T, E = SdkError>(
  promise: Promise<T>,
  mapError: (error: unknown) => E
): Promise<Result<T, E>> {
  try {
    const value = await promise;
    return ok(value);
  } catch (error) {
    return err(mapError(error));
  }
}

/**
 * Converts a Result to a Promise.
 */
export function toPromise<T, E>(result: Result<T, E>): Promise<T> {
  return isOk(result) ? Promise.resolve(result.value) : Promise.reject(result.error);
}

/**
 * Combines multiple Results into a single Result.
 */
export function all<T extends readonly unknown[], E>(
  results: { [K in keyof T]: Result<T[K], E> }
): Result<T, E> {
  const values: unknown[] = [];
  for (const result of results) {
    if (isErr(result)) {
      return result;
    }
    values.push(result.value);
  }
  return ok(values as unknown as T);
}

/**
 * Async Result type alias.
 */
export type AsyncResult<T, E = SdkError> = Promise<Result<T, E>>;

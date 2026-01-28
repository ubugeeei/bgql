/**
 * @bgql/client - Type-safe GraphQL client for bgql schemas
 *
 * Core principles:
 * 1. Errors as values (no throw-based error handling)
 * 2. Full type safety at compile time
 * 3. Partial Promise for streaming (@defer/@stream)
 * 4. Native AbortController support
 */

export * from './result';
export * from './client';
export * from './types';
export * from './errors';

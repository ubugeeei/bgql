/**
 * Type-safe context for request-scoped data.
 *
 * Uses branded types and TypeScript's structural typing for type safety.
 */

/**
 * Brand type for nominal typing.
 */
declare const brand: unique symbol;
type Brand<T, B> = T & { readonly [brand]: B };

/**
 * Context key type - branded for type safety.
 */
export type ContextKey<T> = Brand<symbol, T>;

/**
 * Creates a typed context key.
 */
export function contextKey<T>(description?: string): ContextKey<T> {
  return Symbol(description) as ContextKey<T>;
}

/**
 * Type-safe context storage.
 */
export class TypedContext {
  private readonly data = new Map<symbol, unknown>();
  private readonly headers = new Map<string, string>();

  /**
   * Sets a typed value.
   */
  set<T>(key: ContextKey<T>, value: T): this {
    this.data.set(key, value);
    return this;
  }

  /**
   * Gets a typed value.
   */
  get<T>(key: ContextKey<T>): T | undefined {
    return this.data.get(key) as T | undefined;
  }

  /**
   * Gets a typed value or throws.
   */
  require<T>(key: ContextKey<T>): T {
    const value = this.get(key);
    if (value === undefined) {
      throw new Error(`Required context key not found: ${key.toString()}`);
    }
    return value;
  }

  /**
   * Checks if a key exists.
   */
  has<T>(key: ContextKey<T>): boolean {
    return this.data.has(key);
  }

  /**
   * Deletes a value.
   */
  delete<T>(key: ContextKey<T>): boolean {
    return this.data.delete(key);
  }

  /**
   * Sets a header.
   */
  setHeader(name: string, value: string): this {
    this.headers.set(name.toLowerCase(), value);
    return this;
  }

  /**
   * Gets a header.
   */
  getHeader(name: string): string | undefined {
    return this.headers.get(name.toLowerCase());
  }

  /**
   * Gets all headers.
   */
  getHeaders(): ReadonlyMap<string, string> {
    return this.headers;
  }

  /**
   * Creates a child context with inherited values.
   */
  child(): TypedContext {
    const child = new TypedContext();
    for (const [key, value] of this.data) {
      child.data.set(key, value);
    }
    for (const [key, value] of this.headers) {
      child.headers.set(key, value);
    }
    return child;
  }
}

/**
 * Creates a new typed context.
 */
export function createContext(): TypedContext {
  return new TypedContext();
}

// Common context keys
export const CurrentUserId = contextKey<string>("CurrentUserId");
export const UserRoles = contextKey<readonly string[]>("UserRoles");
export const RequestId = contextKey<string>("RequestId");
export const RequestStartTime = contextKey<number>("RequestStartTime");

/**
 * User roles helper.
 */
export interface RolesHelper {
  readonly roles: readonly string[];
  has(role: string): boolean;
  hasAny(...roles: string[]): boolean;
  hasAll(...roles: string[]): boolean;
}

/**
 * Creates a roles helper.
 */
export function createRolesHelper(roles: readonly string[]): RolesHelper {
  return {
    roles,
    has: (role) => roles.includes(role),
    hasAny: (...checkRoles) => checkRoles.some((r) => roles.includes(r)),
    hasAll: (...checkRoles) => checkRoles.every((r) => roles.includes(r)),
  };
}

/**
 * Context builder for fluent API.
 */
export class ContextBuilder {
  private readonly ctx = new TypedContext();

  with<T>(key: ContextKey<T>, value: T): this {
    this.ctx.set(key, value);
    return this;
  }

  withHeader(name: string, value: string): this {
    this.ctx.setHeader(name, value);
    return this;
  }

  withUserId(userId: string): this {
    return this.with(CurrentUserId, userId);
  }

  withRoles(roles: readonly string[]): this {
    return this.with(UserRoles, roles);
  }

  withRequestId(requestId?: string): this {
    const id = requestId ?? `req_${Date.now().toString(36)}`;
    return this.with(RequestId, id);
  }

  build(): TypedContext {
    return this.ctx;
  }
}

/**
 * Creates a context builder.
 */
export function buildContext(): ContextBuilder {
  return new ContextBuilder();
}

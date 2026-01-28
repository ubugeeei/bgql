/**
 * Context creation and management for bgql server.
 */

import type {
  BaseContext,
  AuthInfo,
  AuthenticatedUser,
  RequestInfo,
  ResponseHelpers,
  CookieOptions,
  IncomingRequest,
} from './types';

/**
 * Creates a base context from an incoming request.
 */
export function createBaseContext(req: IncomingRequest): BaseContext {
  const responseHeaders = new Map<string, string>();
  const responseCookies = new Map<string, { value: string; options?: CookieOptions }>();

  return {
    request: {
      headers: req.headers,
      cookies: parseCookies(req.headers.get('cookie') ?? ''),
      ip: getClientIp(req.headers),
      method: req.method,
      url: req.url,
    },
    response: {
      setHeader(name: string, value: string) {
        responseHeaders.set(name, value);
      },
      setCookie(name: string, value: string, options?: CookieOptions) {
        responseCookies.set(name, { value, options });
      },
      deleteCookie(name: string) {
        responseCookies.set(name, { value: '', options: { maxAge: 0 } });
      },
    },
    auth: createUnauthenticatedAuth(),
    signal: req.signal,
  };
}

/**
 * Creates an authenticated context.
 */
export function createAuthenticatedContext<TContext extends BaseContext>(
  baseContext: TContext,
  user: AuthenticatedUser
): TContext & { auth: AuthInfo & { user: AuthenticatedUser } } {
  return {
    ...baseContext,
    auth: {
      user,
      isAuthenticated: true,
      hasRole: (role: string) => user.roles.includes(role),
      hasScope: (scope: string) => user.scopes.includes(scope),
      hasPermission: (permission: string) => user.permissions.includes(permission),
    },
  };
}

/**
 * Creates an unauthenticated auth info object.
 */
function createUnauthenticatedAuth(): AuthInfo {
  return {
    user: null,
    isAuthenticated: false,
    hasRole: () => false,
    hasScope: () => false,
    hasPermission: () => false,
  };
}

/**
 * Parses cookies from a cookie header string.
 */
function parseCookies(cookieHeader: string): ReadonlyMap<string, string> {
  const cookies = new Map<string, string>();

  if (!cookieHeader) {
    return cookies;
  }

  for (const cookie of cookieHeader.split(';')) {
    const [name, ...valueParts] = cookie.split('=');
    if (name) {
      cookies.set(name.trim(), valueParts.join('=').trim());
    }
  }

  return cookies;
}

/**
 * Gets the client IP from headers.
 */
function getClientIp(headers: Headers): string {
  // Check common proxy headers
  const forwarded = headers.get('x-forwarded-for');
  if (forwarded) {
    return forwarded.split(',')[0].trim();
  }

  const realIp = headers.get('x-real-ip');
  if (realIp) {
    return realIp;
  }

  return '0.0.0.0';
}

/**
 * Type guard to check if context is authenticated.
 */
export function isAuthenticated<TContext extends BaseContext>(
  context: TContext
): context is TContext & { auth: AuthInfo & { user: AuthenticatedUser } } {
  return context.auth.isAuthenticated && context.auth.user !== null;
}

/**
 * Requires authentication, returning an unauthorized error if not authenticated.
 */
export function requireAuth<TContext extends BaseContext, TResult>(
  context: TContext,
  fn: (authenticatedContext: TContext & { auth: AuthInfo & { user: AuthenticatedUser } }) => TResult
): TResult | { __typename: 'UnauthorizedError'; message: string; code: string } {
  if (!isAuthenticated(context)) {
    return {
      __typename: 'UnauthorizedError',
      message: 'Authentication required',
      code: 'UNAUTHORIZED',
    };
  }
  return fn(context);
}

/**
 * Requires a specific role.
 */
export function requireRole<TContext extends BaseContext, TResult>(
  context: TContext,
  role: string,
  fn: (authenticatedContext: TContext & { auth: AuthInfo & { user: AuthenticatedUser } }) => TResult
): TResult | { __typename: 'UnauthorizedError' | 'ForbiddenError'; message: string; code: string } {
  if (!isAuthenticated(context)) {
    return {
      __typename: 'UnauthorizedError',
      message: 'Authentication required',
      code: 'UNAUTHORIZED',
    };
  }

  if (!context.auth.hasRole(role)) {
    return {
      __typename: 'ForbiddenError',
      message: `Role '${role}' required`,
      code: 'FORBIDDEN',
    };
  }

  return fn(context);
}

/**
 * Requires a specific permission.
 */
export function requirePermission<TContext extends BaseContext, TResult>(
  context: TContext,
  permission: string,
  fn: (authenticatedContext: TContext & { auth: AuthInfo & { user: AuthenticatedUser } }) => TResult
): TResult | { __typename: 'UnauthorizedError' | 'ForbiddenError'; message: string; code: string } {
  if (!isAuthenticated(context)) {
    return {
      __typename: 'UnauthorizedError',
      message: 'Authentication required',
      code: 'UNAUTHORIZED',
    };
  }

  if (!context.auth.hasPermission(permission)) {
    return {
      __typename: 'ForbiddenError',
      message: `Permission '${permission}' required`,
      code: 'FORBIDDEN',
    };
  }

  return fn(context);
}

/**
 * Serializes cookies for the Set-Cookie header.
 */
export function serializeCookie(
  name: string,
  value: string,
  options?: CookieOptions
): string {
  let cookie = `${encodeURIComponent(name)}=${encodeURIComponent(value)}`;

  if (options?.maxAge !== undefined) {
    cookie += `; Max-Age=${options.maxAge}`;
  }

  if (options?.httpOnly) {
    cookie += '; HttpOnly';
  }

  if (options?.secure) {
    cookie += '; Secure';
  }

  if (options?.sameSite) {
    cookie += `; SameSite=${options.sameSite}`;
  }

  if (options?.path) {
    cookie += `; Path=${options.path}`;
  }

  if (options?.domain) {
    cookie += `; Domain=${options.domain}`;
  }

  return cookie;
}

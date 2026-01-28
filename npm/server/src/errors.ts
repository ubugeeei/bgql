/**
 * Server-side error handling.
 */

import type { GraphQLError } from 'graphql';

/**
 * Base bgql server error.
 */
export class BgqlServerError extends Error {
  readonly code: string;
  readonly extensions: Record<string, unknown>;

  constructor(
    message: string,
    code: string,
    extensions: Record<string, unknown> = {}
  ) {
    super(message);
    this.name = 'BgqlServerError';
    this.code = code;
    this.extensions = extensions;
  }

  /**
   * Converts to a GraphQL-compatible error format.
   */
  toGraphQL(): { message: string; extensions: Record<string, unknown> } {
    return {
      message: this.message,
      extensions: {
        code: this.code,
        ...this.extensions,
      },
    };
  }
}

/**
 * Authentication required error.
 */
export class AuthenticationRequiredError extends BgqlServerError {
  constructor(message = 'Authentication required') {
    super(message, 'UNAUTHENTICATED');
    this.name = 'AuthenticationRequiredError';
  }
}

/**
 * Authorization failed error.
 */
export class ForbiddenError extends BgqlServerError {
  constructor(message = 'Forbidden', requiredPermission?: string) {
    super(message, 'FORBIDDEN', requiredPermission ? { requiredPermission } : {});
    this.name = 'ForbiddenError';
  }
}

/**
 * Input validation error.
 */
export class InputValidationError extends BgqlServerError {
  readonly field: string;
  readonly constraint: string;

  constructor(field: string, constraint: string, message?: string) {
    super(
      message ?? `Validation failed for '${field}': ${constraint}`,
      'BAD_USER_INPUT',
      { field, constraint }
    );
    this.name = 'InputValidationError';
    this.field = field;
    this.constraint = constraint;
  }
}

/**
 * Resource not found error.
 */
export class NotFoundError extends BgqlServerError {
  readonly resourceType: string;
  readonly resourceId: string;

  constructor(resourceType: string, resourceId: string) {
    super(`${resourceType} not found: ${resourceId}`, 'NOT_FOUND', {
      resourceType,
      resourceId,
    });
    this.name = 'NotFoundError';
    this.resourceType = resourceType;
    this.resourceId = resourceId;
  }
}

/**
 * Rate limit exceeded error.
 */
export class RateLimitError extends BgqlServerError {
  readonly retryAfter: number;

  constructor(retryAfter: number, message?: string) {
    super(
      message ?? `Rate limit exceeded. Retry after ${retryAfter}ms`,
      'RATE_LIMITED',
      { retryAfter }
    );
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
  }
}

/**
 * Internal server error.
 */
export class InternalServerError extends BgqlServerError {
  readonly originalError?: Error;

  constructor(message = 'Internal server error', originalError?: Error) {
    super(message, 'INTERNAL_SERVER_ERROR');
    this.name = 'InternalServerError';
    this.originalError = originalError;
  }
}

/**
 * Formats an error for GraphQL response.
 */
export function formatError(
  error: GraphQLError
): { message: string; extensions?: Record<string, unknown> } {
  const originalError = error.originalError;

  // Handle bgql errors
  if (originalError instanceof BgqlServerError) {
    return originalError.toGraphQL();
  }

  // Handle standard errors
  if (originalError) {
    // In production, don't leak internal error details
    if (process.env.NODE_ENV === 'production') {
      return {
        message: 'An unexpected error occurred',
        extensions: { code: 'INTERNAL_SERVER_ERROR' },
      };
    }

    return {
      message: originalError.message,
      extensions: {
        code: 'INTERNAL_SERVER_ERROR',
        stack: originalError.stack,
      },
    };
  }

  // GraphQL validation/parsing errors
  return {
    message: error.message,
    extensions: {
      code: 'GRAPHQL_ERROR',
      locations: error.locations,
      path: error.path,
    },
  };
}

/**
 * Wraps a resolver to catch and format errors.
 */
export function wrapResolver<TParent, TArgs, TContext, TResult>(
  resolver: (
    parent: TParent,
    args: TArgs,
    context: TContext,
    info: unknown
  ) => TResult | Promise<TResult>
): (
  parent: TParent,
  args: TArgs,
  context: TContext,
  info: unknown
) => Promise<TResult> {
  return async (parent, args, context, info) => {
    try {
      return await resolver(parent, args, context, info);
    } catch (error) {
      // Re-throw bgql errors as-is
      if (error instanceof BgqlServerError) {
        throw error;
      }

      // Wrap unknown errors
      if (error instanceof Error) {
        throw new InternalServerError(error.message, error);
      }

      throw new InternalServerError(String(error));
    }
  };
}

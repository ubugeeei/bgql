/**
 * Domain Errors
 *
 * Typed error classes for domain-specific failures.
 * These are converted to GraphQL error types in the presentation layer.
 */

import { UserId, PostId, UserRole } from "./entities.js";

// ============================================
// Base Error
// ============================================

export abstract class DomainError extends Error {
  abstract readonly code: string;
  abstract readonly __typename: string;

  constructor(message: string) {
    super(message);
    this.name = this.constructor.name;
  }

  toGraphQL(): Record<string, unknown> {
    return {
      __typename: this.__typename,
      message: this.message,
      code: this.code,
    };
  }
}

// ============================================
// Not Found Errors
// ============================================

export class NotFoundError extends DomainError {
  readonly code = "NOT_FOUND";
  readonly __typename = "NotFoundError";

  constructor(
    readonly resourceType: string,
    readonly resourceId: string
  ) {
    super(`${resourceType} with id "${resourceId}" not found`);
  }

  toGraphQL() {
    return {
      ...super.toGraphQL(),
      resourceType: this.resourceType,
      resourceId: this.resourceId,
    };
  }
}

export class UserNotFoundError extends NotFoundError {
  constructor(id: UserId) {
    super("User", id);
  }
}

export class PostNotFoundError extends NotFoundError {
  constructor(id: PostId) {
    super("Post", id);
  }
}

// ============================================
// Validation Errors
// ============================================

export class ValidationError extends DomainError {
  readonly code = "VALIDATION_ERROR";
  readonly __typename = "ValidationError";

  constructor(
    readonly field: string,
    readonly constraint: string,
    message?: string
  ) {
    super(message ?? `${field}: validation failed (${constraint})`);
  }

  toGraphQL() {
    return {
      ...super.toGraphQL(),
      field: this.field,
      constraint: this.constraint,
    };
  }
}

export class EmailFormatError extends ValidationError {
  constructor() {
    super("email", "email", "email must be a valid email address");
  }
}

export class MinLengthError extends ValidationError {
  constructor(field: string, min: number) {
    super(field, `minLength(${min})`, `${field} must be at least ${min} characters`);
  }
}

export class MaxLengthError extends ValidationError {
  constructor(field: string, max: number) {
    super(field, `maxLength(${max})`, `${field} must be at most ${max} characters`);
  }
}

export class UniqueConstraintError extends ValidationError {
  constructor(field: string) {
    super(field, "unique", `${field} already exists`);
  }
}

// ============================================
// Auth Errors
// ============================================

export class UnauthorizedError extends DomainError {
  readonly code = "UNAUTHORIZED";
  readonly __typename = "UnauthorizedError";
  readonly requiredRole: UserRole | null;

  constructor(message = "Authentication required", requiredRole: UserRole | null = null) {
    super(message);
    this.requiredRole = requiredRole;
  }

  toGraphQL() {
    return {
      ...super.toGraphQL(),
      requiredRole: this.requiredRole,
    };
  }
}

export class ForbiddenError extends DomainError {
  readonly code = "FORBIDDEN";
  readonly __typename = "ForbiddenError";

  constructor(
    readonly reason: string
  ) {
    super(`Access denied: ${reason}`);
  }

  toGraphQL() {
    return {
      ...super.toGraphQL(),
      reason: this.reason,
    };
  }
}

export class InvalidCredentialsError extends DomainError {
  readonly code = "INVALID_CREDENTIALS";
  readonly __typename = "InvalidCredentialsError";

  constructor() {
    super("Invalid email or password");
  }
}

// ============================================
// Rate Limit Error
// ============================================

export class RateLimitError extends DomainError {
  readonly code = "RATE_LIMITED";
  readonly __typename = "RateLimitError";

  constructor(
    readonly retryAfter: number
  ) {
    super(`Rate limit exceeded. Retry after ${retryAfter} seconds.`);
  }

  toGraphQL() {
    return {
      ...super.toGraphQL(),
      retryAfter: this.retryAfter,
    };
  }
}

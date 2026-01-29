/**
 * GraphQL Context
 *
 * Request-scoped context containing all services and loaders.
 * Created fresh for each request to ensure proper isolation.
 */

import type { BaseContext } from "@bgql/server";
import { User, UserId, UserRole } from "../domain/entities.js";
import {
  UserRepository,
  PostRepository,
  CommentRepository,
  InMemoryUserRepository,
  InMemoryPostRepository,
  InMemoryCommentRepository,
} from "../infrastructure/index.js";
import { Loaders, createLoaders } from "../infrastructure/loaders.js";
import {
  UserQueryService,
  UserCommandService,
  PostQueryService,
  PostCommandService,
} from "../application/index.js";

// ============================================
// Database Container (shared across requests)
// ============================================

export interface Database {
  readonly users: UserRepository;
  readonly posts: PostRepository;
  readonly comments: CommentRepository;
}

export function createDatabase(): Database {
  return {
    users: new InMemoryUserRepository(),
    posts: new InMemoryPostRepository(),
    comments: new InMemoryCommentRepository(),
  };
}

// ============================================
// Services Container
// ============================================

export interface Services {
  readonly userQuery: UserQueryService;
  readonly userCommand: UserCommandService;
  readonly postQuery: PostQueryService;
  readonly postCommand: PostCommandService;
}

function createServices(db: Database): Services {
  return {
    userQuery: new UserQueryService(db.users),
    userCommand: new UserCommandService(db.users),
    postQuery: new PostQueryService(db.posts),
    postCommand: new PostCommandService(db.posts, db.users),
  };
}

// ============================================
// GraphQL Context
// ============================================

export interface Context extends BaseContext {
  /** Current authenticated user (null if not authenticated) */
  readonly currentUser: User | null;
  /** DataLoaders for batch loading */
  readonly loaders: Loaders;
  /** Application services */
  readonly services: Services;
  /** Direct repository access (for resolvers that need it) */
  readonly db: Database;
}

export interface ContextOptions {
  readonly db: Database;
  readonly currentUser: User | null;
  readonly baseContext: BaseContext;
}

/**
 * Creates a new context for each request.
 * Loaders are created per-request to ensure proper batching and caching isolation.
 */
export function createContext(options: ContextOptions): Context {
  const { db, currentUser, baseContext } = options;

  return {
    ...baseContext,
    currentUser,
    loaders: createLoaders(db.users, db.posts, db.comments),
    services: createServices(db),
    db,
  };
}

// ============================================
// Auth Helpers
// ============================================

/**
 * Extracts user from authorization header.
 * In a real app, this would verify JWT tokens.
 */
export function extractUserFromHeader(
  authHeader: string | null,
  db: Database
): User | null {
  if (!authHeader) return null;

  // Simple token format: "Bearer user_X" where X is the user ID
  // In production, use proper JWT verification
  const match = authHeader.match(/^Bearer\s+(.+)$/);
  if (!match) return null;

  const token = match[1];

  // For demo purposes, we treat the token as a user ID
  // Real implementation would decode/verify JWT
  const userId = UserId(token);

  // Synchronous lookup for demo (in production, this would be async with caching)
  // Note: This is simplified - real auth would use async token verification
  return null; // Will be populated by middleware
}

export function requireAuth(ctx: Context): User {
  if (!ctx.currentUser) {
    throw new Error("Authentication required");
  }
  return ctx.currentUser;
}

export function requireRole(ctx: Context, ...roles: UserRole[]): User {
  const user = requireAuth(ctx);
  if (!roles.includes(user.role)) {
    throw new Error(`Required role: ${roles.join(" or ")}`);
  }
  return user;
}

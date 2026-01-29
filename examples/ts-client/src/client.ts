/**
 * BGQL Type-Safe Client
 *
 * Uses @bgql/client SDK for proper Result-based error handling.
 * Re-exports SDK functionality with typed operations for this schema.
 */

import { createClient as createBgqlClient, gql } from "@bgql/client";
import type { BgqlClient } from "@bgql/client";
import {
  UserId, PostId,
  User, Post, UserAnalytics,
  UserResult, CreateUserResult, CreatePostResult, AuthResult,
  Connection,
  CreateUserInput, CreatePostInput, LoginCredentials,
} from "./types.js";

// Re-export types from SDK
export type { BgqlClient };

// Client configuration
export interface ClientConfig {
  endpoint: string;
  headers?: Record<string, string>;
}

/**
 * Typed wrapper around the BGQL client for this schema.
 *
 * Provides typed methods that return discriminated unions
 * instead of throwing exceptions.
 */
export class TypedBgqlClient {
  private client: BgqlClient;

  constructor(config: ClientConfig) {
    this.client = createBgqlClient({
      url: config.endpoint,
      headers: config.headers,
    });
  }

  /**
   * Set the authentication token.
   */
  setToken(token: string): void {
    this.client.setAuthToken(token);
  }

  /**
   * Clear the authentication token.
   */
  clearToken(): void {
    this.client.setAuthToken(null);
  }

  // ============================================
  // Query Methods
  // ============================================

  /**
   * Get the current authenticated user.
   */
  async me(signal?: AbortSignal): Promise<User | null> {
    const result = await this.client.query<{ me: User | null }>(
      `query Me {
        me {
          id name email bio avatarUrl role createdAt updatedAt
        }
      }`,
      undefined,
      { signal }
    );

    if (result.ok) {
      return result.value.me;
    }
    // Return null on error (not authenticated)
    return null;
  }

  /**
   * Get a user by ID.
   *
   * Returns a typed union: User | NotFoundError | UnauthorizedError
   * Handle all cases explicitly - no exceptions thrown.
   */
  async getUser(id: UserId, signal?: AbortSignal): Promise<UserResult> {
    const result = await this.client.query<{ user: UserResult }>(
      `query GetUser($id: UserId!) {
        user(id: $id) {
          ... on User {
            __typename id name email bio avatarUrl role createdAt updatedAt
          }
          ... on NotFoundError {
            __typename message code resourceType resourceId
          }
          ... on UnauthorizedError {
            __typename message code requiredRole
          }
        }
      }`,
      { id },
      { signal }
    );

    if (result.ok) {
      return result.value.user;
    }

    // Convert SDK error to typed error
    return {
      __typename: "NotFoundError",
      message: result.error.message,
      code: "CLIENT_ERROR",
      resourceType: "User",
      resourceId: id,
    };
  }

  /**
   * Get a user with their analytics (supports @defer).
   */
  async getUserWithAnalytics(
    id: UserId,
    signal?: AbortSignal
  ): Promise<{ user: UserResult; analytics?: UserAnalytics }> {
    const result = await this.client.query<{
      user: UserResult & { analytics?: UserAnalytics }
    }>(
      `query GetUserWithAnalytics($id: UserId!) {
        user(id: $id) {
          ... on User {
            __typename id name email
            analytics {
              totalPosts totalComments totalLikes
            }
          }
          ... on NotFoundError {
            __typename message code
          }
        }
      }`,
      { id },
      { signal }
    );

    if (result.ok && result.value.user.__typename === "User") {
      return {
        user: result.value.user,
        analytics: result.value.user.analytics,
      };
    }

    if (result.ok) {
      return { user: result.value.user };
    }

    return {
      user: {
        __typename: "NotFoundError",
        message: result.error.message,
        code: "CLIENT_ERROR",
        resourceType: "User",
        resourceId: id,
      },
    };
  }

  /**
   * List users with pagination.
   */
  async listUsers(
    options?: {
      first?: number;
      after?: string | null;
    },
    signal?: AbortSignal
  ): Promise<Connection<Omit<User, "__typename">>> {
    const result = await this.client.query<{
      users: Connection<Omit<User, "__typename">>
    }>(
      `query ListUsers($first: Int, $after: String) {
        users(first: $first, after: $after) {
          edges {
            cursor
            node { id name email role createdAt }
          }
          pageInfo {
            hasNextPage hasPreviousPage startCursor endCursor
          }
          totalCount
        }
      }`,
      options,
      { signal }
    );

    if (result.ok) {
      return result.value.users;
    }

    // Return empty connection on error
    return {
      edges: [],
      pageInfo: {
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: null,
        endCursor: null,
      },
      totalCount: 0,
    };
  }

  /**
   * Get a post by ID.
   */
  async getPost(id: PostId, signal?: AbortSignal): Promise<Post | null> {
    const result = await this.client.query<{
      post: Post | { __typename: "NotFoundError" }
    }>(
      `query GetPost($id: PostId!) {
        post(id: $id) {
          ... on Post {
            __typename id title content authorId status publishedAt createdAt
          }
          ... on NotFoundError { __typename }
        }
      }`,
      { id },
      { signal }
    );

    if (result.ok && result.value.post.__typename === "Post") {
      return result.value.post;
    }
    return null;
  }

  /**
   * List posts with pagination.
   */
  async listPosts(
    options?: {
      first?: number;
      after?: string | null;
    },
    signal?: AbortSignal
  ): Promise<Connection<Omit<Post, "__typename">>> {
    const result = await this.client.query<{
      posts: Connection<Omit<Post, "__typename">>
    }>(
      `query ListPosts($first: Int, $after: String) {
        posts(first: $first, after: $after) {
          edges {
            cursor
            node { id title status authorId createdAt }
          }
          pageInfo { hasNextPage endCursor }
          totalCount
        }
      }`,
      options,
      { signal }
    );

    if (result.ok) {
      return result.value.posts;
    }

    return {
      edges: [],
      pageInfo: {
        hasNextPage: false,
        hasPreviousPage: false,
        startCursor: null,
        endCursor: null,
      },
      totalCount: 0,
    };
  }

  // ============================================
  // Mutation Methods
  // ============================================

  /**
   * Create a new user.
   *
   * Returns: User | ValidationError
   * Validation errors are part of the return type, not exceptions.
   */
  async createUser(input: CreateUserInput, signal?: AbortSignal): Promise<CreateUserResult> {
    const result = await this.client.mutate<{ createUser: CreateUserResult }>(
      `mutation CreateUser($input: CreateUserInput!) {
        createUser(input: $input) {
          ... on User {
            __typename id name email role createdAt
          }
          ... on ValidationError {
            __typename message code field constraint
          }
        }
      }`,
      { input },
      { signal }
    );

    if (result.ok) {
      return result.value.createUser;
    }

    return {
      __typename: "ValidationError",
      message: result.error.message,
      code: "CLIENT_ERROR",
      field: "unknown",
      constraint: "unknown",
    };
  }

  /**
   * Create a new post (requires authentication).
   *
   * Returns: Post | ValidationError | UnauthorizedError
   */
  async createPost(input: CreatePostInput, signal?: AbortSignal): Promise<CreatePostResult> {
    const result = await this.client.mutate<{ createPost: CreatePostResult }>(
      `mutation CreatePost($input: CreatePostInput!) {
        createPost(input: $input) {
          ... on Post {
            __typename id title content status createdAt
          }
          ... on ValidationError {
            __typename message code field constraint
          }
          ... on UnauthorizedError {
            __typename message code requiredRole
          }
        }
      }`,
      { input },
      { signal }
    );

    if (result.ok) {
      return result.value.createPost;
    }

    return {
      __typename: "UnauthorizedError",
      message: result.error.message,
      code: "CLIENT_ERROR",
      requiredRole: null,
    };
  }

  /**
   * Login with email/password or OAuth.
   *
   * Returns: AuthPayload | InvalidCredentialsError | ValidationError
   */
  async login(credentials: LoginCredentials, signal?: AbortSignal): Promise<AuthResult> {
    const result = await this.client.mutate<{ login: AuthResult }>(
      `mutation Login($credentials: LoginCredentialsInput!) {
        login(credentials: $credentials) {
          ... on AuthPayload {
            __typename token user { id name email role } expiresAt
          }
          ... on InvalidCredentialsError {
            __typename message code
          }
          ... on ValidationError {
            __typename message code field constraint
          }
        }
      }`,
      { credentials },
      { signal }
    );

    if (result.ok) {
      return result.value.login;
    }

    return {
      __typename: "InvalidCredentialsError",
      message: result.error.message,
      code: "CLIENT_ERROR",
    };
  }
}

/**
 * Create a new typed BGQL client.
 */
export function createClient(config: ClientConfig): TypedBgqlClient {
  return new TypedBgqlClient(config);
}

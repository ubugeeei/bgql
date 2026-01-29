/**
 * GraphQL Resolvers
 *
 * Maps GraphQL operations to application layer use cases.
 * Handles result types and converts domain errors to GraphQL responses.
 */

import type { Resolvers } from "@bgql/server";
import { Context, requireAuth, requireRole } from "./context.js";
import {
  User, Post, Comment, UserId, PostId, UserRole,
  PostStatus, UserAnalytics,
} from "../domain/entities.js";
import { DomainError } from "../domain/errors.js";

// ============================================
// Helper: Convert Result to Union Response
// ============================================

type ErrorResponse = { __typename: string; message: string; code: string; [key: string]: unknown };

function resultToUnion<T, E extends DomainError>(
  result: { ok: true; value: T } | { ok: false; error: E },
  successMapper: (value: T) => { __typename: string; [key: string]: unknown }
): { __typename: string; [key: string]: unknown } {
  if (result.ok) {
    return successMapper(result.value);
  }
  return result.error.toGraphQL();
}

// ============================================
// Query Resolvers
// ============================================

const queryResolvers = {
  user: async (
    _parent: unknown,
    args: { id: string },
    ctx: Context
  ) => {
    const result = await ctx.services.userQuery.getUser({ id: UserId(args.id) });
    return resultToUnion(result, (user) => ({
      __typename: "User",
      ...userToGraphQL(user),
    }));
  },

  users: async (
    _parent: unknown,
    args: { filter?: { role?: UserRole; search?: string } },
    ctx: Context
  ) => {
    const users = await ctx.services.userQuery.listUsers({ filter: args.filter });
    return users.map((user) => ({
      __typename: "User",
      ...userToGraphQL(user),
    }));
  },

  post: async (
    _parent: unknown,
    args: { id: string },
    ctx: Context
  ) => {
    const result = await ctx.services.postQuery.getPost({ id: PostId(args.id) });
    return resultToUnion(result, (post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },

  posts: async (
    _parent: unknown,
    args: { filter?: { status?: PostStatus; authorId?: string; search?: string } },
    ctx: Context
  ) => {
    const filter = args.filter ? {
      ...args.filter,
      authorId: args.filter.authorId ? UserId(args.filter.authorId) : undefined,
    } : undefined;

    const posts = await ctx.services.postQuery.listPosts({ filter });
    return posts.map((post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },

  me: async (_parent: unknown, _args: unknown, ctx: Context) => {
    if (!ctx.currentUser) {
      return {
        __typename: "UnauthorizedError",
        message: "Not authenticated",
        code: "UNAUTHORIZED",
        requiredRole: null,
      };
    }
    return {
      __typename: "User",
      ...userToGraphQL(ctx.currentUser),
    };
  },
};

// ============================================
// Mutation Resolvers
// ============================================

const mutationResolvers = {
  createUser: async (
    _parent: unknown,
    args: { input: { name: string; email: string; bio?: string | null; role?: UserRole } },
    ctx: Context
  ) => {
    const result = await ctx.services.userCommand.createUser({
      name: args.input.name,
      email: args.input.email,
      bio: args.input.bio,
      role: args.input.role,
    });
    return resultToUnion(result, (user) => ({
      __typename: "User",
      ...userToGraphQL(user),
    }));
  },

  updateUser: async (
    _parent: unknown,
    args: { id: string; input: { name?: string; email?: string; bio?: string | null; avatarUrl?: string | null } },
    ctx: Context
  ) => {
    const result = await ctx.services.userCommand.updateUser({
      id: UserId(args.id),
      ...args.input,
    });
    return resultToUnion(result, (user) => ({
      __typename: "User",
      ...userToGraphQL(user),
    }));
  },

  deleteUser: async (
    _parent: unknown,
    args: { id: string },
    ctx: Context
  ) => {
    const currentUser = requireAuth(ctx);
    const result = await ctx.services.userCommand.deleteUser({
      id: UserId(args.id),
      requesterId: currentUser.id,
      requesterRole: currentUser.role,
    });

    if (result.ok) {
      return { __typename: "DeleteSuccess", success: true };
    }
    return result.error.toGraphQL();
  },

  createPost: async (
    _parent: unknown,
    args: { input: { title: string; content: string; status?: PostStatus } },
    ctx: Context
  ) => {
    const currentUser = requireAuth(ctx);
    const result = await ctx.services.postCommand.createPost({
      title: args.input.title,
      content: args.input.content,
      authorId: currentUser.id,
      status: args.input.status,
    });
    return resultToUnion(result, (post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },

  updatePost: async (
    _parent: unknown,
    args: { id: string; input: { title?: string; content?: string; status?: PostStatus } },
    ctx: Context
  ) => {
    const currentUser = requireAuth(ctx);
    const result = await ctx.services.postCommand.updatePost({
      id: PostId(args.id),
      requesterId: currentUser.id,
      requesterRole: currentUser.role,
      ...args.input,
    });
    return resultToUnion(result, (post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },

  deletePost: async (
    _parent: unknown,
    args: { id: string },
    ctx: Context
  ) => {
    const currentUser = requireAuth(ctx);
    const result = await ctx.services.postCommand.deletePost({
      id: PostId(args.id),
      requesterId: currentUser.id,
      requesterRole: currentUser.role,
    });

    if (result.ok) {
      return { __typename: "DeleteSuccess", success: true };
    }
    return result.error.toGraphQL();
  },

  publishPost: async (
    _parent: unknown,
    args: { id: string },
    ctx: Context
  ) => {
    const currentUser = requireAuth(ctx);
    const result = await ctx.services.postCommand.publishPost({
      id: PostId(args.id),
      requesterId: currentUser.id,
      requesterRole: currentUser.role,
    });
    return resultToUnion(result, (post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },
};

// ============================================
// Type Resolvers (for nested fields)
// ============================================

const userTypeResolvers = {
  posts: async (parent: { id: string }, _args: unknown, ctx: Context) => {
    const posts = await ctx.loaders.postsByAuthor.load(UserId(parent.id));
    return posts.map((post) => ({
      __typename: "Post",
      ...postToGraphQL(post),
    }));
  },

  analytics: async (parent: { id: string }, _args: unknown, ctx: Context): Promise<UserAnalytics> => {
    return ctx.services.userQuery.getUserAnalytics(UserId(parent.id));
  },
};

const postTypeResolvers = {
  author: async (parent: { authorId: string }, _args: unknown, ctx: Context) => {
    const user = await ctx.loaders.user.load(UserId(parent.authorId));
    if (!user) {
      return null;
    }
    return {
      __typename: "User",
      ...userToGraphQL(user),
    };
  },

  comments: async (parent: { id: string }, _args: unknown, ctx: Context) => {
    const comments = await ctx.loaders.commentsByPost.load(PostId(parent.id));
    return comments.map((comment) => ({
      __typename: "Comment",
      ...commentToGraphQL(comment),
    }));
  },
};

const commentTypeResolvers = {
  author: async (parent: { authorId: string }, _args: unknown, ctx: Context) => {
    const user = await ctx.loaders.user.load(UserId(parent.authorId));
    if (!user) {
      return null;
    }
    return {
      __typename: "User",
      ...userToGraphQL(user),
    };
  },

  post: async (parent: { postId: string }, _args: unknown, ctx: Context) => {
    const post = await ctx.loaders.post.load(PostId(parent.postId));
    if (!post) {
      return null;
    }
    return {
      __typename: "Post",
      ...postToGraphQL(post),
    };
  },
};

// ============================================
// Domain to GraphQL Mappers
// ============================================

function userToGraphQL(user: User) {
  return {
    id: user.id,
    name: user.name,
    email: user.email,
    bio: user.bio,
    avatarUrl: user.avatarUrl,
    role: user.role,
    createdAt: user.createdAt.toISOString(),
    updatedAt: user.updatedAt?.toISOString() ?? null,
  };
}

function postToGraphQL(post: Post) {
  return {
    id: post.id,
    title: post.title,
    content: post.content,
    authorId: post.authorId,
    status: post.status,
    publishedAt: post.publishedAt?.toISOString() ?? null,
    createdAt: post.createdAt.toISOString(),
  };
}

function commentToGraphQL(comment: Comment) {
  return {
    id: comment.id,
    content: comment.content,
    authorId: comment.authorId,
    postId: comment.postId,
    createdAt: comment.createdAt.toISOString(),
  };
}

// ============================================
// Export Combined Resolvers
// ============================================

export const resolvers: Resolvers<Context> = {
  Query: queryResolvers,
  Mutation: mutationResolvers,
  User: userTypeResolvers,
  Post: postTypeResolvers,
  Comment: commentTypeResolvers,
};

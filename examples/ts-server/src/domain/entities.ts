/**
 * Domain Entities
 *
 * Pure domain objects without infrastructure concerns.
 * These represent the core business concepts.
 */

// ============================================
// Value Objects (Branded types for type safety)
// ============================================

export type UserId = string & { readonly __brand: "UserId" };
export type PostId = string & { readonly __brand: "PostId" };
export type CommentId = string & { readonly __brand: "CommentId" };
export type Email = string & { readonly __brand: "Email" };

export const UserId = (id: string): UserId => id as UserId;
export const PostId = (id: string): PostId => id as PostId;
export const CommentId = (id: string): CommentId => id as CommentId;
export const Email = (email: string): Email => email.toLowerCase().trim() as Email;

// ============================================
// Enums
// ============================================

export type UserRole = "Admin" | "Moderator" | "User" | "Guest";
export type PostStatus = "Draft" | "Published" | "Archived";
export type OAuthProvider = "Google" | "GitHub" | "Apple";

// ============================================
// Domain Entities
// ============================================

export interface User {
  readonly id: UserId;
  readonly name: string;
  readonly email: Email;
  readonly bio: string | null;
  readonly avatarUrl: string | null;
  readonly role: UserRole;
  readonly createdAt: Date;
  readonly updatedAt: Date | null;
}

export interface Post {
  readonly id: PostId;
  readonly title: string;
  readonly content: string;
  readonly authorId: UserId;
  readonly status: PostStatus;
  readonly publishedAt: Date | null;
  readonly createdAt: Date;
}

export interface Comment {
  readonly id: CommentId;
  readonly content: string;
  readonly authorId: UserId;
  readonly postId: PostId;
  readonly createdAt: Date;
}

// ============================================
// Aggregate types
// ============================================

export interface UserWithPosts extends User {
  readonly posts: readonly Post[];
}

export interface PostWithAuthor extends Post {
  readonly author: User;
}

export interface UserAnalytics {
  readonly totalPosts: number;
  readonly totalComments: number;
  readonly totalLikes: number;
}

// ============================================
// Auth types
// ============================================

export interface AuthToken {
  readonly token: string;
  readonly userId: UserId;
  readonly expiresAt: Date;
}

export interface Session {
  readonly user: User;
  readonly token: AuthToken;
}

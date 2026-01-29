/**
 * Repository Interfaces & Implementations
 *
 * Repositories abstract data access from the domain.
 * They return domain entities and use Result types for errors.
 */

import { Result, ok, err } from "@bgql/client";
import {
  User, Post, Comment, UserAnalytics,
  UserId, PostId, Email,
  UserRole, PostStatus,
} from "../domain/entities.js";
import {
  DomainError,
  UserNotFoundError,
  PostNotFoundError,
  UniqueConstraintError,
} from "../domain/errors.js";

// ============================================
// Repository Interfaces
// ============================================

export interface UserRepository {
  findById(id: UserId): Promise<Result<User, UserNotFoundError>>;
  findByEmail(email: Email): Promise<Result<User, UserNotFoundError>>;
  findMany(ids: readonly UserId[]): Promise<Map<UserId, User>>;
  findAll(filter?: UserFilter): Promise<User[]>;
  create(input: CreateUserData): Promise<Result<User, UniqueConstraintError>>;
  update(id: UserId, input: UpdateUserData): Promise<Result<User, UserNotFoundError>>;
  delete(id: UserId): Promise<Result<void, UserNotFoundError>>;
  getAnalytics(id: UserId): Promise<UserAnalytics>;
}

export interface PostRepository {
  findById(id: PostId): Promise<Result<Post, PostNotFoundError>>;
  findMany(ids: readonly PostId[]): Promise<Map<PostId, Post>>;
  findAll(filter?: PostFilter): Promise<Post[]>;
  findByAuthor(authorId: UserId): Promise<Post[]>;
  findManyByAuthors(authorIds: readonly UserId[]): Promise<Map<UserId, Post[]>>;
  create(input: CreatePostData): Promise<Result<Post, DomainError>>;
  update(id: PostId, input: UpdatePostData): Promise<Result<Post, PostNotFoundError>>;
  delete(id: PostId): Promise<Result<void, PostNotFoundError>>;
}

export interface CommentRepository {
  findByPost(postId: PostId): Promise<Comment[]>;
  findManyByPosts(postIds: readonly PostId[]): Promise<Map<PostId, Comment[]>>;
}

// ============================================
// Filter Types
// ============================================

export interface UserFilter {
  readonly role?: UserRole;
  readonly search?: string;
}

export interface PostFilter {
  readonly status?: PostStatus;
  readonly authorId?: UserId;
  readonly search?: string;
}

// ============================================
// Input Types
// ============================================

export interface CreateUserData {
  readonly name: string;
  readonly email: Email;
  readonly bio?: string | null;
  readonly role?: UserRole;
}

export interface UpdateUserData {
  readonly name?: string;
  readonly email?: Email;
  readonly bio?: string | null;
  readonly avatarUrl?: string | null;
}

export interface CreatePostData {
  readonly title: string;
  readonly content: string;
  readonly authorId: UserId;
  readonly status?: PostStatus;
}

export interface UpdatePostData {
  readonly title?: string;
  readonly content?: string;
  readonly status?: PostStatus;
}

// ============================================
// In-Memory Implementation
// ============================================

export class InMemoryUserRepository implements UserRepository {
  private users = new Map<string, User>();
  private emailIndex = new Map<string, UserId>();

  constructor() {
    this.seed();
  }

  private seed(): void {
    const users: User[] = [
      {
        id: UserId("user_1"),
        name: "Alice Johnson",
        email: Email("alice@example.com"),
        bio: "Software engineer and blogger",
        avatarUrl: "https://example.com/avatars/alice.jpg",
        role: "Admin",
        createdAt: new Date("2024-01-01"),
        updatedAt: new Date("2024-06-15"),
      },
      {
        id: UserId("user_2"),
        name: "Bob Smith",
        email: Email("bob@example.com"),
        bio: "Tech enthusiast",
        avatarUrl: null,
        role: "Moderator",
        createdAt: new Date("2024-02-15"),
        updatedAt: null,
      },
      {
        id: UserId("user_3"),
        name: "Carol Williams",
        email: Email("carol@example.com"),
        bio: null,
        avatarUrl: "https://example.com/avatars/carol.jpg",
        role: "User",
        createdAt: new Date("2024-03-20"),
        updatedAt: null,
      },
    ];

    for (const user of users) {
      this.users.set(user.id, user);
      this.emailIndex.set(user.email, user.id);
    }
  }

  async findById(id: UserId): Promise<Result<User, UserNotFoundError>> {
    const user = this.users.get(id);
    return user ? ok(user) : err(new UserNotFoundError(id));
  }

  async findByEmail(email: Email): Promise<Result<User, UserNotFoundError>> {
    const id = this.emailIndex.get(email);
    if (!id) return err(new UserNotFoundError(email as unknown as UserId));
    return this.findById(id);
  }

  async findMany(ids: readonly UserId[]): Promise<Map<UserId, User>> {
    const result = new Map<UserId, User>();
    for (const id of ids) {
      const user = this.users.get(id);
      if (user) result.set(id, user);
    }
    return result;
  }

  async findAll(filter?: UserFilter): Promise<User[]> {
    let users = Array.from(this.users.values());

    if (filter?.role) {
      users = users.filter(u => u.role === filter.role);
    }
    if (filter?.search) {
      const s = filter.search.toLowerCase();
      users = users.filter(u =>
        u.name.toLowerCase().includes(s) ||
        u.email.toLowerCase().includes(s)
      );
    }

    return users;
  }

  async create(input: CreateUserData): Promise<Result<User, UniqueConstraintError>> {
    if (this.emailIndex.has(input.email)) {
      return err(new UniqueConstraintError("email"));
    }

    const user: User = {
      id: UserId(`user_${this.users.size + 1}`),
      name: input.name,
      email: input.email,
      bio: input.bio ?? null,
      avatarUrl: null,
      role: input.role ?? "User",
      createdAt: new Date(),
      updatedAt: null,
    };

    this.users.set(user.id, user);
    this.emailIndex.set(user.email, user.id);
    return ok(user);
  }

  async update(id: UserId, input: UpdateUserData): Promise<Result<User, UserNotFoundError>> {
    const existing = this.users.get(id);
    if (!existing) return err(new UserNotFoundError(id));

    const updated: User = {
      ...existing,
      ...input,
      updatedAt: new Date(),
    };

    this.users.set(id, updated);
    return ok(updated);
  }

  async delete(id: UserId): Promise<Result<void, UserNotFoundError>> {
    const user = this.users.get(id);
    if (!user) return err(new UserNotFoundError(id));

    this.users.delete(id);
    this.emailIndex.delete(user.email);
    return ok(undefined);
  }

  async getAnalytics(id: UserId): Promise<UserAnalytics> {
    // In real app, this would query the database
    return {
      totalPosts: Math.floor(Math.random() * 10),
      totalComments: Math.floor(Math.random() * 50),
      totalLikes: Math.floor(Math.random() * 100),
    };
  }
}

export class InMemoryPostRepository implements PostRepository {
  private posts = new Map<string, Post>();

  constructor() {
    this.seed();
  }

  private seed(): void {
    const posts: Post[] = [
      {
        id: PostId("post_1"),
        title: "Introduction to BGQL",
        content: "Better GraphQL is a superset of GraphQL...",
        authorId: UserId("user_1"),
        status: "Published",
        publishedAt: new Date("2024-01-15"),
        createdAt: new Date("2024-01-10"),
      },
      {
        id: PostId("post_2"),
        title: "Schema-First Development",
        content: "The schema is the source of truth...",
        authorId: UserId("user_1"),
        status: "Published",
        publishedAt: new Date("2024-02-01"),
        createdAt: new Date("2024-01-28"),
      },
      {
        id: PostId("post_3"),
        title: "DataLoaders Explained",
        content: "The DataLoader pattern helps prevent N+1...",
        authorId: UserId("user_2"),
        status: "Published",
        publishedAt: new Date("2024-03-10"),
        createdAt: new Date("2024-03-05"),
      },
      {
        id: PostId("post_4"),
        title: "Draft: Typed Errors",
        content: "Error handling in BGQL uses union types...",
        authorId: UserId("user_1"),
        status: "Draft",
        publishedAt: null,
        createdAt: new Date("2024-04-01"),
      },
      {
        id: PostId("post_5"),
        title: "TypeScript and GraphQL",
        content: "TypeScript provides excellent type safety...",
        authorId: UserId("user_3"),
        status: "Published",
        publishedAt: new Date("2024-04-15"),
        createdAt: new Date("2024-04-10"),
      },
    ];

    for (const post of posts) {
      this.posts.set(post.id, post);
    }
  }

  async findById(id: PostId): Promise<Result<Post, PostNotFoundError>> {
    const post = this.posts.get(id);
    return post ? ok(post) : err(new PostNotFoundError(id));
  }

  async findMany(ids: readonly PostId[]): Promise<Map<PostId, Post>> {
    const result = new Map<PostId, Post>();
    for (const id of ids) {
      const post = this.posts.get(id);
      if (post) result.set(id, post);
    }
    return result;
  }

  async findAll(filter?: PostFilter): Promise<Post[]> {
    let posts = Array.from(this.posts.values());

    if (filter?.status) {
      posts = posts.filter(p => p.status === filter.status);
    }
    if (filter?.authorId) {
      posts = posts.filter(p => p.authorId === filter.authorId);
    }
    if (filter?.search) {
      const s = filter.search.toLowerCase();
      posts = posts.filter(p =>
        p.title.toLowerCase().includes(s) ||
        p.content.toLowerCase().includes(s)
      );
    }

    return posts.sort((a, b) => b.createdAt.getTime() - a.createdAt.getTime());
  }

  async findByAuthor(authorId: UserId): Promise<Post[]> {
    return Array.from(this.posts.values()).filter(p => p.authorId === authorId);
  }

  async findManyByAuthors(authorIds: readonly UserId[]): Promise<Map<UserId, Post[]>> {
    const result = new Map<UserId, Post[]>();
    const allPosts = Array.from(this.posts.values());

    for (const authorId of authorIds) {
      result.set(authorId, allPosts.filter(p => p.authorId === authorId));
    }

    return result;
  }

  async create(input: CreatePostData): Promise<Result<Post, DomainError>> {
    const post: Post = {
      id: PostId(`post_${this.posts.size + 1}`),
      title: input.title,
      content: input.content,
      authorId: input.authorId,
      status: input.status ?? "Draft",
      publishedAt: input.status === "Published" ? new Date() : null,
      createdAt: new Date(),
    };

    this.posts.set(post.id, post);
    return ok(post);
  }

  async update(id: PostId, input: UpdatePostData): Promise<Result<Post, PostNotFoundError>> {
    const existing = this.posts.get(id);
    if (!existing) return err(new PostNotFoundError(id));

    const updated: Post = {
      ...existing,
      ...input,
      publishedAt: input.status === "Published" && !existing.publishedAt
        ? new Date()
        : existing.publishedAt,
    };

    this.posts.set(id, updated);
    return ok(updated);
  }

  async delete(id: PostId): Promise<Result<void, PostNotFoundError>> {
    if (!this.posts.has(id)) return err(new PostNotFoundError(id));
    this.posts.delete(id);
    return ok(undefined);
  }
}

export class InMemoryCommentRepository implements CommentRepository {
  private comments = new Map<string, Comment>();

  constructor() {
    this.seed();
  }

  private seed(): void {
    const comments: Comment[] = [
      {
        id: CommentId("comment_1"),
        content: "Great introduction!",
        authorId: UserId("user_2"),
        postId: PostId("post_1"),
        createdAt: new Date("2024-01-16"),
      },
      {
        id: CommentId("comment_2"),
        content: "Very helpful, thanks!",
        authorId: UserId("user_3"),
        postId: PostId("post_1"),
        createdAt: new Date("2024-01-17"),
      },
    ];

    for (const comment of comments) {
      this.comments.set(comment.id, comment);
    }
  }

  async findByPost(postId: PostId): Promise<Comment[]> {
    return Array.from(this.comments.values())
      .filter(c => c.postId === postId)
      .sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
  }

  async findManyByPosts(postIds: readonly PostId[]): Promise<Map<PostId, Comment[]>> {
    const result = new Map<PostId, Comment[]>();
    const allComments = Array.from(this.comments.values());

    for (const postId of postIds) {
      result.set(postId, allComments.filter(c => c.postId === postId));
    }

    return result;
  }
}

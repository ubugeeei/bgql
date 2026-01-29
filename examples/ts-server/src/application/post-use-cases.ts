/**
 * Post Use Cases
 *
 * Application-level business logic for post operations.
 * Uses Result types for railway-oriented error handling.
 */

import { Result, ok, err } from "@bgql/client";
import {
  Post, PostId, UserId, PostStatus,
} from "../domain/entities.js";
import {
  DomainError,
  PostNotFoundError,
  UserNotFoundError,
  ValidationError,
  UnauthorizedError,
  ForbiddenError,
  MinLengthError,
  MaxLengthError,
} from "../domain/errors.js";
import {
  PostRepository,
  UserRepository,
  CreatePostData,
  UpdatePostData,
  PostFilter,
} from "../infrastructure/repositories.js";

// ============================================
// Input Validation
// ============================================

function validateTitle(title: string): Result<string, ValidationError> {
  const trimmed = title.trim();
  if (trimmed.length < 1) {
    return err(new MinLengthError("title", 1));
  }
  if (trimmed.length > 200) {
    return err(new MaxLengthError("title", 200));
  }
  return ok(trimmed);
}

function validateContent(content: string): Result<string, ValidationError> {
  const trimmed = content.trim();
  if (trimmed.length < 1) {
    return err(new MinLengthError("content", 1));
  }
  return ok(trimmed);
}

// ============================================
// Query Use Cases
// ============================================

export interface GetPostQuery {
  readonly id: PostId;
}

export interface ListPostsQuery {
  readonly filter?: PostFilter;
}

export interface GetPostsByAuthorQuery {
  readonly authorId: UserId;
}

export class PostQueryService {
  constructor(private readonly postRepo: PostRepository) {}

  async getPost(query: GetPostQuery): Promise<Result<Post, PostNotFoundError>> {
    return this.postRepo.findById(query.id);
  }

  async listPosts(query: ListPostsQuery): Promise<Post[]> {
    return this.postRepo.findAll(query.filter);
  }

  async getPostsByAuthor(query: GetPostsByAuthorQuery): Promise<Post[]> {
    return this.postRepo.findByAuthor(query.authorId);
  }
}

// ============================================
// Command Use Cases
// ============================================

export interface CreatePostCommand {
  readonly title: string;
  readonly content: string;
  readonly authorId: UserId;
  readonly status?: PostStatus;
}

export interface UpdatePostCommand {
  readonly id: PostId;
  readonly requesterId: UserId;
  readonly requesterRole: string;
  readonly title?: string;
  readonly content?: string;
  readonly status?: PostStatus;
}

export interface DeletePostCommand {
  readonly id: PostId;
  readonly requesterId: UserId;
  readonly requesterRole: string;
}

export interface PublishPostCommand {
  readonly id: PostId;
  readonly requesterId: UserId;
  readonly requesterRole: string;
}

export class PostCommandService {
  constructor(
    private readonly postRepo: PostRepository,
    private readonly userRepo: UserRepository
  ) {}

  async createPost(command: CreatePostCommand): Promise<Result<Post, ValidationError | UserNotFoundError | DomainError>> {
    // Validate author exists
    const authorResult = await this.userRepo.findById(command.authorId);
    if (!authorResult.ok) {
      return err(authorResult.error);
    }

    // Validate title
    const titleResult = validateTitle(command.title);
    if (!titleResult.ok) {
      return err(titleResult.error);
    }

    // Validate content
    const contentResult = validateContent(command.content);
    if (!contentResult.ok) {
      return err(contentResult.error);
    }

    const data: CreatePostData = {
      title: titleResult.value,
      content: contentResult.value,
      authorId: command.authorId,
      status: command.status,
    };

    return this.postRepo.create(data);
  }

  async updatePost(command: UpdatePostCommand): Promise<Result<Post, PostNotFoundError | ValidationError | ForbiddenError>> {
    // Get existing post
    const postResult = await this.postRepo.findById(command.id);
    if (!postResult.ok) {
      return err(postResult.error);
    }

    const post = postResult.value;

    // Authorization: only author or Admin/Moderator can update
    if (post.authorId !== command.requesterId &&
        command.requesterRole !== "Admin" &&
        command.requesterRole !== "Moderator") {
      return err(new ForbiddenError("Only the author can edit this post"));
    }

    // Validate title if provided
    let validatedTitle: string | undefined;
    if (command.title !== undefined) {
      const titleResult = validateTitle(command.title);
      if (!titleResult.ok) {
        return err(titleResult.error);
      }
      validatedTitle = titleResult.value;
    }

    // Validate content if provided
    let validatedContent: string | undefined;
    if (command.content !== undefined) {
      const contentResult = validateContent(command.content);
      if (!contentResult.ok) {
        return err(contentResult.error);
      }
      validatedContent = contentResult.value;
    }

    const data: UpdatePostData = {
      title: validatedTitle,
      content: validatedContent,
      status: command.status,
    };

    return this.postRepo.update(command.id, data);
  }

  async deletePost(command: DeletePostCommand): Promise<Result<void, PostNotFoundError | ForbiddenError>> {
    // Get existing post
    const postResult = await this.postRepo.findById(command.id);
    if (!postResult.ok) {
      return err(postResult.error);
    }

    const post = postResult.value;

    // Authorization: only author or Admin can delete
    if (post.authorId !== command.requesterId && command.requesterRole !== "Admin") {
      return err(new ForbiddenError("Only the author or admin can delete this post"));
    }

    return this.postRepo.delete(command.id);
  }

  async publishPost(command: PublishPostCommand): Promise<Result<Post, PostNotFoundError | ForbiddenError>> {
    // Get existing post
    const postResult = await this.postRepo.findById(command.id);
    if (!postResult.ok) {
      return err(postResult.error);
    }

    const post = postResult.value;

    // Authorization: only author or Admin/Moderator can publish
    if (post.authorId !== command.requesterId &&
        command.requesterRole !== "Admin" &&
        command.requesterRole !== "Moderator") {
      return err(new ForbiddenError("Only the author can publish this post"));
    }

    return this.postRepo.update(command.id, { status: "Published" });
  }
}

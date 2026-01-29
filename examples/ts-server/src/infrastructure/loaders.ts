/**
 * DataLoader Factory
 *
 * Creates DataLoaders for batch loading to prevent N+1 queries.
 * Loaders are created per-request for isolation.
 */

import { DataLoader, createLoader, createRelationLoader } from "@bgql/server";
import { User, Post, Comment, UserId, PostId } from "../domain/entities.js";
import { UserRepository, PostRepository, CommentRepository } from "./repositories.js";

/**
 * All DataLoaders for a single request.
 */
export interface Loaders {
  readonly user: DataLoader<UserId, User | null, UserId>;
  readonly post: DataLoader<PostId, Post | null, PostId>;
  readonly postsByAuthor: DataLoader<UserId, Post[], UserId>;
  readonly commentsByPost: DataLoader<PostId, Comment[], PostId>;
}

/**
 * Creates all DataLoaders for a request.
 *
 * DataLoaders batch and cache loads within a single request:
 * - Multiple `user.load(id)` calls are batched into one `findMany(ids)`
 * - Results are cached for the duration of the request
 */
export function createLoaders(
  userRepo: UserRepository,
  postRepo: PostRepository,
  commentRepo: CommentRepository
): Loaders {
  return {
    user: createLoader<UserId, User>(
      async (ids) => {
        console.log(`[Loader] Batch loading ${ids.length} users`);
        return userRepo.findMany(ids);
      },
      { name: "user" }
    ),

    post: createLoader<PostId, Post>(
      async (ids) => {
        console.log(`[Loader] Batch loading ${ids.length} posts`);
        return postRepo.findMany(ids);
      },
      { name: "post" }
    ),

    postsByAuthor: createRelationLoader<UserId, Post>(
      async (authorIds) => {
        console.log(`[Loader] Batch loading posts for ${authorIds.length} authors`);
        return postRepo.findManyByAuthors(authorIds);
      },
      { name: "postsByAuthor" }
    ),

    commentsByPost: createRelationLoader<PostId, Comment>(
      async (postIds) => {
        console.log(`[Loader] Batch loading comments for ${postIds.length} posts`);
        return commentRepo.findManyByPosts(postIds);
      },
      { name: "commentsByPost" }
    ),
  };
}

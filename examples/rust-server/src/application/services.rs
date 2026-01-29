//! Application services - Business logic orchestration.

use crate::domain::*;
use crate::infrastructure::*;
use std::sync::Arc;

pub struct UserService {
    user_repo: Arc<dyn UserRepository>,
    post_repo: Arc<dyn PostRepository>,
}

impl UserService {
    pub fn new(user_repo: Arc<dyn UserRepository>, post_repo: Arc<dyn PostRepository>) -> Self {
        Self {
            user_repo,
            post_repo,
        }
    }

    pub async fn get_user(&self, id: &UserId) -> DomainResult<User> {
        self.user_repo
            .find_by_id(id)
            .await
            .ok_or_else(|| DomainError::UserNotFound(id.0.clone()))
    }

    pub async fn get_user_with_posts(&self, id: &UserId) -> DomainResult<(User, Vec<Post>)> {
        let user = self.get_user(id).await?;
        let posts = self.post_repo.find_by_author(id, None).await;
        Ok((user, posts))
    }

    pub async fn list_users(&self, pagination: Pagination) -> Connection<User> {
        self.user_repo.find_all(pagination).await
    }

    pub async fn get_users_by_ids(
        &self,
        ids: &[UserId],
    ) -> std::collections::HashMap<UserId, User> {
        self.user_repo.find_by_ids(ids).await
    }
}

pub struct PostService {
    post_repo: Arc<dyn PostRepository>,
    user_repo: Arc<dyn UserRepository>,
}

impl PostService {
    pub fn new(post_repo: Arc<dyn PostRepository>, user_repo: Arc<dyn UserRepository>) -> Self {
        Self {
            post_repo,
            user_repo,
        }
    }

    pub async fn get_post(&self, id: &PostId) -> DomainResult<Post> {
        self.post_repo
            .find_by_id(id)
            .await
            .ok_or_else(|| DomainError::PostNotFound(id.0.clone()))
    }

    pub async fn get_post_with_author(&self, id: &PostId) -> DomainResult<(Post, Option<User>)> {
        let post = self.get_post(id).await?;
        let author = self.user_repo.find_by_id(&post.author_id).await;
        Ok((post, author))
    }

    pub async fn list_posts(&self, filter: PostFilter, pagination: Pagination) -> Connection<Post> {
        self.post_repo.find_all(filter, pagination).await
    }

    pub async fn create_post(&self, input: CreatePostInput) -> DomainResult<Post> {
        if input.title.is_empty() {
            return Err(DomainError::validation("title", "Title is required"));
        }
        if input.title.len() > 200 {
            return Err(DomainError::validation(
                "title",
                "Title must be at most 200 characters",
            ));
        }
        if input.content.len() < 10 {
            return Err(DomainError::validation(
                "content",
                "Content must be at least 10 characters",
            ));
        }

        let post = Post {
            id: PostId::new(""),
            title: input.title,
            content: input.content,
            status: input.status.unwrap_or(PostStatus::Draft),
            author_id: input.author_id,
            created_at: chrono::Utc::now(),
            updated_at: None,
        };
        Ok(self.post_repo.create(post).await)
    }

    pub async fn update_post(&self, id: &PostId, input: UpdatePostInput) -> DomainResult<Post> {
        if let Some(ref title) = input.title {
            if title.is_empty() {
                return Err(DomainError::validation("title", "Title cannot be empty"));
            }
        }

        self.post_repo
            .update(
                id,
                PostUpdate {
                    title: input.title,
                    content: input.content,
                    status: input.status,
                },
            )
            .await
            .ok_or_else(|| DomainError::PostNotFound(id.0.clone()))
    }

    pub async fn publish_post(&self, id: &PostId) -> DomainResult<Post> {
        self.post_repo
            .update(
                id,
                PostUpdate {
                    status: Some(PostStatus::Published),
                    ..Default::default()
                },
            )
            .await
            .ok_or_else(|| DomainError::PostNotFound(id.0.clone()))
    }

    pub async fn delete_post(&self, id: &PostId) -> DomainResult<()> {
        if self.post_repo.delete(id).await {
            Ok(())
        } else {
            Err(DomainError::PostNotFound(id.0.clone()))
        }
    }
}

pub struct CommentService {
    comment_repo: Arc<dyn CommentRepository>,
    post_repo: Arc<dyn PostRepository>,
}

impl CommentService {
    pub fn new(
        comment_repo: Arc<dyn CommentRepository>,
        post_repo: Arc<dyn PostRepository>,
    ) -> Self {
        Self {
            comment_repo,
            post_repo,
        }
    }

    pub async fn create_comment(&self, input: CreateCommentInput) -> DomainResult<Comment> {
        if input.content.is_empty() {
            return Err(DomainError::validation("content", "Content is required"));
        }
        if self.post_repo.find_by_id(&input.post_id).await.is_none() {
            return Err(DomainError::PostNotFound(input.post_id.0.clone()));
        }

        let comment = Comment {
            id: CommentId::new(""),
            content: input.content,
            post_id: input.post_id,
            author_id: input.author_id,
            created_at: chrono::Utc::now(),
        };
        Ok(self.comment_repo.create(comment).await)
    }
}

// Input Types
#[derive(Debug, Clone)]
pub struct CreatePostInput {
    pub author_id: UserId,
    pub title: String,
    pub content: String,
    pub status: Option<PostStatus>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdatePostInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub status: Option<PostStatus>,
}

#[derive(Debug, Clone)]
pub struct CreateCommentInput {
    pub post_id: PostId,
    pub author_id: UserId,
    pub content: String,
}

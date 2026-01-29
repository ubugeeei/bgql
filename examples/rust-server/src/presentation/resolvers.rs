//! GraphQL resolvers - Type-safe implementation using generated traits.

use crate::application::*;
use crate::generated::*;
use crate::infrastructure::*;
use async_trait::async_trait;
use bgql_sdk::server::Context;
use bgql_sdk::SdkResult;
use serde_json::json;
use std::sync::Arc;

/// Application context containing all services.
pub struct AppContext {
    pub user_service: UserService,
    pub post_service: PostService,
    pub comment_service: CommentService,
}

impl AppContext {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        post_repo: Arc<dyn PostRepository>,
        comment_repo: Arc<dyn CommentRepository>,
    ) -> Self {
        Self {
            user_service: UserService::new(user_repo.clone(), post_repo.clone()),
            post_service: PostService::new(post_repo.clone(), user_repo.clone()),
            comment_service: CommentService::new(comment_repo, post_repo),
        }
    }
}

/// Type-safe Query resolvers implementation.
pub struct AppQueryResolvers {
    ctx: Arc<AppContext>,
}

impl AppQueryResolvers {
    pub fn new(ctx: Arc<AppContext>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl QueryResolvers for AppQueryResolvers {
    async fn user(&self, _ctx: &Context, args: UserArgs) -> SdkResult<serde_json::Value> {
        let id = crate::domain::UserId::new(&args.id.0);

        match self.ctx.user_service.get_user_with_posts(&id).await {
            Ok((user, posts)) => Ok(json!({
                "__typename": "User",
                "id": user.id.0,
                "name": user.name,
                "email": user.email,
                "bio": user.bio,
                "createdAt": user.created_at.to_rfc3339(),
                "posts": posts.iter().map(|p| json!({
                    "id": p.id.0,
                    "title": p.title,
                    "status": p.status.to_string()
                })).collect::<Vec<_>>(),
                "postsCount": posts.len()
            })),
            Err(crate::domain::DomainError::UserNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("User '{}' not found", id),
                "code": "NOT_FOUND",
                "resourceType": "User",
                "resourceId": id
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string(),
                "code": "ERROR"
            })),
        }
    }

    async fn users(&self, _ctx: &Context, args: UsersArgs) -> SdkResult<serde_json::Value> {
        let pagination = args
            .pagination
            .map(|p| crate::domain::Pagination {
                first: p.first.map(|n| n as usize),
                after: p.after,
            })
            .unwrap_or_default();

        let conn = self.ctx.user_service.list_users(pagination).await;

        Ok(json!({
            "edges": conn.edges.iter().map(|e| json!({
                "cursor": e.cursor,
                "node": {
                    "id": e.node.id.0,
                    "name": e.node.name,
                    "email": e.node.email,
                    "bio": e.node.bio,
                    "createdAt": e.node.created_at.to_rfc3339()
                }
            })).collect::<Vec<_>>(),
            "pageInfo": {
                "hasNextPage": conn.page_info.has_next_page,
                "hasPreviousPage": conn.page_info.has_previous_page,
                "startCursor": conn.page_info.start_cursor,
                "endCursor": conn.page_info.end_cursor
            },
            "totalCount": conn.total_count
        }))
    }

    async fn post(&self, _ctx: &Context, args: PostArgs) -> SdkResult<serde_json::Value> {
        let id = crate::domain::PostId::new(&args.id.0);

        match self.ctx.post_service.get_post_with_author(&id).await {
            Ok((post, author)) => Ok(json!({
                "__typename": "Post",
                "id": post.id.0,
                "title": post.title,
                "content": post.content,
                "status": post.status.to_string(),
                "authorId": post.author_id.0,
                "author": author.map(|a| json!({
                    "id": a.id.0,
                    "name": a.name,
                    "email": a.email
                })),
                "createdAt": post.created_at.to_rfc3339(),
                "updatedAt": post.updated_at.map(|t| t.to_rfc3339())
            })),
            Err(crate::domain::DomainError::PostNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("Post '{}' not found", id),
                "code": "NOT_FOUND",
                "resourceType": "Post",
                "resourceId": id
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string(),
                "code": "ERROR"
            })),
        }
    }

    async fn posts(&self, _ctx: &Context, args: PostsArgs) -> SdkResult<serde_json::Value> {
        let pagination = args
            .pagination
            .map(|p| crate::domain::Pagination {
                first: p.first.map(|n| n as usize),
                after: p.after,
            })
            .unwrap_or_default();

        let filter = args
            .filter
            .map(|f| crate::domain::PostFilter {
                status: f
                    .status
                    .and_then(|s| crate::domain::PostStatus::from_str(&s.to_string())),
                author_id: f.author_id.map(|id| crate::domain::UserId::new(&id.0)),
            })
            .unwrap_or_default();

        let conn = self.ctx.post_service.list_posts(filter, pagination).await;
        let author_ids: Vec<_> = conn
            .edges
            .iter()
            .map(|e| e.node.author_id.clone())
            .collect();
        let authors = self.ctx.user_service.get_users_by_ids(&author_ids).await;

        Ok(json!({
            "edges": conn.edges.iter().map(|e| {
                let author = authors.get(&e.node.author_id);
                json!({
                    "cursor": e.cursor,
                    "node": {
                        "id": e.node.id.0,
                        "title": e.node.title,
                        "content": e.node.content,
                        "status": e.node.status.to_string(),
                        "authorId": e.node.author_id.0,
                        "author": author.map(|a| json!({"id": a.id.0, "name": a.name})),
                        "createdAt": e.node.created_at.to_rfc3339()
                    }
                })
            }).collect::<Vec<_>>(),
            "pageInfo": {
                "hasNextPage": conn.page_info.has_next_page,
                "hasPreviousPage": conn.page_info.has_previous_page,
                "startCursor": conn.page_info.start_cursor,
                "endCursor": conn.page_info.end_cursor
            },
            "totalCount": conn.total_count
        }))
    }
}

/// Type-safe Mutation resolvers implementation.
pub struct AppMutationResolvers {
    ctx: Arc<AppContext>,
}

impl AppMutationResolvers {
    pub fn new(ctx: Arc<AppContext>) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl MutationResolvers for AppMutationResolvers {
    async fn create_post(
        &self,
        _ctx: &Context,
        args: CreatePostArgs,
    ) -> SdkResult<serde_json::Value> {
        let input = crate::application::CreatePostInput {
            author_id: crate::domain::UserId::new("user_1"), // TODO: from auth context
            title: args.input.title,
            content: args.input.content,
            status: args
                .input
                .status
                .and_then(|s| crate::domain::PostStatus::from_str(&s.to_string())),
        };

        match self.ctx.post_service.create_post(input).await {
            Ok(post) => Ok(json!({
                "__typename": "Post",
                "id": post.id.0,
                "title": post.title,
                "content": post.content,
                "status": post.status.to_string(),
                "createdAt": post.created_at.to_rfc3339()
            })),
            Err(crate::domain::DomainError::ValidationError { field, message }) => Ok(json!({
                "__typename": "ValidationError",
                "message": message,
                "code": "VALIDATION_ERROR",
                "field": field
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string(),
                "code": "ERROR"
            })),
        }
    }

    async fn update_post(
        &self,
        _ctx: &Context,
        args: UpdatePostArgs,
    ) -> SdkResult<serde_json::Value> {
        let id = crate::domain::PostId::new(&args.id.0);
        let input = crate::application::UpdatePostInput {
            title: args.input.title,
            content: args.input.content,
            status: args
                .input
                .status
                .and_then(|s| crate::domain::PostStatus::from_str(&s.to_string())),
        };

        match self.ctx.post_service.update_post(&id, input).await {
            Ok(post) => Ok(json!({
                "__typename": "Post",
                "id": post.id.0,
                "title": post.title,
                "status": post.status.to_string()
            })),
            Err(crate::domain::DomainError::PostNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("Post '{}' not found", id),
                "code": "NOT_FOUND"
            })),
            Err(crate::domain::DomainError::ValidationError { field, message }) => Ok(json!({
                "__typename": "ValidationError",
                "message": message,
                "field": field
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string()
            })),
        }
    }

    async fn publish_post(
        &self,
        _ctx: &Context,
        args: PublishPostArgs,
    ) -> SdkResult<serde_json::Value> {
        let id = crate::domain::PostId::new(&args.id.0);

        match self.ctx.post_service.publish_post(&id).await {
            Ok(post) => Ok(json!({
                "__typename": "Post",
                "id": post.id.0,
                "title": post.title,
                "status": post.status.to_string()
            })),
            Err(crate::domain::DomainError::PostNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("Post '{}' not found", id),
                "code": "NOT_FOUND"
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string()
            })),
        }
    }

    async fn delete_post(
        &self,
        _ctx: &Context,
        args: DeletePostArgs,
    ) -> SdkResult<serde_json::Value> {
        let id = crate::domain::PostId::new(&args.id.0);

        match self.ctx.post_service.delete_post(&id).await {
            Ok(()) => Ok(json!({
                "__typename": "DeleteSuccess",
                "success": true,
                "deletedId": args.id.0
            })),
            Err(crate::domain::DomainError::PostNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("Post '{}' not found", id),
                "code": "NOT_FOUND"
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string()
            })),
        }
    }

    async fn create_comment(
        &self,
        _ctx: &Context,
        args: CreateCommentArgs,
    ) -> SdkResult<serde_json::Value> {
        let input = crate::application::CreateCommentInput {
            post_id: crate::domain::PostId::new(&args.input.post_id.0),
            author_id: crate::domain::UserId::new("user_1"), // TODO: from auth context
            content: args.input.content,
        };

        match self.ctx.comment_service.create_comment(input).await {
            Ok(comment) => Ok(json!({
                "__typename": "Comment",
                "id": comment.id.0,
                "content": comment.content,
                "postId": comment.post_id.0,
                "createdAt": comment.created_at.to_rfc3339()
            })),
            Err(crate::domain::DomainError::PostNotFound(id)) => Ok(json!({
                "__typename": "NotFoundError",
                "message": format!("Post '{}' not found", id),
                "code": "NOT_FOUND"
            })),
            Err(crate::domain::DomainError::ValidationError { field, message }) => Ok(json!({
                "__typename": "ValidationError",
                "message": message,
                "field": field
            })),
            Err(e) => Ok(json!({
                "__typename": "ValidationError",
                "message": e.to_string()
            })),
        }
    }
}

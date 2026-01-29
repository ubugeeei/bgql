//! Repository implementations - Data access layer.

use crate::domain::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

// Repository Traits
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &UserId) -> Option<User>;
    async fn find_by_ids(&self, ids: &[UserId]) -> HashMap<UserId, User>;
    async fn find_all(&self, pagination: Pagination) -> Connection<User>;
}

#[async_trait]
pub trait PostRepository: Send + Sync {
    async fn find_by_id(&self, id: &PostId) -> Option<Post>;
    async fn find_by_author(&self, author_id: &UserId, status: Option<PostStatus>) -> Vec<Post>;
    async fn find_all(&self, filter: PostFilter, pagination: Pagination) -> Connection<Post>;
    async fn create(&self, post: Post) -> Post;
    async fn update(&self, id: &PostId, update: PostUpdate) -> Option<Post>;
    async fn delete(&self, id: &PostId) -> bool;
}

#[async_trait]
pub trait CommentRepository: Send + Sync {
    async fn create(&self, comment: Comment) -> Comment;
}

// Re-export from domain
pub use crate::domain::PostFilter;

// Update Types
#[derive(Debug, Clone, Default)]
pub struct PostUpdate {
    pub title: Option<String>,
    pub content: Option<String>,
    pub status: Option<PostStatus>,
}

// In-Memory User Repository
pub struct InMemoryUserRepository {
    users: RwLock<HashMap<UserId, User>>,
}

impl InMemoryUserRepository {
    pub fn with_seed_data() -> Self {
        let mut users = HashMap::new();
        for (id, name, email, bio) in [
            (
                "user_1",
                "Alice Johnson",
                "alice@example.com",
                Some("Software engineer"),
            ),
            ("user_2", "Bob Smith", "bob@example.com", None),
            (
                "user_3",
                "Carol Williams",
                "carol@example.com",
                Some("Tech writer"),
            ),
        ] {
            let user = User {
                id: UserId::new(id),
                name: name.to_string(),
                email: email.to_string(),
                bio: bio.map(String::from),
                created_at: chrono::Utc::now(),
            };
            users.insert(user.id.clone(), user);
        }
        Self {
            users: RwLock::new(users),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(&self, id: &UserId) -> Option<User> {
        self.users.read().await.get(id).cloned()
    }

    async fn find_by_ids(&self, ids: &[UserId]) -> HashMap<UserId, User> {
        let users = self.users.read().await;
        ids.iter()
            .filter_map(|id| users.get(id).map(|u| (id.clone(), u.clone())))
            .collect()
    }

    async fn find_all(&self, pagination: Pagination) -> Connection<User> {
        let users = self.users.read().await;
        let mut all: Vec<_> = users.values().cloned().collect();
        all.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let total = all.len();
        let first = pagination.first.unwrap_or(10);
        let start = pagination
            .after
            .and_then(|cursor| all.iter().position(|u| u.id.0 == cursor))
            .map(|i| i + 1)
            .unwrap_or(0);

        let items: Vec<_> = all.into_iter().skip(start).take(first + 1).collect();
        let has_next = items.len() > first;
        let items: Vec<_> = items.into_iter().take(first).collect();

        let edges: Vec<Edge<User>> = items
            .into_iter()
            .map(|u| Edge {
                cursor: u.id.0.clone(),
                node: u,
            })
            .collect();

        Connection {
            page_info: PageInfo {
                has_next_page: has_next,
                has_previous_page: start > 0,
                start_cursor: edges.first().map(|e| e.cursor.clone()),
                end_cursor: edges.last().map(|e| e.cursor.clone()),
            },
            edges,
            total_count: total,
        }
    }
}

// In-Memory Post Repository
pub struct InMemoryPostRepository {
    posts: RwLock<HashMap<PostId, Post>>,
    next_id: RwLock<u64>,
}

impl InMemoryPostRepository {
    pub fn with_seed_data() -> Self {
        let mut posts = HashMap::new();
        for (id, author_id, title, content, status) in [
            (
                "post_1",
                "user_1",
                "Introduction to BGQL",
                "Better GraphQL is...",
                PostStatus::Published,
            ),
            (
                "post_2",
                "user_1",
                "Schema-First Development",
                "In schema-first...",
                PostStatus::Published,
            ),
            (
                "post_3",
                "user_2",
                "DataLoaders Explained",
                "DataLoaders solve...",
                PostStatus::Published,
            ),
            (
                "post_4",
                "user_1",
                "Draft: Typed Errors",
                "Work in progress...",
                PostStatus::Draft,
            ),
            (
                "post_5",
                "user_3",
                "Rust and GraphQL",
                "Building GraphQL servers...",
                PostStatus::Published,
            ),
        ] {
            let post = Post {
                id: PostId::new(id),
                author_id: UserId::new(author_id),
                title: title.to_string(),
                content: content.to_string(),
                status,
                created_at: chrono::Utc::now(),
                updated_at: None,
            };
            posts.insert(post.id.clone(), post);
        }
        Self {
            posts: RwLock::new(posts),
            next_id: RwLock::new(6),
        }
    }
}

#[async_trait]
impl PostRepository for InMemoryPostRepository {
    async fn find_by_id(&self, id: &PostId) -> Option<Post> {
        self.posts.read().await.get(id).cloned()
    }

    async fn find_by_author(&self, author_id: &UserId, status: Option<PostStatus>) -> Vec<Post> {
        self.posts
            .read()
            .await
            .values()
            .filter(|p| &p.author_id == author_id && status.map(|s| p.status == s).unwrap_or(true))
            .cloned()
            .collect()
    }

    async fn find_all(&self, filter: PostFilter, pagination: Pagination) -> Connection<Post> {
        let posts = self.posts.read().await;
        let mut all: Vec<_> = posts
            .values()
            .filter(|p| {
                filter.status.map(|s| p.status == s).unwrap_or(true)
                    && filter
                        .author_id
                        .as_ref()
                        .map(|a| &p.author_id == a)
                        .unwrap_or(true)
            })
            .cloned()
            .collect();
        all.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let total = all.len();
        let first = pagination.first.unwrap_or(10);
        let start = pagination
            .after
            .and_then(|cursor| all.iter().position(|p| p.id.0 == cursor))
            .map(|i| i + 1)
            .unwrap_or(0);

        let items: Vec<_> = all.into_iter().skip(start).take(first + 1).collect();
        let has_next = items.len() > first;
        let items: Vec<_> = items.into_iter().take(first).collect();

        let edges: Vec<Edge<Post>> = items
            .into_iter()
            .map(|p| Edge {
                cursor: p.id.0.clone(),
                node: p,
            })
            .collect();

        Connection {
            page_info: PageInfo {
                has_next_page: has_next,
                has_previous_page: start > 0,
                start_cursor: edges.first().map(|e| e.cursor.clone()),
                end_cursor: edges.last().map(|e| e.cursor.clone()),
            },
            edges,
            total_count: total,
        }
    }

    async fn create(&self, mut post: Post) -> Post {
        let mut posts = self.posts.write().await;
        let mut next_id = self.next_id.write().await;
        post.id = PostId::new(format!("post_{}", *next_id));
        *next_id += 1;
        post.created_at = chrono::Utc::now();
        posts.insert(post.id.clone(), post.clone());
        post
    }

    async fn update(&self, id: &PostId, update: PostUpdate) -> Option<Post> {
        let mut posts = self.posts.write().await;
        if let Some(post) = posts.get_mut(id) {
            if let Some(title) = update.title {
                post.title = title;
            }
            if let Some(content) = update.content {
                post.content = content;
            }
            if let Some(status) = update.status {
                post.status = status;
            }
            post.updated_at = Some(chrono::Utc::now());
            Some(post.clone())
        } else {
            None
        }
    }

    async fn delete(&self, id: &PostId) -> bool {
        self.posts.write().await.remove(id).is_some()
    }
}

// In-Memory Comment Repository
pub struct InMemoryCommentRepository {
    comments: RwLock<Vec<Comment>>,
    next_id: RwLock<u64>,
}

impl InMemoryCommentRepository {
    pub fn new() -> Self {
        Self {
            comments: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
        }
    }
}

#[async_trait]
impl CommentRepository for InMemoryCommentRepository {
    async fn create(&self, mut comment: Comment) -> Comment {
        let mut comments = self.comments.write().await;
        let mut next_id = self.next_id.write().await;
        comment.id = CommentId::new(format!("comment_{}", *next_id));
        *next_id += 1;
        comment.created_at = chrono::Utc::now();
        comments.push(comment.clone());
        comment
    }
}

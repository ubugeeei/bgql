//! Mock database for demonstration.
//!
//! In production, this would be replaced with actual database connections
//! (e.g., sqlx, diesel, or sea-orm).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Post status enum matching the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl std::fmt::Display for PostStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostStatus::Draft => write!(f, "Draft"),
            PostStatus::Published => write!(f, "Published"),
            PostStatus::Archived => write!(f, "Archived"),
        }
    }
}

/// User entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Post entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub content: String,
    pub status: PostStatus,
    pub author_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Mock database with in-memory storage.
#[derive(Debug, Clone)]
pub struct Database {
    users: Arc<RwLock<HashMap<String, User>>>,
    posts: Arc<RwLock<HashMap<String, Post>>>,
    next_post_id: Arc<RwLock<u64>>,
}

impl Database {
    /// Creates a new database with sample data.
    pub fn new_with_sample_data() -> Self {
        let mut users = HashMap::new();
        let mut posts = HashMap::new();

        // Sample users
        users.insert(
            "user_1".to_string(),
            User {
                id: "user_1".to_string(),
                name: "Alice Johnson".to_string(),
                email: "alice@example.com".to_string(),
                bio: Some("Software engineer and blogger".to_string()),
                created_at: Utc::now() - chrono::Duration::days(365),
            },
        );
        users.insert(
            "user_2".to_string(),
            User {
                id: "user_2".to_string(),
                name: "Bob Smith".to_string(),
                email: "bob@example.com".to_string(),
                bio: None,
                created_at: Utc::now() - chrono::Duration::days(180),
            },
        );
        users.insert(
            "user_3".to_string(),
            User {
                id: "user_3".to_string(),
                name: "Carol Williams".to_string(),
                email: "carol@example.com".to_string(),
                bio: Some("Tech writer and open source enthusiast".to_string()),
                created_at: Utc::now() - chrono::Duration::days(90),
            },
        );

        // Sample posts
        posts.insert(
            "post_1".to_string(),
            Post {
                id: "post_1".to_string(),
                title: "Introduction to BGQL".to_string(),
                content: "Better GraphQL (BGQL) is a modern evolution of GraphQL that addresses its weaknesses...".to_string(),
                status: PostStatus::Published,
                author_id: "user_1".to_string(),
                created_at: Utc::now() - chrono::Duration::days(30),
                updated_at: Some(Utc::now() - chrono::Duration::days(5)),
            },
        );
        posts.insert(
            "post_2".to_string(),
            Post {
                id: "post_2".to_string(),
                title: "Schema-First Development".to_string(),
                content: "In schema-first development, the schema is the source of truth...".to_string(),
                status: PostStatus::Published,
                author_id: "user_1".to_string(),
                created_at: Utc::now() - chrono::Duration::days(20),
                updated_at: None,
            },
        );
        posts.insert(
            "post_3".to_string(),
            Post {
                id: "post_3".to_string(),
                title: "DataLoaders Explained".to_string(),
                content: "DataLoaders solve the N+1 query problem by batching requests...".to_string(),
                status: PostStatus::Published,
                author_id: "user_2".to_string(),
                created_at: Utc::now() - chrono::Duration::days(15),
                updated_at: None,
            },
        );
        posts.insert(
            "post_4".to_string(),
            Post {
                id: "post_4".to_string(),
                title: "Draft: Typed Errors".to_string(),
                content: "Work in progress...".to_string(),
                status: PostStatus::Draft,
                author_id: "user_1".to_string(),
                created_at: Utc::now() - chrono::Duration::days(2),
                updated_at: None,
            },
        );
        posts.insert(
            "post_5".to_string(),
            Post {
                id: "post_5".to_string(),
                title: "Rust and GraphQL".to_string(),
                content: "Building GraphQL servers in Rust offers excellent performance...".to_string(),
                status: PostStatus::Published,
                author_id: "user_3".to_string(),
                created_at: Utc::now() - chrono::Duration::days(10),
                updated_at: None,
            },
        );

        Self {
            users: Arc::new(RwLock::new(users)),
            posts: Arc::new(RwLock::new(posts)),
            next_post_id: Arc::new(RwLock::new(6)),
        }
    }

    // =========================================================================
    // User Operations
    // =========================================================================

    /// Gets a user by ID.
    pub async fn get_user(&self, id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.get(id).cloned()
    }

    /// Gets multiple users by IDs (batch operation for DataLoader).
    pub async fn get_users_by_ids(&self, ids: Vec<String>) -> HashMap<String, User> {
        let users = self.users.read().await;
        ids.into_iter()
            .filter_map(|id| users.get(&id).map(|u| (id, u.clone())))
            .collect()
    }

    /// Gets all users with pagination.
    pub async fn get_users(&self, limit: usize, after: Option<&str>) -> (Vec<User>, bool) {
        let users = self.users.read().await;
        let mut all_users: Vec<_> = users.values().cloned().collect();
        all_users.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let start_index = if let Some(cursor) = after {
            all_users
                .iter()
                .position(|u| u.id == cursor)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let items: Vec<_> = all_users.into_iter().skip(start_index).take(limit + 1).collect();
        let has_next = items.len() > limit;
        let items: Vec<_> = items.into_iter().take(limit).collect();

        (items, has_next)
    }

    /// Gets the total user count.
    pub async fn get_user_count(&self) -> usize {
        self.users.read().await.len()
    }

    // =========================================================================
    // Post Operations
    // =========================================================================

    /// Gets a post by ID.
    pub async fn get_post(&self, id: &str) -> Option<Post> {
        let posts = self.posts.read().await;
        posts.get(id).cloned()
    }

    /// Gets posts by author ID with optional status filter.
    pub async fn get_posts_by_author(
        &self,
        author_id: &str,
        status: Option<PostStatus>,
    ) -> Vec<Post> {
        let posts = self.posts.read().await;
        posts
            .values()
            .filter(|p| {
                p.author_id == author_id && status.map(|s| p.status == s).unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Gets posts by multiple author IDs (batch operation for DataLoader).
    #[allow(dead_code)]
    pub async fn get_posts_by_author_ids(
        &self,
        author_ids: Vec<String>,
    ) -> HashMap<String, Vec<Post>> {
        let posts = self.posts.read().await;
        let mut result: HashMap<String, Vec<Post>> = HashMap::new();

        for post in posts.values() {
            if author_ids.contains(&post.author_id) {
                result
                    .entry(post.author_id.clone())
                    .or_default()
                    .push(post.clone());
            }
        }

        // Ensure all requested author_ids have an entry (even if empty)
        for author_id in author_ids {
            result.entry(author_id).or_default();
        }

        result
    }

    /// Gets posts count by author ID.
    #[allow(dead_code)]
    pub async fn get_posts_count_by_author(&self, author_id: &str) -> usize {
        let posts = self.posts.read().await;
        posts.values().filter(|p| p.author_id == author_id).count()
    }

    /// Gets all posts with pagination and optional filter.
    pub async fn get_posts(
        &self,
        limit: usize,
        after: Option<&str>,
        status: Option<PostStatus>,
        author_id: Option<&str>,
    ) -> (Vec<Post>, bool) {
        let posts = self.posts.read().await;
        let mut all_posts: Vec<_> = posts
            .values()
            .filter(|p| {
                let status_match = status.map(|s| p.status == s).unwrap_or(true);
                let author_match = author_id.map(|a| p.author_id == a).unwrap_or(true);
                status_match && author_match
            })
            .cloned()
            .collect();
        all_posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let start_index = if let Some(cursor) = after {
            all_posts
                .iter()
                .position(|p| p.id == cursor)
                .map(|i| i + 1)
                .unwrap_or(0)
        } else {
            0
        };

        let items: Vec<_> = all_posts.into_iter().skip(start_index).take(limit + 1).collect();
        let has_next = items.len() > limit;
        let items: Vec<_> = items.into_iter().take(limit).collect();

        (items, has_next)
    }

    /// Gets the total post count with optional filter.
    pub async fn get_post_count(
        &self,
        status: Option<PostStatus>,
        author_id: Option<&str>,
    ) -> usize {
        let posts = self.posts.read().await;
        posts
            .values()
            .filter(|p| {
                let status_match = status.map(|s| p.status == s).unwrap_or(true);
                let author_match = author_id.map(|a| p.author_id == a).unwrap_or(true);
                status_match && author_match
            })
            .count()
    }

    /// Creates a new post.
    pub async fn create_post(
        &self,
        author_id: String,
        title: String,
        content: String,
        status: PostStatus,
    ) -> Post {
        let mut posts = self.posts.write().await;
        let mut next_id = self.next_post_id.write().await;

        let id = format!("post_{}", *next_id);
        *next_id += 1;

        let post = Post {
            id: id.clone(),
            title,
            content,
            status,
            author_id,
            created_at: Utc::now(),
            updated_at: None,
        };

        posts.insert(id, post.clone());
        post
    }

    /// Updates an existing post.
    pub async fn update_post(
        &self,
        id: &str,
        title: Option<String>,
        content: Option<String>,
        status: Option<PostStatus>,
    ) -> Option<Post> {
        let mut posts = self.posts.write().await;

        if let Some(post) = posts.get_mut(id) {
            if let Some(t) = title {
                post.title = t;
            }
            if let Some(c) = content {
                post.content = c;
            }
            if let Some(s) = status {
                post.status = s;
            }
            post.updated_at = Some(Utc::now());
            Some(post.clone())
        } else {
            None
        }
    }

    /// Deletes a post.
    pub async fn delete_post(&self, id: &str) -> bool {
        let mut posts = self.posts.write().await;
        posts.remove(id).is_some()
    }
}

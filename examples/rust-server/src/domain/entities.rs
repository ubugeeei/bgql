//! Domain entities - Core business objects.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Value Objects
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

impl UserId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl AsRef<str> for UserId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PostId(pub String);

impl PostId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl AsRef<str> for PostId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommentId(pub String);

impl CommentId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

// Entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl PostStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Draft" => Some(PostStatus::Draft),
            "Published" => Some(PostStatus::Published),
            "Archived" => Some(PostStatus::Archived),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: PostId,
    pub title: String,
    pub content: String,
    pub status: PostStatus,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: CommentId,
    pub content: String,
    pub post_id: PostId,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
}

// Filters
#[derive(Debug, Clone, Default)]
pub struct PostFilter {
    pub status: Option<PostStatus>,
    pub author_id: Option<UserId>,
}

// Pagination
#[derive(Debug, Clone, Default)]
pub struct Pagination {
    pub first: Option<usize>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Connection<T> {
    pub edges: Vec<Edge<T>>,
    pub page_info: PageInfo,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
pub struct Edge<T> {
    pub node: T,
    pub cursor: String,
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

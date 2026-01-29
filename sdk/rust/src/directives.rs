//! Built-in directive types for Better GraphQL.
//!
//! This module provides types for working with BGQL's streaming-first directives:
//!
//! - **@defer** - Deferred field resolution for non-critical data
//! - **@stream** - Incremental list streaming
//! - **@binary** - Binary data streaming with progressive delivery
//! - **@server** - Server-side only fragments
//! - **@boundary** - Client-server boundary markers
//! - **@priority** - Execution priority hints
//! - **@resources** - Resource usage hints for scheduling
//! - **@resumable** - Pause/resume support for long-running queries
//!
//! # Example
//!
//! ```ignore
//! use bgql_sdk::directives::{DeferDirective, StreamDirective, PriorityDirective};
//!
//! // Configure a defer directive
//! let defer = DeferDirective::labeled("userBio")
//!     .with_fallback(serde_json::json!({"bio": "Loading..."}));
//!
//! // Configure a stream directive
//! let stream = StreamDirective::labeled("posts")
//!     .with_initial_count(5);
//!
//! // Set execution priority
//! let priority = PriorityDirective::high()
//!     .with_deadline("2024-01-01T00:00:00Z");
//! ```

// Re-export all directive types from runtime
pub use bgql_runtime::directives::{
    // @binary directive
    BinaryDirective,
    // @boundary directive
    BoundaryDirective,
    // Cache strategy enum
    CacheStrategy,
    // @defer directive (extended)
    DeferDirective,
    // @hydrate directive
    HydrateDirective,
    // Hydration enums
    HydrationPriority,
    HydrationStrategy,
    // @island directive
    IslandDirective,
    // @priority directive
    PriorityDirective,
    // @resources directive
    ResourcesDirective,
    // @resumable directive
    ResumableDirective,
    // Serialization strategy enum
    SerializeStrategy,
    // @server directive
    ServerDirective,
    // @stream directive (extended)
    StreamDirective,
    // Utility function
    create_streaming_directives,
};

// Re-export resource level from resource module
pub use bgql_runtime::resource::ResourceLevel;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed directives from a GraphQL field or fragment.
#[derive(Debug, Clone, Default)]
pub struct ParsedDirectives {
    /// @defer directive if present.
    pub defer: Option<DeferDirective>,
    /// @stream directive if present.
    pub stream: Option<StreamDirective>,
    /// @binary directive if present.
    pub binary: Option<BinaryDirective>,
    /// @server directive if present.
    pub server: Option<ServerDirective>,
    /// @boundary directive if present.
    pub boundary: Option<BoundaryDirective>,
    /// @island directive if present.
    pub island: Option<IslandDirective>,
    /// @hydrate directive if present.
    pub hydrate: Option<HydrateDirective>,
    /// @priority directive if present.
    pub priority: Option<PriorityDirective>,
    /// @resources directive if present.
    pub resources: Option<ResourcesDirective>,
    /// @resumable directive if present.
    pub resumable: Option<ResumableDirective>,
    /// @cache directive if present.
    pub cache: Option<CacheDirective>,
    /// @rateLimit directive if present.
    pub rate_limit: Option<RateLimitDirective>,
    /// @requireAuth directive if present.
    pub require_auth: Option<RequireAuthDirective>,
}

impl ParsedDirectives {
    /// Creates a new empty set of parsed directives.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if any streaming directive is present.
    pub fn has_streaming(&self) -> bool {
        self.defer.is_some() || self.stream.is_some() || self.binary.is_some()
    }

    /// Returns true if the field/type should be server-only.
    pub fn is_server_only(&self) -> bool {
        self.server.as_ref().map(|s| s.isolate).unwrap_or(false)
            || self
                .boundary
                .as_ref()
                .map(|b| b.is_sensitive())
                .unwrap_or(false)
    }

    /// Returns true if authentication is required.
    pub fn requires_auth(&self) -> bool {
        self.require_auth.is_some()
    }

    /// Gets the required roles, if any.
    pub fn required_roles(&self) -> Option<&[String]> {
        self.require_auth.as_ref().map(|r| r.roles.as_slice())
    }

    /// Gets the execution priority level.
    pub fn priority_level(&self) -> u8 {
        self.priority.as_ref().map(|p| p.level).unwrap_or(5)
    }

    /// Gets the cache max-age in seconds.
    pub fn cache_max_age(&self) -> Option<u32> {
        self.cache.as_ref().map(|c| c.max_age)
    }
}

/// @cache directive for HTTP caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDirective {
    /// Maximum age in seconds.
    pub max_age: u32,
    /// Cache scope.
    pub scope: CacheScope,
    /// Stale-while-revalidate time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_while_revalidate: Option<u32>,
    /// Vary headers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vary: Vec<String>,
}

impl Default for CacheDirective {
    fn default() -> Self {
        Self {
            max_age: 0,
            scope: CacheScope::Private,
            stale_while_revalidate: None,
            vary: Vec::new(),
        }
    }
}

impl CacheDirective {
    /// Creates a public cache directive.
    pub fn public(max_age: u32) -> Self {
        Self {
            max_age,
            scope: CacheScope::Public,
            ..Default::default()
        }
    }

    /// Creates a private cache directive.
    pub fn private(max_age: u32) -> Self {
        Self {
            max_age,
            scope: CacheScope::Private,
            ..Default::default()
        }
    }

    /// Creates a no-store directive.
    pub fn no_store() -> Self {
        Self {
            max_age: 0,
            scope: CacheScope::NoStore,
            ..Default::default()
        }
    }

    /// Sets stale-while-revalidate time.
    pub fn with_swr(mut self, seconds: u32) -> Self {
        self.stale_while_revalidate = Some(seconds);
        self
    }

    /// Adds a vary header.
    pub fn vary_on(mut self, header: impl Into<String>) -> Self {
        self.vary.push(header.into());
        self
    }

    /// Generates a Cache-Control header value.
    pub fn to_cache_control(&self) -> String {
        let mut parts = Vec::new();

        match self.scope {
            CacheScope::Public => parts.push("public".to_string()),
            CacheScope::Private => parts.push("private".to_string()),
            CacheScope::NoStore => return "no-store".to_string(),
        }

        parts.push(format!("max-age={}", self.max_age));

        if let Some(swr) = self.stale_while_revalidate {
            parts.push(format!("stale-while-revalidate={}", swr));
        }

        parts.join(", ")
    }
}

/// Cache scope for @cache directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CacheScope {
    /// Publicly cacheable.
    Public,
    /// Private (user-specific) cache only.
    #[default]
    Private,
    /// Do not cache.
    NoStore,
}

/// @rateLimit directive for request throttling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitDirective {
    /// Maximum requests allowed.
    pub requests: u32,
    /// Time window (e.g., "1m", "1h", "1d").
    pub window: String,
    /// Optional custom key for rate limiting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl Default for RateLimitDirective {
    fn default() -> Self {
        Self {
            requests: 100,
            window: "1m".to_string(),
            key: None,
        }
    }
}

impl RateLimitDirective {
    /// Creates a new rate limit directive.
    pub fn new(requests: u32, window: impl Into<String>) -> Self {
        Self {
            requests,
            window: window.into(),
            key: None,
        }
    }

    /// Sets a custom rate limit key.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Parses the window duration to seconds.
    pub fn window_seconds(&self) -> Option<u64> {
        parse_duration(&self.window)
    }
}

/// @requireAuth directive for authentication.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequireAuthDirective {
    /// Required roles (empty means any authenticated user).
    #[serde(default)]
    pub roles: Vec<String>,
}

impl RequireAuthDirective {
    /// Creates an auth requirement for any authenticated user.
    pub fn any() -> Self {
        Self { roles: Vec::new() }
    }

    /// Creates an auth requirement for specific roles.
    pub fn with_roles(roles: Vec<String>) -> Self {
        Self { roles }
    }

    /// Checks if a user with the given roles is authorized.
    pub fn is_authorized(&self, user_roles: &[String]) -> bool {
        if self.roles.is_empty() {
            // Any authenticated user
            return true;
        }
        // Check if user has any of the required roles
        self.roles.iter().any(|r| user_roles.contains(r))
    }
}

/// Parses a duration string like "1m", "1h", "1d" to seconds.
fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: u64 = num_str.parse().ok()?;

    match unit {
        "s" => Some(num),
        "m" => Some(num * 60),
        "h" => Some(num * 3600),
        "d" => Some(num * 86400),
        _ => None,
    }
}

/// Context for directive execution.
#[derive(Debug, Clone)]
pub struct DirectiveContext {
    /// The current field path.
    pub path: Vec<String>,
    /// Variables from the request.
    pub variables: HashMap<String, serde_json::Value>,
    /// Whether we're in a streaming context.
    pub is_streaming: bool,
    /// The authenticated user's roles.
    pub user_roles: Vec<String>,
}

impl DirectiveContext {
    /// Creates a new directive context.
    pub fn new() -> Self {
        Self {
            path: Vec::new(),
            variables: HashMap::new(),
            is_streaming: false,
            user_roles: Vec::new(),
        }
    }

    /// Adds a path segment.
    pub fn push_path(&mut self, segment: impl Into<String>) {
        self.path.push(segment.into());
    }

    /// Removes the last path segment.
    pub fn pop_path(&mut self) -> Option<String> {
        self.path.pop()
    }

    /// Sets the user roles.
    pub fn with_user_roles(mut self, roles: Vec<String>) -> Self {
        self.user_roles = roles;
        self
    }
}

impl Default for DirectiveContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_directive() {
        let cache = CacheDirective::public(300).with_swr(60).vary_on("Authorization");

        assert_eq!(cache.max_age, 300);
        assert_eq!(cache.scope, CacheScope::Public);
        assert_eq!(cache.stale_while_revalidate, Some(60));
        assert!(cache.vary.contains(&"Authorization".to_string()));

        let header = cache.to_cache_control();
        assert!(header.contains("public"));
        assert!(header.contains("max-age=300"));
        assert!(header.contains("stale-while-revalidate=60"));
    }

    #[test]
    fn test_rate_limit_directive() {
        let limit = RateLimitDirective::new(100, "1h");
        assert_eq!(limit.window_seconds(), Some(3600));

        let limit2 = RateLimitDirective::new(10, "1m");
        assert_eq!(limit2.window_seconds(), Some(60));
    }

    #[test]
    fn test_require_auth_directive() {
        let any_auth = RequireAuthDirective::any();
        assert!(any_auth.is_authorized(&["Reader".to_string()]));
        assert!(any_auth.is_authorized(&[])); // Still true - any auth

        let admin_only = RequireAuthDirective::with_roles(vec!["Admin".to_string()]);
        assert!(admin_only.is_authorized(&["Admin".to_string()]));
        assert!(!admin_only.is_authorized(&["Reader".to_string()]));

        let editor_or_admin =
            RequireAuthDirective::with_roles(vec!["Admin".to_string(), "Editor".to_string()]);
        assert!(editor_or_admin.is_authorized(&["Editor".to_string()]));
        assert!(editor_or_admin.is_authorized(&["Admin".to_string()]));
        assert!(!editor_or_admin.is_authorized(&["Reader".to_string()]));
    }

    #[test]
    fn test_parsed_directives() {
        let mut directives = ParsedDirectives::new();
        assert!(!directives.has_streaming());
        assert!(!directives.requires_auth());

        directives.defer = Some(DeferDirective::labeled("test"));
        assert!(directives.has_streaming());

        directives.require_auth =
            Some(RequireAuthDirective::with_roles(vec!["Admin".to_string()]));
        assert!(directives.requires_auth());
        assert_eq!(
            directives.required_roles(),
            Some(["Admin".to_string()].as_slice())
        );
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(30));
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("2h"), Some(7200));
        assert_eq!(parse_duration("1d"), Some(86400));
        assert_eq!(parse_duration("invalid"), None);
    }
}

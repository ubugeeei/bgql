# Better GraphQL Specification - Directives

## 1. Overview

Directives are a mechanism to attach additional metadata or behavior to schema definitions or queries. They start with the `@` symbol.

## 2. Directive Definition

### 2.1 Syntax

```graphql
directive @directiveName(
  arg1: Type = defaultValue
  arg2: Option<Type>
) repeatable? on LOCATION | LOCATION2
```

### 2.2 Locations

| Location | Description |
|----------|-------------|
| `SCHEMA` | Schema definition |
| `SCALAR` | Scalar type definition |
| `OBJECT` | Object type definition |
| `FIELD_DEFINITION` | Field definition |
| `ARGUMENT_DEFINITION` | Argument definition |
| `INTERFACE` | Interface type definition |
| `UNION` | Union type definition |
| `ENUM` | Enum type definition |
| `ENUM_VALUE` | Enum value |
| `INPUT_OBJECT` | Input type definition |
| `INPUT_FIELD_DEFINITION` | Input field definition |
| `QUERY` | Query operation |
| `MUTATION` | Mutation operation |
| `SUBSCRIPTION` | Subscription operation |
| `FIELD` | Field in query |
| `FRAGMENT_DEFINITION` | Fragment definition |
| `FRAGMENT_SPREAD` | Fragment spread |
| `INLINE_FRAGMENT` | Inline fragment |
| `VARIABLE_DEFINITION` | Variable definition |

## 3. Built-in Directives

### 3.1 @deprecated

Indicates that a field or enum value is deprecated.

```graphql
directive @deprecated(
  """Reason for deprecation"""
  reason: String = "No longer supported"
) on FIELD_DEFINITION | ENUM_VALUE | ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
type User {
  avatar: String @deprecated(reason: "Use avatarUrl instead")
  avatarUrl: String
}

enum Status {
  ACTIVE
  INACTIVE @deprecated(reason: "Use DISABLED instead")
  DISABLED
}
```

### 3.2 @timezone

Specifies the timezone for DateTime fields.

```graphql
directive @timezone(
  """IANA timezone name"""
  tz: String
) on FIELD_DEFINITION | ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
type Event {
  # Stored in UTC, returned in specified timezone
  startTime: DateTime @timezone(tz: "Asia/Tokyo")

  # Client-specified timezone
  localStartTime(tz: String): DateTime
}
```

### 3.3 @patch

Indicates that an input type is for partial updates of a target type.

```graphql
directive @patch(
  """Target type"""
  type: String
) on INPUT_OBJECT

# Example
input UpdateUserInput @patch(type: "User") {
  name: Option<String>
  email: Option<String>
  bio: Option<String>
}
```

### 3.4 @put

Indicates that an input type is for complete replacement of a target type.

```graphql
directive @put(
  """Target type"""
  type: String
) on INPUT_OBJECT

# Example
input ReplaceUserInput @put(type: "User") {
  name: String
  email: String
  bio: Option<String>
}
```

## 4. Validation Directives

### 4.1 Numeric Validation

```graphql
directive @min(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION
directive @max(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION
directive @range(min: Int, max: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION
directive @positive on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @negative on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @nonZero on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
input PaginationInput {
  page: Int @min(1)
  perPage: Int @range(min: 1, max: 100) = 20
}

input ProductInput {
  price: Float @positive
  discount: Float @range(min: 0, max: 100)
  quantity: Int @min(0)
}
```

### 4.2 String Validation

```graphql
directive @minLength(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION
directive @maxLength(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION
directive @length(min: Option<Int>, max: Option<Int>) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @pattern(regex: String, flags: Option<String>) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @trim on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @lowercase on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @uppercase on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
input CreateUserInput {
  username: String @minLength(3) @maxLength(20) @pattern(regex: "^[a-zA-Z0-9_]+$")
  displayName: String @length(min: 1, max: 50) @trim
}
```

### 4.3 Format Validation

```graphql
directive @email on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @url on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @uuid on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @ipv4 on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @ipv6 on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @phone on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @creditCard on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
input ContactInput {
  email: String @email
  website: Option<String> @url
  phone: Option<String> @phone
}

input PaymentInput {
  cardNumber: String @creditCard
  transactionId: String @uuid
}
```

### 4.4 Empty Value Validation

```graphql
"""
Allows empty strings or empty lists.
By default, non-null strings/lists must have content.
"""
directive @allowEmpty on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION

"""
Explicitly requires non-empty value (default behavior, for documentation).
"""
directive @notEmpty(
  """Custom error message"""
  message: Option<String>
) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION | FIELD_DEFINITION

# Example
input CreatePostInput {
  """Title is required and cannot be empty"""
  title: String @notEmpty

  """Content can be an empty string"""
  content: String @allowEmpty

  """Tags list can be empty []"""
  tags: List<String> @allowEmpty

  """Categories must have at least one item (default behavior)"""
  categories: List<CategoryId>
}

input SearchInput {
  """Empty string means "search all" """
  query: String @allowEmpty

  """Filters can be empty list"""
  filters: List<Filter> @allowEmpty
}

# Combined with other validations
input ProfileInput {
  """Bio can be empty, but if provided, max 500 chars"""
  bio: String @allowEmpty @maxLength(500)

  """Skills can be empty list, but each item is validated"""
  skills: List<String> @allowEmpty @maxLength(50)
}
```

### 4.5 List Validation

```graphql
directive @minItems(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @maxItems(value: Int) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @unique on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION
directive @sorted(order: SortOrder = Asc) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

enum SortOrder {
  Asc
  Desc
}

# Example
input CreatePostInput {
  title: String
  tags: List<String> @minItems(1) @maxItems(10) @unique
  mentions: List<ID> @maxItems(50)
  priorityIds: List<Int> @sorted(order: Desc)
}
```

### 4.6 Custom Validation

```graphql
directive @validate(
  """Validation function name"""
  fn: String
  """Error message"""
  message: Option<String>
) on ARGUMENT_DEFINITION | INPUT_FIELD_DEFINITION

# Example
input TransferInput {
  fromAccount: ID
  toAccount: ID @validate(fn: "notEqual", message: "Cannot transfer to the same account")
  amount: Float @positive
}
```

### 4.6 HTML Sanitization

The `@sanitize` directive configures HTML sanitization for `HTML` type fields.

```graphql
directive @sanitize(
  """Tags to allow (overrides default)"""
  allowTags: Option<List<String>>

  """Additional tags to allow (extends default)"""
  addTags: Option<List<String>>

  """Tags to explicitly deny"""
  denyTags: Option<List<String>>

  """Attributes to allow per tag (JSON object)"""
  allowAttributes: Option<JSON>

  """Allow style attribute"""
  allowStyle: Boolean = false

  """Allowed CSS properties when allowStyle is true"""
  allowedStyleProperties: Option<List<String>>

  """Allow data: URLs for specified tags"""
  allowDataUrls: Option<List<String>>

  """Custom URL schemes to allow"""
  allowUrlSchemes: List<String> = ["http", "https", "mailto"]
) on FIELD_DEFINITION | INPUT_FIELD_DEFINITION | ARGUMENT_DEFINITION

# Example
type Post {
  # Basic sanitization (default)
  content: HTML

  # Minimal: text formatting only
  summary: HTML @sanitize(
    allowTags: ["p", "br", "b", "i", "a"]
  )

  # Rich: with images and more formatting
  richContent: HTML @sanitize(
    addTags: ["img", "video"],
    allowAttributes: {
      "img": ["src", "alt", "width", "height"],
      "video": ["src", "controls", "width", "height"]
    },
    allowDataUrls: ["img"]
  )
}

input CreatePostInput {
  # Sanitize user input
  content: HTML @sanitize(
    allowTags: ["p", "br", "b", "i", "u", "a", "ul", "ol", "li", "blockquote"],
    allowAttributes: { "a": ["href"] }
  )
}
```

## 5. Streaming Directives

### 5.1 @defer

Defers field resolution and returns via streaming.

```graphql
directive @defer(
  """Label for the deferred fragment"""
  label: Option<String>
  """Priority (1 is highest, larger numbers are lower priority)"""
  priority: Int = 1
  """Conditional deferral"""
  if: Boolean = true
) on FRAGMENT_SPREAD | INLINE_FRAGMENT

# Example
query GetUser($id: ID) {
  user(id: $id) {
    id
    name
    ... @defer(label: "profile", priority: 1) {
      bio
      avatarUrl
    }
    ... @defer(label: "stats", priority: 2) {
      postsCount
      followersCount
    }
  }
}
```

### 5.2 @stream

Streams list elements sequentially.

```graphql
directive @stream(
  """Label"""
  label: Option<String>
  """Number of elements to return initially"""
  initialCount: Int = 0
  """Priority"""
  priority: Int = 1
  """Conditional streaming"""
  if: Boolean = true
) on FIELD

# Example
query GetUserPosts($userId: ID) {
  user(id: $userId) {
    name
    posts @stream(initialCount: 5, priority: 1) {
      id
      title
      content @defer(priority: 2)
    }
  }
}
```

### 5.3 Priority Behavior

Priority starts at 1, with smaller numbers indicating higher priority.

```
Priority 1: Begin resolution immediately
Priority 2: Resolve after Priority 1
Priority 3: Resolve after Priority 2
...
```

## 6. HTTP Directives

### 6.1 @cors (Schema-level)

Defines CORS configuration.

```graphql
directive @cors(
  """Allowed origins"""
  origins: List<String> = ["*"]
  """Allowed methods"""
  methods: List<HTTPMethod> = [Get, Post, Options]
  """Allowed headers"""
  allowHeaders: Option<List<String>>
  """Exposed headers"""
  exposeHeaders: Option<List<String>>
  """Allow credentials"""
  credentials: Boolean = false
  """Preflight cache time (seconds)"""
  maxAge: Int = 86400
) on SCHEMA

enum HTTPMethod {
  GET
  POST
  PUT
  DELETE
  PATCH
  OPTIONS
  HEAD
}

# Example
schema @cors(
  origins: ["https://app.example.com", "https://admin.example.com"],
  credentials: true,
  allowHeaders: ["Authorization", "X-Request-ID"],
  exposeHeaders: ["X-RateLimit-Remaining"]
) {
  query: Query
  mutation: Mutation
}
```

### 6.2 @cookie

Defines cookie read/write operations.

```graphql
directive @cookie(
  """Cookie name"""
  name: String
  """Cookie options (for writing)"""
  options: Option<CookieOptions>
) on FIELD_DEFINITION | ARGUMENT_DEFINITION

input CookieOptions {
  maxAge: Option<Int>
  expires: Option<DateTime>
  path: String = "/"
  domain: Option<String>
  secure: Boolean = true
  httpOnly: Boolean = true
  sameSite: SameSite = STRICT
}

enum SameSite {
  STRICT
  LAX
  NONE
}

# Example
type Query {
  """Get user from session cookie"""
  currentUser @cookie(name: "session"): Option<User>
}

type Mutation {
  """Set session cookie after login"""
  login(input: LoginInput): LoginResult
    @cookie(name: "session", options: {
      maxAge: 604800,  # 7 days
      httpOnly: true,
      secure: true,
      sameSite: STRICT
    })
}
```

### 6.3 @header

Defines HTTP header read/write operations.

```graphql
directive @header(
  """Header name"""
  name: String
  """Write mode"""
  write: Boolean = false
) on FIELD_DEFINITION | ARGUMENT_DEFINITION

# Example
type Query {
  """Authenticate using Authorization header"""
  me @header(name: "Authorization"): Option<User>

  """Get request ID"""
  requestInfo @header(name: "X-Request-ID"): RequestInfo
}

type Mutation {
  """Set token in response header"""
  refreshToken: RefreshTokenResult
    @header(name: "X-New-Token", write: true)
}
```

### 6.4 @requireAuth

Indicates that a field requires authentication.

```graphql
directive @requireAuth(
  """Required roles"""
  roles: Option<List<String>>
  """Required scopes"""
  scopes: Option<List<String>>
) on FIELD_DEFINITION | OBJECT

# Example
type Query {
  publicPosts: List<Post>
  myPosts: List<Post> @requireAuth
  adminDashboard: Dashboard @requireAuth(roles: ["Admin"])
}

type User @requireAuth {
  id: ID
  name: String
  email: String  # Authentication required to access this field
}
```

## 7. Cache Directives

### 7.1 @cache

Defines field caching strategy.

```graphql
directive @cache(
  """Cache duration (seconds)"""
  maxAge: Int
  """Cache scope"""
  scope: CacheScope = PUBLIC
  """Stale-While-Revalidate (seconds)"""
  swr: Option<Int>
  """Headers to include in cache key"""
  vary: Option<List<String>>
) on FIELD_DEFINITION

enum CacheScope {
  PUBLIC   # Cacheable by CDN
  PRIVATE  # User-specific, no CDN
}

# Example
type Query {
  """1 hour public cache"""
  popularPosts: List<Post> @cache(maxAge: 3600, scope: PUBLIC)

  """5 minute private cache"""
  myFeed: List<Post> @cache(maxAge: 300, scope: PRIVATE)

  """SWR pattern"""
  settings: Settings @cache(maxAge: 60, swr: 3600)
}
```

### 7.2 @noCache

Disables caching.

```graphql
directive @noCache on FIELD_DEFINITION

# Example
type Query {
  """Always fetch fresh data"""
  realTimeStats: Stats @noCache
}
```

## 8. Media Directives

### 8.1 @signedUrl

Generates signed URLs for secure media access.

```graphql
directive @signedUrl(
  """Expiration time in seconds"""
  expiresIn: Uint = 3600

  """Allowed HTTP methods"""
  methods: List<HTTPMethod> = [Get]

  """IP address restriction (CIDR notation)"""
  ipRange: Option<String>

  """Custom claims to include"""
  claims: Option<JSON>
) on FIELD_DEFINITION

# Example
type File {
  """Public URL (no signing)"""
  url: String

  """Signed URL for secure access"""
  secureUrl: String @signedUrl(expiresIn: 7200)
}

type PrivateDocument {
  """Signed URL with IP restriction"""
  downloadUrl: String @signedUrl(
    expiresIn: 300,
    methods: [Get],
    ipRange: "10.0.0.0/8"
  )
}
```

### 8.2 @resize

On-the-fly image resizing.

```graphql
directive @resize(
  """Target width"""
  width: Option<Uint>

  """Target height"""
  height: Option<Uint>

  """Fit mode"""
  fit: ImageFit = Cover

  """Output format"""
  format: Option<ImageFormat>

  """Quality (1-100)"""
  quality: Uint = 80
) on FIELD_DEFINITION

# Example
type User {
  """Thumbnail avatar (100x100)"""
  avatarThumb: String @resize(width: 100, height: 100, fit: Cover)

  """Medium avatar (300x300, WebP format)"""
  avatarMedium: String @resize(width: 300, height: 300, format: WebP, quality: 85)
}
```

### 8.3 @transcode

Media transcoding configuration.

```graphql
directive @transcode(
  """Target format"""
  format: Option<String>

  """Target bitrate"""
  bitrate: Option<Uint>

  """Target quality preset"""
  preset: Option<TranscodePreset>
) on FIELD_DEFINITION

enum TranscodePreset {
  Low      # 480p, low bitrate
  Medium   # 720p, medium bitrate
  High     # 1080p, high bitrate
  Ultra    # 4K, high bitrate
}

# Example
type Video {
  """Original video"""
  original: String

  """720p version"""
  medium: String @transcode(preset: Medium)

  """Mobile-optimized version"""
  mobile: String @transcode(format: "mp4", bitrate: 1000)
}
```

### 8.4 @hls

HLS streaming configuration.

```graphql
directive @hls(
  """Segment duration in seconds"""
  segmentDuration: Uint = 6

  """Playlist type"""
  playlistType: HlsPlaylistType = Vod

  """Include variants"""
  variants: List<HlsVariant> = [
    { height: 360, bitrate: 800 },
    { height: 480, bitrate: 1400 },
    { height: 720, bitrate: 2800 },
    { height: 1080, bitrate: 5000 }
  ]
) on FIELD_DEFINITION

enum HlsPlaylistType {
  Vod     # Video on demand
  Event   # Live event (can be rewound)
  Live    # Live stream (no rewind)
}

input HlsVariant {
  height: Uint
  bitrate: Uint
}

# Example
type Video {
  """HLS master playlist URL"""
  hlsUrl: String @hls(
    segmentDuration: 4,
    playlistType: Vod,
    variants: [
      { height: 480, bitrate: 1000 },
      { height: 720, bitrate: 2500 },
      { height: 1080, bitrate: 5000 }
    ]
  )
}
```

## 9. Server-side Fragment Directives

### 9.1 @server

Defines a server-managed fragment.

```graphql
directive @server on FRAGMENT_DEFINITION

fragment UserCard on User @server {
  id
  name
  avatarUrl
}
```

### 9.2 @version

Specifies fragment version.

```graphql
directive @version(value: String) on FRAGMENT_DEFINITION

fragment UserCard on User @server @version("2024-01") {
  id
  name
  avatarUrl
  verified  # New field
}
```

## 10. Directive Composition

Multiple directives can be combined.

```graphql
input CreateUserInput {
  email: String @trim @lowercase @email
  password: String @minLength(8) @pattern(regex: "^(?=.*[A-Za-z])(?=.*\\d).+$")
  age: Option<Int> @range(min: 0, max: 150)
}

type Query {
  user(id: ID): UserResult
    @requireAuth
    @cache(maxAge: 60, scope: PRIVATE)

  adminUsers: List<User>
    @requireAuth(roles: ["ADMIN"])
    @cache(maxAge: 300, scope: PRIVATE)
}
```

## 11. Custom Directive Definition

```graphql
"""
Transform field values
"""
directive @transform(
  """Transform function name"""
  fn: String
  """Transform parameters"""
  params: Option<JSON>
) on FIELD_DEFINITION

# Example
type User {
  name: String @transform(fn: "capitalize")
  bio: Option<String> @transform(fn: "sanitize", params: { allowedTags: ["b", "i"] })
}
```

## 12. Advanced Streaming Directives

### 12.1 @binary

Marks a field as returning binary stream data for media or file content.

```graphql
directive @binary(
  """Enable progressive download/playback"""
  progressive: Boolean = false

  """Chunk size for streaming (bytes)"""
  chunkSize: Uint = 65536
) on FIELD_DEFINITION

# Example
type Audio {
  format: AudioFormat
  duration: Float
  url: String

  """Binary stream for progressive playback"""
  stream: BinaryStream @binary(progressive: true, chunkSize: 32768)
}

type Video {
  format: VideoFormat
  width: Uint
  height: Uint

  """Binary stream with chunked transfer"""
  stream: BinaryStream @binary(progressive: true)
  hlsUrl: String
}
```

### 12.2 @resumable

Enables pause/resume functionality for long-running queries.

```graphql
directive @resumable(
  """Time-to-live for checkpoints (seconds)"""
  ttl: Uint = 3600

  """Interval for creating checkpoints (items processed)"""
  checkpointInterval: Uint = 50
) on QUERY

# Example
query GetLargeFeed @resumable(ttl: 7200, checkpointInterval: 100) {
  feed @stream(initialCount: 10) {
    id
    content
    media {
      ... on Video {
        stream @binary
      }
    }
  }
}
```

### 12.3 @priority (Extended)

Enhanced priority directive for query scheduling.

```graphql
directive @priority(
  """Priority level (1 is highest, 10 is lowest)"""
  level: Int = 5

  """Optional deadline for completion"""
  deadline: Option<DateTime>

  """Whether this task can be preempted by higher priority tasks"""
  preemptible: Boolean = true
) on QUERY | MUTATION | FIELD

# Example
query GetCriticalData @priority(level: 1, preemptible: false) {
  urgentNotifications {
    id
    message
  }
}

query GetAnalytics @priority(level: 8, deadline: "2024-01-01T00:00:00Z") {
  statistics {
    views
    engagement
  }
}
```

### 12.4 @resources

Specifies resource requirements for query scheduling.

```graphql
directive @resources(
  """CPU usage (0.0-1.0)"""
  cpu: Float

  """Memory requirement (bytes)"""
  memory: Uint

  """I/O intensity level"""
  io: ResourceLevel = LOW

  """Network intensity level"""
  network: ResourceLevel = LOW
) on FIELD_DEFINITION

enum ResourceLevel {
  LOW
  MEDIUM
  HIGH
}

# Example
type Query {
  """Light query - minimal resources"""
  user(id: ID): User @resources(cpu: 0.1, memory: 1048576)

  """Heavy query - significant resources"""
  generateReport(params: ReportParams): Report @resources(
    cpu: 0.8,
    memory: 536870912,
    io: HIGH
  )
}
```

## 13. Component Model Directives

### 13.1 @server (Extended)

Extended server-side fragment directive with caching and isolation.

```graphql
directive @server(
  """Execute only on server side"""
  isolate: Boolean = true

  """Cache strategy for the fragment"""
  cache: CacheStrategy = NONE

  """Allow pre-rendering of this fragment"""
  prerender: Boolean = false
) on FRAGMENT_DEFINITION

enum CacheStrategy {
  NONE      # No caching
  REQUEST   # Cache per request
  USER      # Cache per user session
  GLOBAL    # Global cache
}

# Example
fragment UserProfile on User @server(cache: USER, prerender: true) {
  id
  name
  avatar {
    url
    blurHash
  }
  ... @defer(label: "bio") {
    bio
    socialLinks
  }
}
```

### 13.2 @boundary

Defines client-server boundaries for data isolation.

```graphql
directive @boundary(
  """Server-only field (never sent to client)"""
  server: Boolean = false

  """Client-only field"""
  client: Boolean = false

  """Serialization strategy when crossing boundary"""
  serialize: SerializeStrategy = JSON
) on OBJECT | FIELD_DEFINITION

enum SerializeStrategy {
  JSON       # Standard JSON serialization
  BINARY     # Binary serialization (msgpack)
  REFERENCE  # ID reference only (fetch separately)
}

# Example
type User {
  id: ID
  name: String
  email: String

  """Never sent to client"""
  passwordHash: String @boundary(server: true)

  """Sensitive data - server only"""
  internalNotes: String @boundary(server: true)

  """Large data - fetch separately if needed"""
  activityLog: List<Activity> @boundary(serialize: REFERENCE)
}
```

### 13.3 @island

Defines interactive islands for partial hydration.

```graphql
directive @island(
  """Island identifier"""
  name: String

  """Hydration strategy"""
  hydrate: HydrationStrategy = VISIBLE

  """Client bundle to load for this island"""
  clientBundle: Option<String>
) on FRAGMENT_DEFINITION

enum HydrationStrategy {
  IMMEDIATE    # Hydrate immediately on page load
  IDLE         # Hydrate during browser idle time
  VISIBLE      # Hydrate when scrolled into view
  INTERACTION  # Hydrate on user interaction
  NEVER        # Static content, no hydration
}

# Example
fragment CommentSection on Post @island(
  name: "comments",
  hydrate: VISIBLE,
  clientBundle: "comments.js"
) {
  comments @stream(initialCount: 5) {
    id
    author { name avatarUrl }
    content
    createdAt
  }
  commentCount
}

fragment ShareButtons on Post @island(
  name: "share",
  hydrate: INTERACTION
) {
  shareUrl
  title
  platforms
}
```

### 13.4 @hydrate

Specifies hydration behavior for fragment spreads.

```graphql
directive @hydrate(
  """Hydration strategy"""
  strategy: HydrationStrategy = IDLE

  """Hydration priority"""
  priority: HydrationPriority = NORMAL
) on FRAGMENT_SPREAD | INLINE_FRAGMENT

enum HydrationPriority {
  CRITICAL  # Must hydrate first
  HIGH      # High priority
  NORMAL    # Default priority
  LOW       # Can wait
}

# Example
query GetPage($id: ID) {
  page(id: $id) {
    title
    content

    # Critical interactive elements - hydrate immediately
    ...NavigationMenu @hydrate(strategy: IMMEDIATE, priority: CRITICAL)

    # Comments - hydrate when visible
    ...CommentSection @hydrate(strategy: VISIBLE, priority: NORMAL)

    # Share buttons - hydrate on interaction
    ...ShareButtons @hydrate(strategy: INTERACTION, priority: LOW)
  }
}
```

## 14. Client-Server Isolation (Experimental Vision)

> **Note:** This section describes experimental features and future design goals inspired by React Server Components. The Vue-specific syntax extensions described here are not yet implemented and represent a long-term vision for the project.

### 14.1 Design Philosophy

BGQL's Client-Server Isolation aims to provide React Server Components-like capabilities in a framework-agnostic way, with Vue as the first-class implementation target.

**Core Principles:**

1. **Server-resolved queries stay on server** - Data that doesn't need to reach the client is resolved server-side and only the rendered HTML is sent
2. **Minimal client payload** - Only hydration-necessary JavaScript and data are sent to the client
3. **Clear boundaries** - Explicit demarcation between server and client code/data

### 14.2 Server Field Isolation

```graphql
type User {
  id: ID
  name: String
  email: String

  # Server-only fields - resolved on server, never serialized to client
  passwordHash: String @boundary(server: true)

  # Used for server-side rendering, but result (HTML) is sent to client
  formattedBio: HTML @server(renderOnly: true)
}

# Server Fragment - entire fragment resolved server-side
fragment UserCard on User @server {
  id
  name
  avatar {
    url
    blurHash  # For placeholder
  }
}
```

### 14.3 Vue Integration (Future Vision)

> **⚠️ Experimental:** The following Vue syntax extensions are aspirational and not yet implemented. They represent the long-term vision for deep Vue integration.

#### 14.3.1 `<script server>` (Proposed)

A new script block that executes only on the server during SSR:

```vue
<!-- UserProfile.vue -->
<script server>
// This code runs ONLY on the server
// Never bundled for client, never sent to browser

import { useServerFragment } from '@bgql/client/vue'
import { sendEmail } from '~/server/email'  // Server-only import

const props = defineServerProps<{
  userId: string
}>()

// Server-side data fetching with @server fragment
const { data } = await useServerFragment(USER_PROFILE_FRAGMENT, {
  id: props.userId
})

// Server-side logic (e.g., logging, notifications)
if (data.user.needsVerification) {
  await sendEmail(data.user.email, 'verify')
}

// Only `user` is exposed to template, email logic stays on server
defineExpose({ user: data.user })
</script>

<script setup>
// Client-side script - runs on both server (SSR) and client (hydration)
import { ref } from 'vue'

const isFollowing = ref(false)
const toggleFollow = () => { /* client-side interaction */ }
</script>

<template>
  <!-- Template receives server-resolved data -->
  <div class="profile">
    <img :src="user.avatar.url" />
    <h1>{{ user.name }}</h1>

    <!-- Client-interactive island -->
    <button @click="toggleFollow">
      {{ isFollowing ? 'Unfollow' : 'Follow' }}
    </button>
  </div>
</template>
```

#### 14.3.2 `defineServerProps` (Proposed)

Type-safe props that are only available during server-side execution:

```typescript
// Proposed API
const props = defineServerProps<{
  // Available on server during SSR
  userId: string
  sessionToken: string  // Sensitive - never sent to client
}>()

// After SSR, only non-sensitive data is serialized for hydration
```

#### 14.3.3 Server Actions (Proposed)

Server-side mutations invocable from client components:

```vue
<script server>
// Server action - executes on server, callable from client
const updateProfile = defineServerAction(async (input: UpdateProfileInput) => {
  // Runs on server with full access to server resources
  const result = await db.users.update(input)
  return result
})
</script>

<script setup>
// Client can call server action
const onSubmit = async (formData) => {
  // This triggers a server request, not client-side execution
  await updateProfile(formData)
}
</script>
```

### 14.4 Hydration Strategies

Combined with `@island` and `@hydrate` directives:

```vue
<template>
  <article>
    <!-- Static: rendered on server, no hydration -->
    <header>{{ article.title }}</header>
    <div v-html="article.content" />

    <!-- Island: hydrated when visible -->
    <Bgql.Defer
      label="comments"
      :hydrate="{ strategy: 'visible', priority: 'normal' }"
    >
      <CommentSection :articleId="article.id" />
    </Bgql.Defer>

    <!-- Island: hydrated on interaction -->
    <Bgql.Defer
      label="share"
      :hydrate="{ strategy: 'interaction' }"
    >
      <ShareButtons :url="article.shareUrl" />
    </Bgql.Defer>
  </article>
</template>
```

### 14.5 Implementation Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| 1 | `@boundary` directive runtime | ✅ Designed |
| 2 | `@server` fragment execution | 🚧 In Progress |
| 3 | Vite plugin SSR integration | ✅ Basic |
| 4 | `<script server>` transform | 📋 Planned |
| 5 | `defineServerProps` macro | 📋 Planned |
| 6 | Server Actions | 📋 Future |

## 15. Module System Directives

### 14.1 @mod

Declares a module within the schema.

```graphql
directive @mod(
  """Module path (e.g., "users", "users::auth")"""
  path: String
) on SCHEMA

# Example
# In users.graphql
schema @mod(path: "users") {
  query: Query
}

# In users/auth.graphql
schema @mod(path: "users::auth") {
  query: Query
}
```

### 14.2 @use

Imports types from other modules.

```graphql
directive @use(
  """Module path to import from"""
  from: String

  """Specific items to import (empty = all public items)"""
  items: Option<List<String>>

  """Import all public items (glob import)"""
  glob: Boolean = false

  """Alias for imported items"""
  as: Option<String>
) on SCHEMA

# Example
# Import specific types
schema @use(from: "users", items: ["User", "UserInput"]) {
  query: Query
}

# Import all public types
schema @use(from: "common", glob: true) {
  query: Query
}

# Import with alias
schema @use(from: "users", items: ["User"], as: "AuthUser") {
  query: Query
}
```

### 14.3 @pub

Controls visibility of types across modules.

```graphql
directive @pub(
  """Visibility level"""
  visibility: Visibility = PUBLIC
) on OBJECT | INPUT_OBJECT | ENUM | SCALAR | INTERFACE | UNION

enum Visibility {
  PUBLIC   # Visible everywhere
  CRATE    # Visible within the same crate/package
  SUPER    # Visible to parent module
  PRIVATE  # Visible only within same module (default without @pub)
}

# Example
# Public type - visible everywhere
type User @pub {
  id: ID
  name: String
}

# Crate-internal type
type InternalConfig @pub(visibility: CRATE) {
  setting: String
}

# Module-private type (default)
type PrivateHelper {
  data: String
}
```

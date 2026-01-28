# Better GraphQL Specification - Type System

## 1. Overview

Better GraphQL's type system extends GraphQL's type system to provide stronger type safety and expressiveness.

## 2. Scalar Types

### 2.1 Built-in Scalar Types

| Type | Description | Examples |
|------|-------------|----------|
| `String` | UTF-8 string | `"Hello, World!"` |
| `Int` | 32-bit signed integer | `42`, `-1` |
| `Uint` | 32-bit unsigned integer | `0`, `42`, `4294967295` |
| `Float` | 64-bit floating point | `3.14`, `-0.5` |
| `Boolean` | Boolean value | `true`, `false` |
| `ID` | Unique identifier (String internally) | `"user_123"` |
| `Date` | Date without time (ISO 8601) | `"2024-01-15"` |
| `DateTime` | Date and time (ISO 8601) | `"2024-01-15T10:30:00Z"` |
| `JSON` | Arbitrary JSON value | `{"key": "value"}`, `[1, 2, 3]` |
| `HTML` | Sanitized HTML string | `"<p>Hello <b>World</b></p>"` |
| `TrustedHTML` | DOM Trusted Types HTML | XSS-safe HTML for innerHTML |
| `TrustedScript` | DOM Trusted Types Script | XSS-safe script content |
| `TrustedScriptURL` | DOM Trusted Types Script URL | XSS-safe script src |
| `Void` | No return value | (no value) |
| `File` | File upload/download | Binary data with metadata |
| `Image` | Image with variants | PNG, JPEG, WebP, AVIF, etc. |
| `Audio` | Audio stream/file | MP3, WAV, OGG, etc. |
| `Video` | Video stream/file | MP4, WebM, HLS, etc. |
| `Stream` | Binary stream | Chunked binary data |

### 2.2 Built-in Date/Time Types

Better GraphQL provides built-in date/time type support.

#### Date

Represents a date only (no time information). ISO 8601 format.

```graphql
type Event {
  date: Date  # "2024-01-15"
}
```

**Serialization**: `YYYY-MM-DD`

#### DateTime

Represents date and time. UTC by default.

```graphql
type Event {
  createdAt: DateTime  # "2024-01-15T10:30:00Z"
  localTime: DateTime @timezone(tz: "Asia/Tokyo")  # "2024-01-15T19:30:00+09:00"
}
```

**Serialization**: ISO 8601 extended format

### 2.3 Built-in JSON Type

The `JSON` type represents arbitrary JSON data. This is useful for dynamic or unstructured data where a strict schema is not practical.

```graphql
type User {
  id: ID
  name: String
  preferences: JSON    # {"theme": "dark", "language": "en"}
  metadata: Option<JSON>  # Nullable JSON
}

type Notification {
  id: ID
  type: String
  payload: JSON        # Structure varies by notification type
}
```

**Serialization**: Any valid JSON value (object, array, string, number, boolean, null)

#### Use Cases

- **Dynamic metadata**: User preferences, feature flags, configuration
- **Third-party integrations**: Webhook payloads, external API responses
- **Polymorphic data**: When structure varies significantly by context
- **Migration**: Temporary escape hatch when migrating from untyped systems

#### Caution

While `JSON` provides flexibility, it bypasses type safety. Prefer strongly-typed fields when:
- The structure is known and stable
- Type safety is important for clients
- The data will be queried or filtered on specific fields

```graphql
# Prefer this (strongly typed)
type UserPreferences {
  theme: Theme
  language: String
  notifications: NotificationSettings
}

# Over this (untyped)
type User {
  preferences: JSON
}
```

### 2.4 Built-in HTML Type

The `HTML` type represents sanitized HTML content. All HTML values are automatically sanitized to prevent XSS attacks.

```graphql
type Post {
  id: ID
  title: String
  content: HTML           # Rich text content, auto-sanitized
  summary: Option<HTML>   # Nullable HTML
}

type Comment {
  id: ID
  body: HTML              # User-generated HTML content
}
```

**Serialization**: Sanitized HTML string

#### Default Sanitization

By default, `HTML` allows safe tags and removes potentially dangerous content:

**Allowed by default**:
- Text formatting: `<p>`, `<br>`, `<b>`, `<i>`, `<u>`, `<s>`, `<strong>`, `<em>`, `<mark>`
- Lists: `<ul>`, `<ol>`, `<li>`
- Headings: `<h1>` - `<h6>`
- Links: `<a>` (with `href`, `rel`, `target` attributes)
- Media: `<img>` (with `src`, `alt`, `width`, `height` attributes)
- Structure: `<div>`, `<span>`, `<blockquote>`, `<pre>`, `<code>`
- Tables: `<table>`, `<thead>`, `<tbody>`, `<tr>`, `<th>`, `<td>`

**Always removed**:
- Scripts: `<script>`, `onclick`, `onerror`, etc.
- Frames: `<iframe>`, `<frame>`, `<object>`, `<embed>`
- Forms: `<form>`, `<input>`, `<button>`
- Dangerous attributes: `javascript:` URLs, `data:` URLs (except images)
- Style injections: `<style>`, `style` attribute (configurable)

#### Custom Sanitization with @sanitize

Use the `@sanitize` directive to customize allowed tags and attributes:

```graphql
type Post {
  # Minimal: Only basic text formatting
  content: HTML @sanitize(
    allowTags: ["p", "br", "b", "i", "a"],
    allowAttributes: { "a": ["href"] }
  )
}

type Article {
  # Rich: Allow more tags including images and embeds
  content: HTML @sanitize(
    allowTags: ["p", "br", "b", "i", "a", "img", "h1", "h2", "h3", "ul", "ol", "li", "blockquote", "pre", "code"],
    allowAttributes: {
      "a": ["href", "rel", "target"],
      "img": ["src", "alt", "width", "height"]
    },
    allowDataUrls: ["img"]
  )
}

type TrustedContent {
  # Allow style attribute (use with caution)
  content: HTML @sanitize(
    allowStyle: true,
    allowedStyleProperties: ["color", "background-color", "font-size", "text-align"]
  )
}
```

#### @sanitize Directive Definition

```graphql
directive @sanitize(
  """Tags to allow (overrides default)"""
  allowTags: Option<List<String>>

  """Additional tags to allow (extends default)"""
  addTags: Option<List<String>>

  """Tags to explicitly deny"""
  denyTags: Option<List<String>>

  """Attributes to allow per tag"""
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
```

#### Input Sanitization

HTML is also sanitized on input:

```graphql
input CreatePostInput {
  title: String
  content: HTML @sanitize(allowTags: ["p", "br", "b", "i", "a", "ul", "ol", "li"])
}

type Mutation {
  createPost(input: CreatePostInput): Post
}
```

When a client sends:
```json
{
  "content": "<p>Hello</p><script>alert('xss')</script><b>World</b>"
}
```

The server receives sanitized content:
```json
{
  "content": "<p>Hello</p><b>World</b>"
}
```

### 2.5 Built-in Trusted Types

Better GraphQL provides first-class support for [DOM Trusted Types](https://developer.mozilla.org/en-US/docs/Web/API/Trusted_Types_API) to prevent DOM XSS attacks at the type level.

#### TrustedHTML

Represents HTML content that is safe to insert into the DOM via `innerHTML`, `outerHTML`, `insertAdjacentHTML`, etc.

```graphql
type Article {
  id: ArticleId
  title: String

  """Content that is safe for innerHTML"""
  content: TrustedHTML

  """Optional rich text"""
  summary: Option<TrustedHTML>
}
```

**Server-side**: Content is sanitized and marked as trusted before returning.
**Client-side**: The value satisfies the Trusted Types API and can be safely used with DOM sinks.

#### TrustedScript

Represents script content that is safe to execute.

```graphql
type DynamicWidget {
  id: WidgetId

  """Script that is safe to execute"""
  initScript: Option<TrustedScript>
}
```

**Usage**: Safe to assign to `script.text`, pass to `eval()` (where allowed), etc.

#### TrustedScriptURL

Represents a URL that is safe to load as a script source.

```graphql
type ExternalResource {
  id: ResourceId

  """URL safe to use as script src"""
  scriptUrl: TrustedScriptURL
}
```

**Usage**: Safe to assign to `script.src`, `Worker()` constructor, etc.

#### Type Definitions

```graphql
"""
HTML content that has been validated and is safe for DOM insertion.
Automatically sanitized on server, satisfies Trusted Types on client.
"""
scalar TrustedHTML

"""
Script content that has been validated and is safe for execution.
Only returned for known-safe, audited scripts.
"""
scalar TrustedScript

"""
URL that has been validated and is safe for script loading.
Only allows URLs from approved domains.
"""
scalar TrustedScriptURL
```

#### Server-Side Generation

```typescript
// Server creates TrustedHTML via sanitization
const resolvers = {
  Article: {
    content: async (article, _, ctx) => {
      const rawHtml = await ctx.db.getContent(article.id);

      // Sanitize and wrap as TrustedHTML
      return {
        __typename: "TrustedHTML",
        value: DOMPurify.sanitize(rawHtml, {
          ALLOWED_TAGS: ctx.config.html.allowedTags,
          ALLOWED_ATTR: ctx.config.html.allowedAttributes,
        }),
      };
    },
  },
};
```

#### Client-Side Usage (Vue)

```typescript
// generated/trusted-types.ts
export interface TrustedHTML {
  readonly __brand: "TrustedHTML";
  readonly value: string;
}

// The value is pre-sanitized and safe for v-html
```

```vue
<script setup lang="ts">
const { data: article } = useArticle({ id });
</script>

<template>
  <!-- Safe: content is TrustedHTML from server -->
  <div v-html="article?.content.value" />
</template>
```

#### CSP Integration

When using Trusted Types with CSP:

```graphql
schema @csp(
  requireTrustedTypesFor: [Script],
  trustedTypes: {
    policies: ["better-graphql"],
    defaultPolicy: "better-graphql"
  }
) {
  query: Query
}
```

See [HTTP Protocol - Trusted Types](./05-http-protocol.md#14-trusted-types) for complete CSP and Trusted Types integration.

### 2.6 Built-in File Type

The `File` type represents file uploads and downloads with metadata.

```graphql
type File {
  """File name"""
  name: String

  """MIME type"""
  mimeType: String

  """File size in bytes"""
  size: Uint

  """URL to access the file"""
  url: String

  """File content (for small files or when explicitly requested)"""
  content: Option<Stream>
}

input FileUpload {
  """File name"""
  name: String

  """MIME type"""
  mimeType: String

  """File content"""
  content: Stream
}
```

#### File Upload via FormData

Better GraphQL supports multipart/form-data for file uploads:

```graphql
type Mutation {
  uploadAvatar(file: FileUpload): File
  uploadFiles(files: List<FileUpload>): List<File>
  createPostWithAttachments(
    title: String,
    content: HTML,
    attachments: List<FileUpload>
  ): Post
}
```

Client sends multipart request:
```
POST /graphql HTTP/1.1
Content-Type: multipart/form-data; boundary=----FormBoundary

------FormBoundary
Content-Disposition: form-data; name="operations"

{"query":"mutation($file: FileUpload) { uploadAvatar(file: $file) { url } }","variables":{"file":null}}
------FormBoundary
Content-Disposition: form-data; name="map"

{"0":["variables.file"]}
------FormBoundary
Content-Disposition: form-data; name="0"; filename="avatar.png"
Content-Type: image/png

<binary data>
------FormBoundary--
```

### 2.7 Built-in Image Type

The `Image` type represents images with variant support (different sizes, formats).

```graphql
type Image {
  """Image format"""
  format: ImageFormat

  """Original width in pixels"""
  width: Uint

  """Original height in pixels"""
  height: Uint

  """File size in bytes"""
  size: Uint

  """Alt text for accessibility"""
  alt: Option<String>

  """Direct URL"""
  url: String

  """Signed URL with expiration (for private images)"""
  signedUrl(expiresIn: Uint = 3600): SignedUrl

  """Available size variants"""
  variants: List<ImageVariant>

  """Get URL for specific dimensions (on-the-fly resize)"""
  resized(width: Option<Uint>, height: Option<Uint>, fit: ImageFit = Cover): String

  """Blur hash for placeholder"""
  blurHash: Option<String>

  """Dominant color (hex)"""
  dominantColor: Option<String>
}

type ImageVariant {
  """Variant name (e.g., "thumbnail", "medium", "large")"""
  name: String

  """Width in pixels"""
  width: Uint

  """Height in pixels"""
  height: Uint

  """Format of this variant"""
  format: ImageFormat

  """URL for this variant"""
  url: String

  """Signed URL for this variant"""
  signedUrl(expiresIn: Uint = 3600): SignedUrl
}

type SignedUrl {
  """The signed URL"""
  url: String

  """Expiration timestamp"""
  expiresAt: DateTime

  """Time until expiration in seconds"""
  expiresIn: Uint
}

enum ImageFormat {
  Jpeg
  Png
  Gif
  WebP
  Avif
  Svg
}

enum ImageFit {
  Cover
  Contain
  Fill
  Inside
  Outside
}

input ImageUpload {
  """Image format"""
  format: ImageFormat

  """Alt text"""
  alt: Option<String>

  """Image content"""
  content: Stream

  """Generate variants"""
  generateVariants: Option<List<ImageVariantConfig>>
}

input ImageVariantConfig {
  """Variant name"""
  name: String

  """Target width"""
  width: Option<Uint>

  """Target height"""
  height: Option<Uint>

  """Output format"""
  format: Option<ImageFormat>

  """Fit mode"""
  fit: ImageFit = Cover
}
```

#### Image with Signed URLs

```graphql
type User {
  id: ID
  name: String

  """Private avatar image with signed URL"""
  avatar: Option<Image>
}

# Query
query GetUser($id: UserId) {
  user(id: $id) {
    ... on User {
      name
      avatar {
        # Get signed URL valid for 1 hour
        signedUrl(expiresIn: 3600) {
          url
          expiresAt
        }
        # Get thumbnail variant
        variants {
          name
          url
        }
        # On-the-fly resize
        resized(width: 200, height: 200, fit: Cover)
        # Placeholder
        blurHash
      }
    }
  }
}
```

#### Response with Signed URL

```json
{
  "data": {
    "user": {
      "__typename": "User",
      "name": "John",
      "avatar": {
        "signedUrl": {
          "url": "https://cdn.example.com/images/avatar.jpg?token=abc123&expires=1706486400",
          "expiresAt": "2024-01-29T00:00:00Z"
        },
        "variants": [
          { "name": "thumbnail", "url": "https://cdn.example.com/images/avatar_thumb.jpg" },
          { "name": "medium", "url": "https://cdn.example.com/images/avatar_medium.jpg" }
        ],
        "resized": "https://cdn.example.com/images/avatar.jpg?w=200&h=200&fit=cover",
        "blurHash": "LEHV6nWB2yk8pyo0adR*.7kCMdnj"
      }
    }
  }
}
```

### 2.8 Built-in Audio Type

The `Audio` type represents audio content with streaming support.

```graphql
type Audio {
  """Audio format"""
  format: AudioFormat

  """Duration in seconds"""
  duration: Float

  """Sample rate in Hz"""
  sampleRate: Uint

  """Number of channels"""
  channels: Uint

  """Bitrate in kbps"""
  bitrate: Option<Uint>

  """URL for streaming/download"""
  url: String

  """HLS playlist URL (for adaptive streaming)"""
  hlsUrl: Option<String>
}

enum AudioFormat {
  Mp3
  Wav
  Ogg
  Aac
  Flac
  WebM
}

input AudioUpload {
  """Audio format"""
  format: AudioFormat

  """Audio content"""
  content: Stream
}
```

#### Audio Streaming

```graphql
type Query {
  """Get audio with adaptive streaming"""
  podcast(id: ID): PodcastResult
}

type Podcast {
  id: ID
  title: String
  audio: Audio

  """Transcription (streamed as it's generated)"""
  transcription: Deferred<String>
}

type Subscription {
  """Real-time audio stream (e.g., live broadcast)"""
  liveAudio(channelId: ID): Audio @stream
}
```

### 2.9 Built-in Video Type

The `Video` type represents video content with HLS and adaptive streaming support.

```graphql
type Video {
  """Video format"""
  format: VideoFormat

  """Duration in seconds"""
  duration: Float

  """Width in pixels"""
  width: Uint

  """Height in pixels"""
  height: Uint

  """Frame rate"""
  frameRate: Float

  """Video bitrate in kbps"""
  bitrate: Option<Uint>

  """Direct URL for download/streaming"""
  url: String

  """HLS master playlist URL"""
  hlsUrl: Option<String>

  """DASH manifest URL"""
  dashUrl: Option<String>

  """Available quality variants"""
  variants: List<VideoVariant>

  """Thumbnail images"""
  thumbnails: List<Thumbnail>
}

type VideoVariant {
  """Quality label (e.g., "1080p", "720p")"""
  quality: String

  """Width in pixels"""
  width: Uint

  """Height in pixels"""
  height: Uint

  """Bitrate in kbps"""
  bitrate: Uint

  """URL for this variant"""
  url: String
}

type Thumbnail {
  """Timestamp in seconds"""
  timestamp: Float

  """Thumbnail URL"""
  url: String

  """Width in pixels"""
  width: Uint

  """Height in pixels"""
  height: Uint
}

enum VideoFormat {
  Mp4
  WebM
  Hls
  Dash
  Mov
  Avi
}

input VideoUpload {
  """Video format"""
  format: VideoFormat

  """Video content"""
  content: Stream

  """Generate HLS variants"""
  generateHls: Boolean = false

  """Target qualities for transcoding"""
  targetQualities: Option<List<String>>
}
```

#### HLS Streaming

```graphql
type Query {
  """Get video with HLS adaptive streaming"""
  video(id: ID): VideoResult
}

type Movie {
  id: ID
  title: String
  video: Video

  """Subtitles in multiple languages"""
  subtitles: List<Subtitle>
}

type Subtitle {
  language: String
  url: String
  format: SubtitleFormat
}

enum SubtitleFormat {
  Vtt
  Srt
  Ass
}
```

### 2.10 Built-in Stream Type

The `Stream` type represents binary data streams for large content transfer.

```graphql
scalar Stream

type Query {
  """Download large file as stream"""
  downloadFile(id: ID): Stream
}

type Mutation {
  """Upload file as stream"""
  uploadLargeFile(
    name: String,
    mimeType: String,
    content: Stream
  ): File
}
```

#### Streaming Protocol

Streams use chunked transfer encoding:

```
POST /graphql HTTP/1.1
Content-Type: application/json
Transfer-Encoding: chunked

{"query":"mutation($content: Stream) { uploadLargeFile(name: \"large.zip\", mimeType: \"application/zip\", content: $content) { url } }"}
```

Response streams:
```
HTTP/1.1 200 OK
Content-Type: application/octet-stream
Transfer-Encoding: chunked

<chunked binary data>
```

#### AbortController Integration

All stream operations support cancellation:

```typescript
const controller = new AbortController();

// Upload with progress and cancellation
const result = await client.uploadLargeFile(
  { name: "large.zip", mimeType: "application/zip", content: fileStream },
  {
    signal: controller.signal,
    onProgress: (loaded, total) => {
      console.log(`${(loaded / total * 100).toFixed(1)}%`);
    }
  }
);

// Cancel if needed
controller.abort();
```

### 2.11 Built-in Void Type

The `Void` type represents the absence of a return value. It is used for mutations that perform actions but don't return meaningful data.

```graphql
type Mutation {
  # No return value needed
  logEvent(event: EventInput): Void

  # Fire-and-forget operations
  sendAnalytics(data: AnalyticsInput): Void

  # Side-effect only operations
  invalidateCache(keys: List<String>): Void
}
```

**Serialization**: `null` (always returns null)

#### Use Cases

- **Fire-and-forget operations**: Analytics, logging, cache invalidation
- **Side-effect only mutations**: When the operation's success is implicit
- **Webhook triggers**: Trigger external systems without return data

#### Void vs Nullable

```graphql
type Mutation {
  # Void: intentionally returns nothing
  logEvent(event: EventInput): Void

  # Nullable: might return a User or null
  findUser(email: String): Option<User>
}
```

### 2.12 Tuple Types

Better GraphQL supports tuple types - fixed-length arrays with specific types at each position.

```graphql
type GeoLocation {
  # Tuple of [latitude, longitude]
  coordinates: (Float, Float)

  # Tuple with different types
  bounds: Option<(Float, Float, Float, Float)>  # Nullable tuple: [minLat, minLng, maxLat, maxLng]
}

type Color {
  # RGB tuple
  rgb: (Uint, Uint, Uint)

  # RGBA tuple
  rgba: (Uint, Uint, Uint, Float)
}

type DateRange {
  # Tuple of dates
  range: (Date, Date)
}
```

#### Tuple Syntax

```graphql
# Basic tuple
(Type1, Type2)

# Tuple with more elements
(Type1, Type2, Type3, Type4)

# Nullable tuple
Option<(Type1, Type2)>

# Tuple with nullable elements
(Option<Type1>, Type2, Option<Type3>)
```

#### Tuple Serialization

Tuples are serialized as JSON arrays:

```json
{
  "coordinates": [35.6762, 139.6503],
  "rgb": [255, 128, 0],
  "range": ["2024-01-01", "2024-12-31"]
}
```

#### Named Tuples (Optional)

For improved readability, tuples can have named positions:

```graphql
type GeoLocation {
  # Named tuple
  coordinates: (lat: Float, lng: Float)
}

type Pagination {
  # Named tuple for pagination info
  pageInfo: (page: Uint, perPage: Uint, total: Uint)
}
```

Named tuples serialize identically to regular tuples (as arrays), but provide better documentation and IDE support.

#### Tuple vs Object Type

Use tuples for simple, ordered data. Use object types for complex structures:

```graphql
# Good use of tuple: simple coordinate pair
type Location {
  coordinates: (Float, Float)
}

# Prefer object type for complex structures
type Location {
  address: Address  # Not (String, String, String, String)
}

type Address {
  street: String
  city: String
  state: String
  country: String
}
```

#### Tuples in Input Types

```graphql
input CreateLocationInput {
  name: String
  coordinates: (Float, Float)
}

input ColorInput {
  primary: (Uint, Uint, Uint)
  secondary: Option<(Uint, Uint, Uint)>
}
```

### 2.13 Custom Scalar Types

```graphql
scalar Email @pattern(regex: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$")
scalar URL
scalar UUID
scalar BigInt    # For integers larger than 32-bit
scalar Decimal   # For precise decimal numbers
```

### 2.14 Newtype (Nominal Types)

Better GraphQL supports newtypes for creating distinct types from existing types. This provides nominal typing (type safety based on name, not structure).

```graphql
# Define distinct ID types
newtype UserId = ID
newtype PostId = ID
newtype CommentId = ID

# These are incompatible even though they're all based on ID
type User {
  id: UserId
  posts: List<Post>
}

type Post {
  id: PostId
  authorId: UserId    # Cannot accidentally use PostId here
  comments: List<Comment>
}

type Comment {
  id: CommentId
  postId: PostId      # Cannot accidentally use UserId here
  authorId: UserId
}
```

#### Type Safety with Newtypes

Newtypes prevent accidental mixing of semantically different values:

```graphql
newtype UserId = ID
newtype PostId = ID

type Query {
  user(id: UserId): Option<User>
  post(id: PostId): Option<Post>
}

# Client code:
# user(id: userId)  # OK
# user(id: postId)  # Compile error! PostId is not UserId
```

#### Newtypes with Validation

Newtypes can include validation directives:

```graphql
# Email newtype with built-in validation
newtype EmailAddress = String @email

# Positive integer
newtype PositiveInt = Int @positive

# Percentage (0-100)
newtype Percentage = Float @range(min: 0, max: 100)

# URL newtype
newtype WebURL = String @url

# UUID newtype
newtype UUID = String @uuid

type User {
  id: UserId
  email: EmailAddress      # Validated as email
  age: Option<PositiveInt> # Must be positive if provided
  website: Option<WebURL>  # Validated as URL
}
```

#### Newtypes vs Type Aliases

```graphql
# Newtype: Creates a distinct type (nominal typing)
newtype UserId = ID
newtype PostId = ID
# UserId and PostId are INCOMPATIBLE even though both wrap ID

# Type alias: Just a shorthand (structural typing)
type alias Connection = Connection<User>
# Connection is interchangeable with Connection<User>
```

#### Newtype Serialization

Newtypes serialize as their underlying type:

```json
{
  "user": {
    "id": "user_123",      // UserId serializes as string
    "email": "test@example.com",  // EmailAddress serializes as string
    "posts": [
      { "id": "post_456" }  // PostId serializes as string
    ]
  }
}
```

#### Generic Newtypes

Newtypes can wrap generic types:

```graphql
# Non-empty list
newtype NonEmptyList<T> = List<T> @minItems(1)

# Bounded list
newtype BoundedList<T, Max extends Uint> = List<T> @maxItems(Max)

type User {
  tags: NonEmptyList<String>  # Must have at least one tag
  roles: BoundedList<Role, 5>  # Max 5 roles
}
```

#### Use Cases

1. **ID Safety**: Prevent mixing different entity IDs
2. **Domain Modeling**: Express domain concepts precisely
3. **Validation**: Attach validation rules to types
4. **Documentation**: Self-documenting type names
5. **Refactoring Safety**: Compiler catches type mismatches

### 2.15 Opaque Types

Opaque types are similar to newtypes but with an important distinction: the underlying type is completely hidden from clients. This provides true encapsulation.

```graphql
# Define opaque types
opaque Email = String
opaque UserId = ID
opaque Money = Int

type User {
  id: UserId
  email: Email
  balance: Money
}
```

#### Opaque vs Newtype

| Feature | Newtype | Opaque |
|---------|---------|--------|
| Underlying type visible | Yes | No |
| Client can construct | Yes (with underlying value) | No (must use factory) |
| Introspection shows underlying | Yes | No |
| Validation location | Client or Server | Server only |

```graphql
# Newtype: underlying type is visible
newtype UserId = ID
# Client knows UserId is based on ID and can construct it

# Opaque: underlying type is hidden
opaque Email = String
# Client only sees "Email" type, cannot construct directly
```

#### Opaque Type Benefits

1. **True encapsulation**: Implementation details hidden from clients
2. **Server-side validation**: Only server can create valid instances
3. **API stability**: Underlying type can change without breaking clients
4. **Security**: Sensitive data patterns are hidden

```graphql
# Client sees only the opaque type
opaque SecureToken = String

type AuthResult {
  token: SecureToken  # Client can't construct, only receive
}

type Mutation {
  # Server creates and returns SecureToken
  login(email: Email, password: String): AuthResult

  # Server receives and validates SecureToken
  validateSession(token: SecureToken): SessionResult
}
```

#### Opaque Type Serialization

Opaque types serialize as their underlying type, but clients don't know the underlying type:

```json
{
  "user": {
    "id": "user_123",     // UserId serializes as string (but client sees it as UserId)
    "email": "test@example.com"  // Email serializes as string (but client sees it as Email)
  }
}
```

#### Opaque Type with Validation

```graphql
# Server validates that the underlying String is a valid email
opaque Email = String @email

# Server validates the format
opaque PhoneNumber = String @pattern(regex: "^\\+[1-9]\\d{1,14}$")

# Server validates the range
opaque Percentage = Float @range(min: 0, max: 100)
```

## 3. Nullable and Non-nullable

### 3.1 Non-nullable by Default

In Better GraphQL, types are non-nullable by default. Use `Option<T>` for nullable types.

```graphql
type User {
  id: ID               # Required (cannot be null)
  name: String         # Required (cannot be null)
  email: Option<String>   # Optional (can be null)
  bio: Option<String>     # Optional (can be null)
}
```

### 3.2 Option Type

`Option<T>` wraps a type to make it nullable:

```graphql
type User {
  name: String              # Non-null, must have a value
  nickname: Option<String>  # Nullable, can be null
  age: Option<Int>          # Nullable integer
  profile: Option<Profile>  # Nullable object
}
```

### 3.3 List Type

`List<T>` represents an ordered collection of elements:

```graphql
type User {
  tags: List<String>           # List of strings
  posts: List<Post>            # List of posts
  scores: List<Int>            # List of integers
}
```

### 3.4 Combining List and Option

```graphql
type User {
  # Non-null list, non-null elements (most common)
  tags: List<String>

  # Non-null list, nullable elements
  middleNames: List<Option<String>>

  # Nullable list, non-null elements
  nicknames: Option<List<String>>

  # Nullable list, nullable elements
  aliases: Option<List<Option<String>>>
}
```

### 3.5 Comparison with GraphQL

| GraphQL | Better GraphQL | Meaning |
|---------|----------------|---------|
| `String` | `Option<String>` | Nullable |
| `String!` | `String` | Non-nullable |
| `[String]` | `Option<List<Option<String>>>` | Nullable list, nullable elements |
| `[String!]!` | `List<String>` | Non-nullable list, non-nullable elements |
| `[String]!` | `List<Option<String>>` | Non-nullable list, nullable elements |
| `[String!]` | `Option<List<String>>` | Nullable list, non-nullable elements |

## 4. Object Types

### 4.1 Definition

```graphql
type User {
  id: ID
  name: String
  email: String
  createdAt: DateTime
  updatedAt: Option<DateTime>

  # Relations
  posts: List<Post>
  profile: Option<Profile>
}
```

### 4.2 Field Arguments

```graphql
type User {
  posts(
    first: Int = 10,
    after: Option<String>,
    orderBy: PostOrderBy = CreatedAtDesc
  ): PostConnection
}
```

## 5. Interface Types

### 5.1 Definition

```graphql
interface Node {
  id: ID
}

interface Timestamped {
  createdAt: DateTime
  updatedAt: Option<DateTime>
}

type User implements Node & Timestamped {
  id: ID
  createdAt: DateTime
  updatedAt: Option<DateTime>
  name: String
}
```

### 5.2 Interface Inheritance

```graphql
interface Entity implements Node {
  id: ID
  name: String
}

type User implements Entity {
  id: ID
  name: String
  email: String
}
```

### 5.3 Marker Interfaces (Empty Interfaces)

Better GraphQL supports empty interfaces as "marker" or "tag" interfaces. These are useful for categorizing types without requiring specific fields.

```graphql
# Marker interfaces (no fields)
interface Serializable {}
interface Cacheable {}
interface Auditable {}
interface Deletable {}

# Types implementing marker interfaces
type User implements Node & Serializable & Cacheable {
  id: ID
  name: String
  email: String
}

type SystemConfig implements Serializable {
  key: String
  value: JSON
}

type AuditLog implements Auditable {
  id: ID
  action: String
  timestamp: DateTime
}
```

#### Use Cases for Marker Interfaces

1. **Type categorization**: Group types by capability or behavior

```graphql
interface Exportable {}
interface Importable {}

type User implements Exportable & Importable {
  id: ID
  name: String
}

type SystemSetting implements Exportable {
  # Can be exported but not imported
  key: String
  value: String
}
```

2. **Generic constraints**: Use with generics to constrain type parameters

```graphql
interface Persistable {}

# Only persistable types can be stored
type Repository<T extends Persistable> {
  save(entity: T): T
  delete(id: ID): Void
}

type User implements Node & Persistable {
  id: ID
  name: String
}

type Query {
  userRepo: Repository<User>  # OK: User implements Persistable
}
```

3. **Runtime behavior hints**: Signal to the server how to handle types

```graphql
interface SoftDeletable {}
interface HardDeletable {}

type User implements SoftDeletable {
  id: ID
  deletedAt: Option<DateTime>  # Soft delete: set timestamp
}

type TempFile implements HardDeletable {
  id: ID
  path: String  # Hard delete: actually remove
}
```

4. **Access control markers**

```graphql
interface PubliclyAccessible {}
interface AdminOnly {}
interface OwnerOnly {}

type PublicPost implements PubliclyAccessible {
  id: ID
  title: String
  content: String
}

type AdminDashboard implements AdminOnly {
  stats: Stats
  users: List<User>
}

type PrivateMessage implements OwnerOnly {
  id: ID
  content: String
  recipient: User
}
```

## 6. Union Types

### 6.1 Definition

```graphql
union SearchResult = User | Post | Comment
```

### 6.2 Unions as Typed Errors

Better GraphQL recommends expressing errors as union types.

```graphql
type User {
  id: ID
  name: String
}

type NotFoundError {
  message: String
  resourceId: ID
}

type UnauthorizedError {
  message: String
  requiredPermission: String
}

union UserResult = User | NotFoundError | UnauthorizedError

type Query {
  user(id: ID): UserResult
}
```

## 7. Enum Types

### 7.1 Definition

Better GraphQL uses **PascalCase** for enum values (unlike GraphQL's SCREAMING_SNAKE_CASE).

```graphql
enum UserRole {
  Admin
  Moderator
  User
  Guest
}

enum PostStatus {
  Draft
  Published
  Archived @deprecated(reason: "Use Hidden instead")
  Hidden
}

enum HttpMethod {
  Get
  Post
  Put
  Patch
  Delete
  Options
  Head
}
```

### 7.2 Enum Values and Documentation

```graphql
enum OrderDirection {
  """Sort in ascending order"""
  Asc

  """Sort in descending order"""
  Desc
}

enum CacheScope {
  """Cacheable by CDN and shared caches"""
  Public

  """User-specific, not cacheable by shared caches"""
  Private
}
```

### 7.3 Naming Convention Rationale

PascalCase for enum values provides:

1. **Consistency with types**: Enum values follow the same casing as type names
2. **Better readability**: `UserRole.Admin` is more readable than `UserRole.ADMIN`
3. **Language alignment**: Matches conventions in modern languages (TypeScript, Rust, Swift)

```graphql
# Better GraphQL (PascalCase)
enum Status {
  Active
  Inactive
  Pending
}

# GraphQL (SCREAMING_SNAKE_CASE) - not used in Better GraphQL
# enum Status {
#   ACTIVE
#   INACTIVE
#   PENDING
# }
```

## 8. Input Types

### 8.1 Basic Input Types

```graphql
input CreateUserInput {
  name: String
  email: String
  role: UserRole = User
}
```

### 8.2 Input Enum (Better GraphQL Exclusive Feature)

Better GraphQL supports Input Enum, allowing enum variants to carry input data. This is more expressive than input unions and follows Rust-style enum semantics.

```graphql
# Input enum with variant data
input enum LoginMethod {
  Email { email: String, password: String }
  OAuth { provider: OAuthProvider, token: String }
  Phone { phoneNumber: String, verificationCode: String }
  Passkey { credentialId: String, authenticatorData: String }
}

type Mutation {
  login(method: LoginMethod): AuthResult
}
```

#### Input Enum vs Input Union

Input enum is preferred over input union for:
- **Discriminated unions**: When you need to distinguish variants by a tag
- **Variant-specific data**: When each variant has different fields
- **Type safety**: Compile-time exhaustiveness checking

```graphql
# Input enum (preferred)
input enum PaymentMethod {
  Card { cardNumber: String, cvv: String, expiry: String }
  BankTransfer { accountNumber: String, routingNumber: String }
  Crypto { walletAddress: String, network: CryptoNetwork }
}

# vs Input union (legacy approach)
input CardPayment { cardNumber: String, cvv: String, expiry: String }
input BankPayment { accountNumber: String, routingNumber: String }
input CryptoPayment { walletAddress: String, network: CryptoNetwork }
input union PaymentMethodLegacy = CardPayment | BankPayment | CryptoPayment
```

#### Input Enum Serialization

```json
{
  "method": {
    "__variant": "Email",
    "email": "user@example.com",
    "password": "secret"
  }
}
```

Alternative serialization with external tagging:

```json
{
  "method": {
    "Email": {
      "email": "user@example.com",
      "password": "secret"
    }
  }
}
```

#### Unit Variants

Input enums can have unit variants (no data):

```graphql
input enum SortOrder {
  Ascending
  Descending
  Random { seed: Option<Int> }
}
```

```json
{ "order": { "__variant": "Ascending" } }
{ "order": { "__variant": "Random", "seed": 42 } }
```

### 8.3 Input Union (Legacy)

Input unions are still supported for backwards compatibility, but input enums are preferred.

```graphql
input EmailCredentials {
  email: String
  password: String
}

input OAuthCredentials {
  provider: OAuthProvider
  token: String
}

input PhoneCredentials {
  phoneNumber: String
  verificationCode: String
}

input union LoginCredentials = EmailCredentials | OAuthCredentials | PhoneCredentials

type Mutation {
  login(credentials: LoginCredentials): AuthResult
}
```

#### Input Union Serialization

```json
{
  "credentials": {
    "__typename": "EmailCredentials",
    "email": "user@example.com",
    "password": "secret"
  }
}
```

### 8.4 Patch Modifier

Generates an input type for partial updates.

```graphql
input UpdateUserInput @patch(type: User) {
  # All fields automatically become optional (wrapped in Option)
  # name: Option<String>
  # email: Option<String>
  # bio: Option<String>
}

# Explicit definition
input UpdateUserInput {
  name: Option<String>
  email: Option<String>
  bio: Option<String>
}
```

### 8.4 Put Modifier

Defines an input type for complete replacement.

```graphql
input ReplaceUserInput @put(type: User) {
  # Same fields as target type are required
  name: String
  email: String
  bio: Option<String>  # Stays optional if original was optional
}
```

## 9. Type Modifier Summary

| Type | Meaning | Example |
|------|---------|---------|
| `T` | Non-nullable | `String` |
| `Option<T>` | Nullable (can be null) | `Option<String>` |
| `List<T>` | Ordered collection | `List<String>` |
| `(T1, T2)` | Tuple | `(Float, Float)` |
| `Type<T>` | Generic type instantiation | `Connection<User>` |
| `<T extends X>` | Constrained type parameter | `<T extends Node>` |
| `newtype T = U` | Nominal type (distinct from underlying) | `newtype UserId = ID` |
| `type alias T = U` | Type alias (interchangeable) | `type alias UserConn = Connection<User>` |
| `@patch` | Partial update input | `@patch(type: User)` |
| `@put` | Complete replacement input | `@put(type: User)` |

## 10. Generic Types

Better GraphQL supports generic types (parametric polymorphism) for creating reusable type definitions.

### 10.1 Basic Generic Types

```graphql
# Generic Connection type for pagination
type Connection<T> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

type Edge<T> {
  cursor: String
  node: T
}

type PageInfo {
  hasNextPage: Boolean
  hasPreviousPage: Boolean
  startCursor: Option<String>
  endCursor: Option<String>
}

# Usage
type Query {
  users(first: Uint, after: Option<String>): Connection<User>
  posts(first: Uint, after: Option<String>): Connection<Post>
  comments(first: Uint, after: Option<String>): Connection<Comment>
}
```

### 10.2 Generic Type Constraints (extends)

Use `extends` to constrain generic type parameters:

```graphql
# T must implement Node interface
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

# T must implement both Node and Timestamped
type AuditableConnection<T extends Node & Timestamped> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  lastModified: DateTime
}

# Only types implementing Node can be used
type Query {
  users: Connection<User>      # OK: User implements Node
  # posts: Connection<String>  # Error: String doesn't implement Node
}
```

### 10.3 Multiple Type Parameters

```graphql
# Result type with success and error types
type Result<T, E extends Error> {
  data: Option<T>
  error: Option<E>
  success: Boolean
}

# Key-value pair
type Pair<K, V> {
  key: K
  value: V
}

# Map entries
type MapEntry<K extends Scalar, V> {
  key: K
  value: V
}

# Usage
type Query {
  getUser(id: ID): Result<User, NotFoundError>
  getConfig: List<Pair<String, JSON>>
}
```

### 10.4 Generic Interfaces

```graphql
interface Repository<T extends Node> {
  findById(id: ID): Option<T>
  findAll(first: Uint, after: Option<String>): Connection<T>
  count: Uint
}

type UserRepository implements Repository<User> {
  findById(id: ID): Option<User>
  findAll(first: Uint, after: Option<String>): Connection<User>
  count: Uint
  # Additional methods
  findByEmail(email: String): Option<User>
}
```

### 10.5 Generic Input Types

```graphql
input CreateInput<T> {
  data: T
  metadata: Option<JSON>
}

input UpdateInput<T> {
  id: ID
  data: T
}

input BatchInput<T> {
  items: List<T>
  options: Option<BatchOptions>
}

# Usage
type Mutation {
  createUser(input: CreateInput<UserData>): User
  updateUser(input: UpdateInput<UserData>): User
  batchCreatePosts(input: BatchInput<PostData>): List<Post>
}
```

### 10.6 Default Type Parameters

```graphql
# Default error type
type Result<T, E extends Error = GenericError> {
  data: Option<T>
  error: Option<E>
}

# Default pagination size
type Page<T, Size extends Uint = 20> {
  items: List<T>
  pageSize: Size
  currentPage: Uint
}

# Usage
type Query {
  getUser(id: ID): Result<User>  # Uses GenericError as default
  getUsers: Page<User>           # Uses 20 as default size
}
```

### 10.7 Built-in Generic Types

Better GraphQL provides these built-in generic types:

```graphql
# Option is a built-in generic type (no need to define)
# Option<T> wraps a value that might be null

# Result type for error handling
type Result<T, E extends Error = GenericError> {
  value: Option<T>
  error: Option<E>
  isOk: Boolean
  isErr: Boolean
}

# Connection for Relay-style pagination
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

type Edge<T> {
  cursor: String
  node: T
}
```

### 10.8 Type Parameter Bounds

```graphql
# Single constraint
<T extends Node>

# Multiple constraints (intersection)
<T extends Node & Timestamped>

# Constraint with specific interface
<T extends Serializable>

# No constraint (any type)
<T>
```

### 10.9 Generic Type Aliases

```graphql
# Type alias with generics
type alias UserConnection = Connection<User>
type alias PostConnection = Connection<Post>

type alias UserResult = Result<User, UserError>

# Usage in schema
type Query {
  users: UserConnection
  posts: PostConnection
  user(id: ID): UserResult
}
```

### 10.10 Variance

Better GraphQL supports type variance annotations:

```graphql
# Covariant (output position) - default for return types
type Producer<out T> {
  produce: T
}

# Contravariant (input position) - default for argument types
type Consumer<in T> {
  consume(value: T): Void
}

# Invariant (both positions)
type Processor<T> {
  process(input: T): T
}
```

### 10.11 Utility Types

Better GraphQL provides built-in utility types for common type transformations, similar to TypeScript's utility types.

#### Partial<T>

Makes all fields of T optional (wrapped in Option).

```graphql
type User {
  id: ID
  name: String
  email: String
  bio: Option<String>
}

# Partial<User> is equivalent to:
# {
#   id: Option<ID>
#   name: Option<String>
#   email: Option<String>
#   bio: Option<String>  # Already optional, stays optional
# }

input UpdateUserInput = Partial<User>

type Mutation {
  updateUser(id: ID, data: Partial<User>): User
}
```

#### Required<T>

Makes all fields of T required (removes Option wrapper).

```graphql
type UserDraft {
  id: Option<ID>
  name: Option<String>
  email: Option<String>
}

# Required<UserDraft> is equivalent to:
# {
#   id: ID
#   name: String
#   email: String
# }

input CreateUserInput = Required<UserDraft>
```

#### Pick<T, Keys>

Creates a type with only the specified fields from T.

```graphql
type User {
  id: ID
  name: String
  email: String
  password: String
  createdAt: DateTime
}

# Pick<User, "id" | "name" | "email"> is equivalent to:
# {
#   id: ID
#   name: String
#   email: String
# }

type PublicUser = Pick<User, "id" | "name" | "email">

# Usage in input
input UserIdentifier = Pick<User, "id">
input UserCredentials = Pick<User, "email" | "password">
```

#### Omit<T, Keys>

Creates a type with all fields from T except the specified ones.

```graphql
type User {
  id: ID
  name: String
  email: String
  password: String
  createdAt: DateTime
}

# Omit<User, "password" | "createdAt"> is equivalent to:
# {
#   id: ID
#   name: String
#   email: String
# }

type SafeUser = Omit<User, "password">
input CreateUserInput = Omit<User, "id" | "createdAt">
```

#### Combining Utility Types

Utility types can be combined for powerful type transformations:

```graphql
type User {
  id: ID
  name: String
  email: String
  password: String
  bio: Option<String>
  createdAt: DateTime
}

# Partial update without system fields
input UpdateUserInput = Partial<Omit<User, "id" | "createdAt">>
# Equivalent to:
# {
#   name: Option<String>
#   email: Option<String>
#   password: Option<String>
#   bio: Option<String>
# }

# Create input without auto-generated fields
input CreateUserInput = Omit<User, "id" | "createdAt">
# Equivalent to:
# {
#   name: String
#   email: String
#   password: String
#   bio: Option<String>
# }

# Partial pick for specific field updates
input UpdateNameInput = Partial<Pick<User, "name" | "bio">>
# Equivalent to:
# {
#   name: Option<String>
#   bio: Option<String>
# }
```

#### Readonly<T>

Marks all fields as read-only (for documentation/tooling purposes).

```graphql
type UserSnapshot = Readonly<User>
```

#### Record<K, V>

Creates a map/dictionary type with keys of type K and values of type V.

```graphql
# Map of string keys to user values
type UserMap = Record<String, User>

# Settings as key-value pairs
type Settings = Record<String, JSON>
```

## 11. Type Equivalence

Two types are equivalent when:

1. They are the same scalar type
2. They are the same enum type
3. They are the same Object/Interface/Union type
4. Both are list types with equivalent element types
5. Both have the same nullable modifier
6. Both are generic types with equivalent type parameters
7. Both resolve to the same type after applying utility type transformations

## 12. Type Compatibility

Type A is a subtype of Type B when:

1. A and B are equivalent
2. A implements Interface B
3. A is a member of Union B
4. A is Non-nullable and B is Nullable (same base type)
5. A is a generic type instantiation that satisfies B's constraints

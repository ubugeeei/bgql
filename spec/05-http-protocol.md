# Better GraphQL Specification - HTTP Protocol

## 1. Overview

Better GraphQL treats HTTP as a first-class citizen, providing native support for headers, cookies, CORS, and streaming responses. This is a major departure from GraphQL, which largely ignores HTTP semantics.

## 2. Request Format

### 2.1 HTTP Methods

| Method | Use Case |
|--------|----------|
| `POST` | Primary method for all operations |
| `GET` | For persisted queries and simple reads |

### 2.2 Content Types

**Request**:
- `application/json` - Standard JSON request
- `multipart/form-data` - For file uploads

**Response**:
- `application/json` - Standard JSON response
- `multipart/mixed` - Streaming response
- `text/event-stream` - Server-Sent Events

### 2.3 Request Body (POST)

```json
{
  "query": "query GetUser($id: ID) { user(id: $id) { id name } }",
  "operationName": "GetUser",
  "variables": {
    "id": "123"
  },
  "extensions": {
    "persistedQuery": {
      "sha256Hash": "abc123..."
    }
  }
}
```

### 2.4 GET Requests

For simple queries without variables or with URL-encoded variables:

```
GET /graphql?query={user(id:"123"){id,name}}
GET /graphql?query=query+GetUser($id:ID){user(id:$id){id,name}}&variables={"id":"123"}
```

### 2.5 FormData / Multipart Request

For file uploads, use `multipart/form-data` with the GraphQL multipart request specification:

```
POST /graphql HTTP/1.1
Content-Type: multipart/form-data; boundary=----BetterGraphQLBoundary

------BetterGraphQLBoundary
Content-Disposition: form-data; name="operations"

{"query":"mutation($file: FileUpload) { uploadAvatar(file: $file) { url name size } }","variables":{"file":null}}
------BetterGraphQLBoundary
Content-Disposition: form-data; name="map"

{"0":["variables.file"]}
------BetterGraphQLBoundary
Content-Disposition: form-data; name="0"; filename="avatar.png"
Content-Type: image/png

<binary file content>
------BetterGraphQLBoundary--
```

#### Multiple File Upload

```
POST /graphql HTTP/1.1
Content-Type: multipart/form-data; boundary=----BetterGraphQLBoundary

------BetterGraphQLBoundary
Content-Disposition: form-data; name="operations"

{"query":"mutation($files: List<FileUpload>!) { uploadFiles(files: $files) { url } }","variables":{"files":[null,null,null]}}
------BetterGraphQLBoundary
Content-Disposition: form-data; name="map"

{"0":["variables.files.0"],"1":["variables.files.1"],"2":["variables.files.2"]}
------BetterGraphQLBoundary
Content-Disposition: form-data; name="0"; filename="file1.pdf"
Content-Type: application/pdf

<binary>
------BetterGraphQLBoundary
Content-Disposition: form-data; name="1"; filename="file2.pdf"
Content-Type: application/pdf

<binary>
------BetterGraphQLBoundary
Content-Disposition: form-data; name="2"; filename="file3.pdf"
Content-Type: application/pdf

<binary>
------BetterGraphQLBoundary--
```

### 2.6 Chunked Stream Upload

For large files or streaming uploads, use chunked transfer encoding:

```
POST /graphql HTTP/1.1
Content-Type: application/json
Transfer-Encoding: chunked
X-Better-GraphQL-Stream: upload

{"query":"mutation($content: Stream) { uploadLargeFile(name: \"video.mp4\", content: $content) { url } }"}
<chunked binary data follows>
```

### 2.7 Audio/Video Upload with Transcoding

```graphql
mutation UploadVideo($video: VideoUpload) {
  uploadVideo(video: $video) {
    id
    url
    hlsUrl
    variants {
      quality
      url
    }
    # Transcoding progress (streamed)
    transcodingStatus @defer {
      progress
      completed
    }
  }
}
```

## 3. Response Format

### 3.1 Standard Response

```json
{
  "data": {
    "user": {
      "id": "123",
      "name": "John Doe"
    }
  }
}
```

### 3.2 Response with Errors (Typed Errors)

In Better GraphQL, errors are part of the data using union types:

```json
{
  "data": {
    "user": {
      "__typename": "NotFoundError",
      "message": "User not found",
      "resourceId": "123"
    }
  }
}
```

### 3.3 Transport Errors

Transport-level errors (not application errors) use a separate structure:

```json
{
  "errors": [
    {
      "message": "Syntax error in query",
      "locations": [{ "line": 1, "column": 15 }],
      "extensions": {
        "code": "SYNTAX_ERROR"
      }
    }
  ]
}
```

## 4. HTTP Headers

### 4.1 Request Headers

Better GraphQL defines standard request headers:

| Header | Description | Example |
|--------|-------------|---------|
| `Content-Type` | Request content type | `application/json` |
| `Accept` | Accepted response types | `application/json, multipart/mixed` |
| `Authorization` | Authentication token | `Bearer eyJhbG...` |
| `X-Request-ID` | Request tracing ID | `uuid-v4` |
| `X-BetterGraphQL-Version` | Protocol version | `1.0` |
| `X-BetterGraphQL-Defer-Mode` | Streaming preference | `multipart` or `sse` |

### 4.2 Response Headers

| Header | Description | Example |
|--------|-------------|---------|
| `Content-Type` | Response content type | `application/json` |
| `X-Request-ID` | Request tracing ID | `uuid-v4` |
| `X-RateLimit-Limit` | Rate limit maximum | `1000` |
| `X-RateLimit-Remaining` | Remaining requests | `999` |
| `X-RateLimit-Reset` | Reset timestamp | `1640995200` |
| `Cache-Control` | Cache directives | `max-age=60, private` |

### 4.3 Accessing Headers in Resolvers

Schema definition:
```graphql
type Query {
  me @header(name: "Authorization"): Option<User>
  requestInfo @header(name: "X-Request-ID"): RequestInfo
}

type Mutation {
  refreshToken: RefreshResult @header(name: "X-New-Token", write: true)
}
```

The server implementation receives header values and can set response headers.

## 5. Cookies

### 5.1 Cookie Configuration

Schema-level cookie configuration:

```graphql
directive @cookie(
  name: String
  options: Option<CookieOptions>
) on FIELD_DEFINITION | ARGUMENT_DEFINITION

input CookieOptions {
  maxAge: Option<Int>
  expires: Option<DateTime>
  path: String = "/"
  domain: Option<String>
  secure: Boolean = true
  httpOnly: Boolean = true
  sameSite: SameSite = Strict
}

enum SameSite {
  STRICT
  LAX
  NONE
}
```

### 5.2 Reading Cookies

```graphql
type Query {
  """Get current user from session cookie"""
  currentUser @cookie(name: "session"): Option<User>

  """Get user preferences from cookie"""
  preferences @cookie(name: "prefs"): Preferences
}
```

### 5.3 Writing Cookies

```graphql
type Mutation {
  login(credentials: LoginInput): LoginResult
    @cookie(name: "session", options: {
      maxAge: 604800,  # 7 days
      httpOnly: true,
      secure: true,
      sameSite: STRICT,
      path: "/"
    })

  logout: LogoutResult
    @cookie(name: "session", options: {
      maxAge: 0  # Delete cookie
    })
}
```

### 5.4 Cookie Response Headers

When a mutation sets a cookie, the response includes:

```http
Set-Cookie: session=abc123; Max-Age=604800; Path=/; Secure; HttpOnly; SameSite=Strict
```

## 6. CORS (Cross-Origin Resource Sharing)

### 6.1 Schema-level CORS Configuration

```graphql
schema @cors(
  origins: ["https://app.example.com", "https://admin.example.com"],
  methods: [GET, POST, OPTIONS],
  allowHeaders: ["Authorization", "Content-Type", "X-Request-ID"],
  exposeHeaders: ["X-RateLimit-Remaining", "X-RateLimit-Reset"],
  credentials: true,
  maxAge: 86400
) {
  query: Query
  mutation: Mutation
}
```

### 6.2 CORS Response Headers

For preflight requests (OPTIONS):

```http
HTTP/1.1 204 No Content
Access-Control-Allow-Origin: https://app.example.com
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Authorization, Content-Type, X-Request-ID
Access-Control-Expose-Headers: X-RateLimit-Remaining, X-RateLimit-Reset
Access-Control-Allow-Credentials: true
Access-Control-Max-Age: 86400
```

For actual requests:

```http
HTTP/1.1 200 OK
Access-Control-Allow-Origin: https://app.example.com
Access-Control-Allow-Credentials: true
Access-Control-Expose-Headers: X-RateLimit-Remaining, X-RateLimit-Reset
```

### 6.3 Dynamic CORS

For more complex CORS requirements:

```graphql
schema @cors(
  originsResolver: "resolveCorsOrigins",
  credentials: true
) {
  query: Query
}
```

The `resolveCorsOrigins` function is implemented server-side:

```javascript
function resolveCorsOrigins(request) {
  const origin = request.headers.get('Origin');
  const allowedPatterns = [
    /^https:\/\/.*\.example\.com$/,
    /^http:\/\/localhost:\d+$/
  ];
  return allowedPatterns.some(p => p.test(origin)) ? origin : null;
}
```

## 7. Authentication

### 7.1 Bearer Token Authentication

```graphql
type Query {
  me @requireAuth: User
  adminDashboard @requireAuth(roles: ["ADMIN"]): Dashboard
}
```

Request:
```http
POST /graphql HTTP/1.1
Authorization: Bearer eyJhbGciOiJIUzI1NiIs...
Content-Type: application/json

{"query": "{ me { id name } }"}
```

### 7.2 API Key Authentication

```graphql
type Query {
  publicData: Data
  privateData @requireAuth(header: "X-API-Key"): Data
}
```

### 7.3 Session-based Authentication

```graphql
type Query {
  me @cookie(name: "session") @requireAuth: User
}

type Mutation {
  login(input: LoginInput): LoginResult
    @cookie(name: "session", options: { httpOnly: true, secure: true })
}
```

## 8. Streaming Responses

### 8.1 Multipart Mixed Format

For `@defer` and `@stream` operations:

Request:
```http
POST /graphql HTTP/1.1
Accept: multipart/mixed; boundary="---"
Content-Type: application/json

{
  "query": "query { user(id: \"1\") { id name ... @defer { bio } } }"
}
```

Response:
```http
HTTP/1.1 200 OK
Content-Type: multipart/mixed; boundary="---"

-----
Content-Type: application/json

{"data":{"user":{"id":"1","name":"John"}},"hasNext":true}
-----
Content-Type: application/json

{"incremental":[{"path":["user"],"data":{"bio":"Developer"}}],"hasNext":false}
-------
```

### 8.2 Server-Sent Events (SSE) Format

Alternative streaming format:

Request:
```http
POST /graphql HTTP/1.1
Accept: text/event-stream
Content-Type: application/json

{
  "query": "query { user(id: \"1\") { id name ... @defer { bio } } }"
}
```

Response:
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

event: next
data: {"data":{"user":{"id":"1","name":"John"}},"hasNext":true}

event: next
data: {"incremental":[{"path":["user"],"data":{"bio":"Developer"}}],"hasNext":false}

event: complete
data:
```

### 8.3 Priority-based Streaming

With priority hints, the server optimizes delivery order:

```graphql
query {
  user(id: "1") {
    id
    name
    ... @defer(priority: 1) {
      avatarUrl
    }
    ... @defer(priority: 2) {
      statistics {
        postsCount
        followersCount
      }
    }
  }
}
```

## 9. Subscriptions

### 9.1 WebSocket Transport

Subscriptions use WebSocket with the graphql-ws protocol:

```
ws://example.com/graphql
```

Connection Init:
```json
{
  "type": "connection_init",
  "payload": {
    "Authorization": "Bearer token..."
  }
}
```

Subscribe:
```json
{
  "id": "1",
  "type": "subscribe",
  "payload": {
    "query": "subscription { userCreated { id name } }"
  }
}
```

### 9.2 SSE Transport

Alternative subscription transport:

```http
GET /graphql?query=subscription{userCreated{id,name}} HTTP/1.1
Accept: text/event-stream
Authorization: Bearer token...
```

Response:
```http
HTTP/1.1 200 OK
Content-Type: text/event-stream

event: next
data: {"data":{"userCreated":{"id":"1","name":"John"}}}

event: next
data: {"data":{"userCreated":{"id":"2","name":"Jane"}}}
```

## 10. Cancellation and Resumption

### 10.1 Request Cancellation

Clients can cancel in-flight requests:

- **HTTP/2**: Use stream reset
- **WebSocket**: Send unsubscribe message
- **SSE**: Close connection

### 10.2 Resumable Queries

For large streaming responses:

```graphql
query GetPosts @resumable {
  posts @stream(initialCount: 10) {
    id
    title
  }
}
```

Response includes resume tokens:
```json
{
  "data": {
    "posts": [...]
  },
  "extensions": {
    "resumeToken": "abc123..."
  }
}
```

Resume request:
```json
{
  "query": "query GetPosts @resumable { posts @stream { id title } }",
  "extensions": {
    "resumeToken": "abc123..."
  }
}
```

## 11. Caching

### 11.1 Cache-Control Headers

Schema definition:
```graphql
type Query {
  popularPosts: List<Post> @cache(maxAge: 3600, scope: PUBLIC)
  myFeed: List<Post> @cache(maxAge: 300, scope: PRIVATE)
  realTimeData: Data @noCache
}
```

Response headers:
```http
Cache-Control: max-age=3600, public
Cache-Control: max-age=300, private
Cache-Control: no-store
```

### 11.2 ETags

For conditional requests:

```http
GET /graphql?query={user(id:"1"){id,name}} HTTP/1.1
If-None-Match: "abc123"
```

Response (not modified):
```http
HTTP/1.1 304 Not Modified
ETag: "abc123"
```

## 12. Error Handling

### 12.1 HTTP Status Codes

| Status | Meaning |
|--------|---------|
| `200` | Success (even with typed errors in data) |
| `400` | Invalid query syntax |
| `401` | Authentication required |
| `403` | Forbidden |
| `429` | Rate limited |
| `500` | Server error |

### 12.2 Error Response Format

```json
{
  "errors": [
    {
      "message": "Invalid query syntax",
      "locations": [{ "line": 1, "column": 10 }],
      "extensions": {
        "code": "SYNTAX_ERROR",
        "suggestion": "Did you mean 'user'?"
      }
    }
  ]
}
```

## 13. Content Security Policy (CSP)

Better GraphQL treats CSP as a first-class citizen, providing native configuration and enforcement.

### 13.1 Schema-level CSP Configuration

```graphql
schema @csp(
  defaultSrc: ["'self'"],
  scriptSrc: ["'self'", "'wasm-unsafe-eval'"],
  styleSrc: ["'self'", "'unsafe-inline'"],
  imgSrc: ["'self'", "data:", "https://cdn.example.com"],
  connectSrc: ["'self'", "https://api.example.com", "wss://api.example.com"],
  fontSrc: ["'self'", "https://fonts.gstatic.com"],
  objectSrc: ["'none'"],
  baseUri: ["'self'"],
  formAction: ["'self'"],
  frameAncestors: ["'none'"],
  upgradeInsecureRequests: true,
  reportUri: "/csp-report",
  reportTo: "csp-endpoint"
) {
  query: Query
  mutation: Mutation
  subscription: Subscription
}
```

### 13.2 CSP Directive Type

```graphql
directive @csp(
  defaultSrc: Option<List<String>>
  scriptSrc: Option<List<String>>
  scriptSrcElem: Option<List<String>>
  scriptSrcAttr: Option<List<String>>
  styleSrc: Option<List<String>>
  styleSrcElem: Option<List<String>>
  styleSrcAttr: Option<List<String>>
  imgSrc: Option<List<String>>
  connectSrc: Option<List<String>>
  fontSrc: Option<List<String>>
  objectSrc: Option<List<String>>
  mediaSrc: Option<List<String>>
  frameSrc: Option<List<String>>
  childSrc: Option<List<String>>
  workerSrc: Option<List<String>>
  manifestSrc: Option<List<String>>
  baseUri: Option<List<String>>
  formAction: Option<List<String>>
  frameAncestors: Option<List<String>>
  sandbox: Option<List<SandboxToken>>
  reportUri: Option<String>
  reportTo: Option<String>
  upgradeInsecureRequests: Boolean = false
  blockAllMixedContent: Boolean = false
  requireTrustedTypesFor: Option<List<TrustedTypeRequire>>
  trustedTypes: Option<TrustedTypesConfig>
) on SCHEMA

enum SandboxToken {
  AllowForms
  AllowModals
  AllowOrientationLock
  AllowPointerLock
  AllowPopups
  AllowPopupsToEscapeSandbox
  AllowPresentation
  AllowSameOrigin
  AllowScripts
  AllowTopNavigation
  AllowTopNavigationByUserActivation
}

enum TrustedTypeRequire {
  Script
}

input TrustedTypesConfig {
  policies: List<String>
  allowDuplicates: Boolean = false
  defaultPolicy: Option<String>
}
```

### 13.3 CSP Response Headers

The server automatically generates CSP headers from schema configuration:

```http
HTTP/1.1 200 OK
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https://cdn.example.com; connect-src 'self' https://api.example.com wss://api.example.com; font-src 'self' https://fonts.gstatic.com; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; upgrade-insecure-requests; report-uri /csp-report
```

### 13.4 Nonce-based Script Execution

For inline scripts with strict CSP:

```graphql
schema @csp(
  scriptSrc: ["'self'", "'nonce-{request.nonce}'"],
  styleSrc: ["'self'", "'nonce-{request.nonce}'"]
) {
  query: Query
}
```

Server generates unique nonce per request:

```http
Content-Security-Policy: script-src 'self' 'nonce-abc123xyz'; style-src 'self' 'nonce-abc123xyz'
```

The nonce is available in the response context:

```typescript
// Server-side
const nonce = ctx.security.nonce; // "abc123xyz"

// Client receives nonce in response headers or meta tag
<script nonce="abc123xyz">/* inline script */</script>
```

### 13.5 CSP Violation Reporting

```graphql
type Mutation {
  reportCspViolation(report: CspViolationReport): Boolean @internal
}

input CspViolationReport {
  documentUri: String
  referrer: Option<String>
  blockedUri: String
  violatedDirective: String
  effectiveDirective: String
  originalPolicy: String
  disposition: CspDisposition
  statusCode: Int
  sourceFile: Option<String>
  lineNumber: Option<Int>
  columnNumber: Option<Int>
  sample: Option<String>
}

enum CspDisposition {
  Enforce
  Report
}
```

### 13.6 Report-Only Mode

For testing CSP without enforcement:

```graphql
schema @csp(
  defaultSrc: ["'self'"],
  reportOnly: true,
  reportUri: "/csp-report"
) {
  query: Query
}
```

Response header:

```http
Content-Security-Policy-Report-Only: default-src 'self'; report-uri /csp-report
```

### 13.7 Dynamic CSP Based on Request

```graphql
schema @csp(
  resolver: "resolveCsp"
) {
  query: Query
}
```

```typescript
// Server-side CSP resolver
function resolveCsp(ctx: Context): CspConfig {
  if (ctx.auth.user?.isAdmin) {
    // Relaxed CSP for admin panel
    return {
      scriptSrc: ["'self'", "'unsafe-eval'"],
      styleSrc: ["'self'", "'unsafe-inline'"],
    };
  }

  // Strict CSP for regular users
  return {
    scriptSrc: ["'self'"],
    styleSrc: ["'self'"],
    requireTrustedTypesFor: ["'script'"],
  };
}
```

## 14. Trusted Types

Better GraphQL provides first-class support for Trusted Types to prevent DOM XSS attacks.

### 14.1 Schema-level Trusted Types Configuration

```graphql
schema @csp(
  requireTrustedTypesFor: [Script],
  trustedTypes: {
    policies: ["better-graphql", "dompurify"],
    allowDuplicates: false,
    defaultPolicy: "better-graphql"
  }
) {
  query: Query
}
```

Generated CSP header:

```http
Content-Security-Policy: require-trusted-types-for 'script'; trusted-types better-graphql dompurify
```

### 14.2 Trusted Type Definitions

```graphql
# Built-in Trusted Types as scalars
scalar TrustedHTML
scalar TrustedScript
scalar TrustedScriptURL

# Fields can return Trusted Types
type RichContent {
  """Raw HTML - requires TrustedHTML for safe DOM insertion"""
  html: TrustedHTML

  """Script content - requires TrustedScript"""
  script: Option<TrustedScript>

  """External script URL - requires TrustedScriptURL"""
  scriptUrl: Option<TrustedScriptURL>
}

type Query {
  richContent(id: ContentId): RichContent
  sanitizedHtml(input: String): TrustedHTML
}
```

### 14.3 Client-Side Trusted Types Integration

```typescript
// generated/trusted-types.ts

// Policy creation (called once at app initialization)
export function initTrustedTypes(): void {
  if (window.trustedTypes) {
    window.trustedTypes.createPolicy("better-graphql", {
      createHTML: (input: string) => {
        // Use DOMPurify or similar sanitizer
        return DOMPurify.sanitize(input);
      },
      createScript: (input: string) => {
        // Only allow known safe scripts
        if (isAllowedScript(input)) {
          return input;
        }
        throw new Error("Untrusted script blocked");
      },
      createScriptURL: (input: string) => {
        const url = new URL(input);
        if (ALLOWED_SCRIPT_HOSTS.includes(url.host)) {
          return input;
        }
        throw new Error("Untrusted script URL blocked");
      },
    });
  }
}

// Type-safe TrustedHTML wrapper
export interface TrustedHTML {
  readonly __brand: "TrustedHTML";
  readonly value: globalThis.TrustedHTML | string;
}

export interface TrustedScript {
  readonly __brand: "TrustedScript";
  readonly value: globalThis.TrustedScript | string;
}

export interface TrustedScriptURL {
  readonly __brand: "TrustedScriptURL";
  readonly value: globalThis.TrustedScriptURL | string;
}
```

### 14.4 Vue Integration with Trusted Types

```typescript
// composables/useTrustedHtml.ts
import { computed, type Ref } from "vue";
import type { TrustedHTML } from "@/generated/trusted-types";

export function useTrustedHtml(html: Ref<TrustedHTML | null>) {
  const policy = window.trustedTypes?.defaultPolicy;

  return computed(() => {
    if (!html.value) return null;

    // Already trusted from server
    if (html.value.value instanceof TrustedHTML) {
      return html.value.value;
    }

    // Create trusted value via policy
    if (policy) {
      return policy.createHTML(html.value.value as string);
    }

    // Fallback for browsers without Trusted Types
    return html.value.value;
  });
}
```

```vue
<script setup lang="ts">
import { useTrustedHtml } from "@/composables/useTrustedHtml";
import { useRichContent } from "@/generated/composables";

const { data } = useRichContent({ id: contentId });

const trustedHtml = useTrustedHtml(
  computed(() => data.value?.html ?? null)
);
</script>

<template>
  <!-- Safe: uses Trusted Types -->
  <div v-html="trustedHtml" />
</template>
```

### 14.5 Server-Side HTML Sanitization

```graphql
type Query {
  """
  Returns sanitized HTML as TrustedHTML.
  Server sanitizes with DOMPurify before returning.
  """
  article(id: ArticleId): Article
}

type Article {
  id: ArticleId
  title: String

  """Server-sanitized HTML content"""
  content: TrustedHTML @sanitize(
    allowedTags: ["p", "a", "strong", "em", "ul", "ol", "li", "h1", "h2", "h3"],
    allowedAttributes: {
      a: ["href", "title"],
      img: ["src", "alt"]
    },
    allowedSchemes: ["https", "mailto"]
  )
}
```

### 14.6 @sanitize Directive

```graphql
directive @sanitize(
  allowedTags: Option<List<String>>
  allowedAttributes: Option<JSON>
  allowedSchemes: Option<List<String>>
  allowedClasses: Option<JSON>
  stripIgnoreTag: Boolean = true
  stripIgnoreTagBody: Option<List<String>>
  transformTags: Option<JSON>
) on FIELD_DEFINITION
```

Server implementation:

```typescript
// Server-side sanitization
const resolvers = {
  Article: {
    content: async (article, _, ctx) => {
      const rawHtml = await ctx.db.articles.getContent(article.id);

      // Apply @sanitize directive configuration
      const sanitized = DOMPurify.sanitize(rawHtml, {
        ALLOWED_TAGS: ["p", "a", "strong", "em", "ul", "ol", "li", "h1", "h2", "h3"],
        ALLOWED_ATTR: ["href", "title", "src", "alt"],
        ALLOWED_URI_REGEXP: /^https?:|^mailto:/i,
      });

      return {
        __typename: "TrustedHTML",
        value: sanitized,
      };
    },
  },
};
```

### 14.7 Trusted Types Enforcement Levels

```graphql
schema @trustedTypes(
  enforcement: Strict,  # Strict | Permissive | ReportOnly
  defaultPolicy: "better-graphql",
  fallbackPolicy: "dompurify"
) {
  query: Query
}

enum TrustedTypesEnforcement {
  """Full enforcement - blocks all untrusted values"""
  Strict

  """Enforcement with fallback sanitization"""
  Permissive

  """Report violations without blocking"""
  ReportOnly
}
```

### 14.8 Type-Safe DOM Sink Protection

```typescript
// Generated type-safe DOM helpers
import type { TrustedHTML, TrustedScript, TrustedScriptURL } from "./trusted-types";

// Type-safe innerHTML assignment
export function setInnerHTML(
  element: Element,
  html: TrustedHTML
): void {
  element.innerHTML = html.value as unknown as string;
}

// Type-safe script content
export function setScriptContent(
  script: HTMLScriptElement,
  content: TrustedScript
): void {
  script.text = content.value as unknown as string;
}

// Type-safe script src
export function setScriptSrc(
  script: HTMLScriptElement,
  url: TrustedScriptURL
): void {
  script.src = url.value as unknown as string;
}

// Compile-time error if raw string is used
setInnerHTML(div, "<script>alert('xss')</script>");
// Error: Argument of type 'string' is not assignable to parameter of type 'TrustedHTML'
```

### 14.9 Integration with Streaming Responses

For `@defer` and `@stream` with HTML content:

```graphql
type Article {
  id: ArticleId
  title: String

  # Immediate sanitized summary
  summary: TrustedHTML @sanitize

  # Deferred full content
  fullContent: TrustedHTML @defer @sanitize(
    allowedTags: ["p", "a", "img", "video", "iframe"],
    allowedAttributes: {
      iframe: ["src", "width", "height", "sandbox"]
    }
  )
}
```

```typescript
const article = await client.getArticle({ id });

// Immediate - already sanitized
const summary = article.summary;

// Deferred - sanitized when resolved
const fullContent = await article.fullContent;

// Both are TrustedHTML - safe for v-html
```

### 14.10 CSP and Trusted Types Summary

| Feature | Purpose |
|---------|---------|
| Schema-level CSP | Define security policy in schema |
| Nonce support | Safe inline script execution |
| Report-Only mode | Test CSP without breaking |
| Violation reporting | Monitor CSP violations |
| TrustedHTML scalar | Type-safe HTML content |
| TrustedScript scalar | Type-safe script content |
| TrustedScriptURL scalar | Type-safe script URLs |
| @sanitize directive | Server-side HTML sanitization |
| Vue integration | Safe v-html with Trusted Types |
| Streaming support | Sanitization with @defer/@stream |

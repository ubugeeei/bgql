<p align="center">
  <img src="./assets/logo-full.svg" alt="bgql logo" width="400" />
</p>

<h1 align="center">bgql</h1>

<p align="center">
  <strong>Better GraphQL</strong> - A GraphQL superset with modern type system features.
</p>

<p align="center">
  <a href="https://github.com/ubugeeei/bgql/actions"><img src="https://github.com/ubugeeei/bgql/actions/workflows/ci.yml/badge.svg" alt="CI Status" /></a>
  <a href="https://github.com/ubugeeei/bgql/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="License" /></a>
</p>

---

> [!WARNING]
> **This project is under active development and is NOT production ready.**
> The API may change without notice. Do not use in production environments.

> [!NOTE]
> **Not yet published.** The crates and npm packages are not yet available on crates.io or npm.
> To use bgql, you need to build from source.

---

bgql extends GraphQL with type-safe features inspired by modern programming languages like Rust and TypeScript, while maintaining full compatibility with GraphQL tooling.

## Specification

> [!IMPORTANT]
> **For detailed specifications, please refer to the [`spec/`](./spec) directory.**

The specification covers all aspects of bgql:

| Document | Description |
|----------|-------------|
| [00-overview.md](./spec/00-overview.md) | Introduction and design goals |
| [01-type-system.md](./spec/01-type-system.md) | Type system (Option, List, Generics, Opaque, etc.) |
| [02-schema-definition.md](./spec/02-schema-definition.md) | Schema Definition Language (SDL) |
| [03-directives.md](./spec/03-directives.md) | Built-in and custom directives |
| [04-query-language.md](./spec/04-query-language.md) | Query language extensions |
| [05-http-protocol.md](./spec/05-http-protocol.md) | HTTP protocol and transport |
| [06-execution.md](./spec/06-execution.md) | Query execution model |
| [07-introspection.md](./spec/07-introspection.md) | Introspection system |
| [08-client-sdk.md](./spec/08-client-sdk.md) | Client SDK specification |
| [09-server-sdk-*.md](./spec) | Server SDK specifications (TypeScript, Rust, Go) |

## Features

### Type System Enhancements

#### `Option<T>` and `List<T>` Types

Explicit optional and list types instead of `!` for non-null:

```graphql
type User {
  id: ID                      # Non-null by default
  name: String
  bio: Option<String>         # Explicitly optional
  posts: List<Post>           # List of posts
  tags: List<Option<String>>  # List with optional elements
}
```

#### Opaque Types

Nominal typing for type-safe IDs and domain values:

```graphql
opaque UserId = ID
opaque PostId = ID
opaque EmailAddress = String @email

type User {
  id: UserId                  # Type-safe, not interchangeable with PostId
  email: EmailAddress
}
```

#### Generic Types

Type parameters with constraints:

```graphql
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

#### Type Aliases

Create named aliases for complex types:

```graphql
type alias UserConnection = Connection<User>
type alias PostConnection = Connection<Post>
```

#### Rust-style Enums with Data

Enums can carry associated data:

```graphql
enum UserResult {
  Ok(User)
  NotFound { id: UserId }
  Unauthorized { message: String }
}
```

#### Input Unions

Union types for input objects:

```graphql
input EmailCredentials {
  email: String @email
  password: String
}

input OAuthCredentials {
  provider: OAuthProvider
  token: String
}

input union LoginCredentials = EmailCredentials | OAuthCredentials
```

### Built-in Scalars

bgql includes additional built-in scalar types:

- `Uint` - Unsigned integer
- `DateTime` - ISO 8601 date-time
- `Date` - ISO 8601 date
- `HTML` - HTML content (with sanitization support)
- `JSON` - Arbitrary JSON data
- `File`, `Image`, `Audio`, `Video` - Media types
- `TrustedHTML`, `TrustedScript`, `TrustedScriptURL` - Trusted types

### Validation Directives

Built-in validation directives for input fields:

```graphql
input CreateUserInput {
  email: String @email
  password: String @minLength(8) @pattern(regex: "^(?=.*[A-Za-z])(?=.*\\d).+$")
  name: String @minLength(1) @maxLength(100) @trim
  website: Option<String> @url
  tags: List<String> @maxItems(10)
}
```

### Server-side Fragments

Pre-defined fragments for server-side optimization:

```graphql
fragment UserBasic on User @server {
  id
  name
  avatarUrl
  isVerified
}
```

## Installation

> [!NOTE]
> Packages are not yet published. Build from source for now.

### From Source

```bash
# Clone the repository
git clone https://github.com/ubugeeei/bgql.git
cd bgql

# Build all crates
cargo build --release

# Build WASM module
wasm-pack build crates/bgql_wasm --target web
```

### Rust (coming soon)

```toml
[dependencies]
bgql_core = "0.1"
```

### Node.js (coming soon)

```bash
npm install @bgql/core
```

### WebAssembly

```javascript
import init, { BetterGraphQL } from '@bgql/wasm';

await init();
const bgql = new BetterGraphQL();
const result = bgql.parse(`
  type Query {
    hello: String
  }
`);
```

## Usage

### Parsing a Schema

```rust
use bgql_parser::{Parser, Interner};

let source = r#"
  type User {
    id: UserId
    name: String
    email: Option<String>
  }

  opaque UserId = ID
"#;

let interner = Interner::new();
let mut parser = Parser::new(source, &interner);
let document = parser.parse_document();
let diagnostics = parser.take_diagnostics();

if diagnostics.has_errors() {
    for error in diagnostics.errors() {
        eprintln!("{}: {}", error.code, error.title);
    }
}
```

### Using the WASM Module

```javascript
import init, { BetterGraphQL } from 'bgql_wasm';

async function main() {
  await init();

  const bgql = new BetterGraphQL();

  const result = bgql.parse(`
    type User implements Node {
      id: UserId
      name: String
      email: Option<String>
    }

    opaque UserId = ID

    interface Node {
      id: ID
    }
  `);

  if (result.success) {
    console.log('Types:', result.types);
  } else {
    console.log('Errors:', result.diagnostics);
  }
}
```

## Project Structure

```
bgql/
├── crates/
│   ├── bgql_arena/          # Arena allocator
│   ├── bgql_span/           # Source spans
│   ├── bgql_text/           # String interner
│   ├── bgql_diagnostics/    # Error reporting
│   ├── bgql_syntax/         # Token definitions
│   ├── bgql_lexer/          # Lexer
│   ├── bgql_parser/         # Parser
│   ├── bgql_ast/            # AST types
│   ├── bgql_types/          # Type system
│   ├── bgql_schema/         # Schema handling
│   ├── bgql_lsp/            # Language Server
│   ├── bgql_codegen/        # Code generation
│   ├── bgql_wasm/           # WebAssembly bindings
│   └── bgql_cli/            # CLI tool
├── playground/              # Web playground (Vue 3)
└── examples/                # Example schemas
```

## Playground

Try bgql in the browser: [bgql Playground](https://ubugeeei.github.io/better-graphql/)

Or run locally:

```bash
cd playground
bun install
bun run dev
```

## Development

### Prerequisites

- Rust 1.75+
- wasm-pack (for WebAssembly)
- Bun or Node.js (for playground)

### Building

```bash
# Build all crates
cargo build

# Build WASM module
cd crates/bgql_wasm
wasm-pack build --target web

# Run tests
cargo test

# Run playground
cd playground
bun run dev
```

## Examples

See the [examples](./examples) directory for complete schema examples.

### Basic Schema

```graphql
# User type with opaque ID
opaque UserId = ID

type User implements Node {
  id: UserId
  name: String
  email: Option<String>
  posts: List<Post>
  createdAt: DateTime
}

# Post with validation
type Post implements Node {
  id: PostId
  title: String @minLength(1) @maxLength(200)
  content: HTML
  author: User
  tags: List<Tag>
  publishedAt: Option<DateTime>
}

opaque PostId = ID

# Generic connection type
type Connection<T extends Node> {
  edges: List<Edge<T>>
  pageInfo: PageInfo
  totalCount: Uint
}

type alias UserConnection = Connection<User>

interface Node {
  id: ID
}
```

## Roadmap

- [x] Lexer and Parser
- [x] `Option<T>` and `List<T>` types
- [x] Opaque types
- [x] Generic types with constraints
- [x] Type aliases
- [x] Input unions
- [x] Rust-style enums with data
- [x] WebAssembly bindings
- [x] Web playground
- [ ] Language Server Protocol (LSP)
- [ ] Code generation (TypeScript, Rust)
- [ ] Schema validation
- [ ] Query execution engine

## License

MIT OR Apache-2.0

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

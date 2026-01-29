# CLI Overview

The Better GraphQL CLI provides tools for schema validation, code generation, formatting, and development.

## Installation

```bash
# With Cargo (recommended)
cargo install bgql

# Or with npm
npm install -g @bgql/cli
```

Verify installation:

```bash
bgql --version
```

## Commands

| Command | Description |
|---------|-------------|
| `bgql check` | Validate schema files |
| `bgql fmt` | Format schema files |
| `bgql codegen` | Generate code from schema |
| `bgql parse` | Parse and display AST |
| `bgql lsp` | Start the language server |

## bgql check

Validate GraphQL schema files:

```bash
# Check a single file
bgql check schema.bgql

# Check multiple files
bgql check schema/*.bgql

# Strict mode (treat warnings as errors)
bgql check --strict schema.bgql

# Verbose output
bgql -v check schema.bgql
```

### Output Example

```
Checking schema.bgql
  --> Error: Undefined type `Profile`
      at schema.bgql:15:12
      |
   15 |   profile: Profile
      |            ^^^^^^^ Type `Profile` is not defined

  --> Warning: Field name `Name` should be camelCase
      at schema.bgql:8:3
      |
    8 |   Name: String
      |   ^^^^ Field names should start with a lowercase letter

Error: 1 error and 1 warning found
```

## bgql fmt

Format schema files:

```bash
# Format a file (in-place)
bgql fmt schema.bgql

# Format multiple files
bgql fmt schema/*.bgql

# Check formatting without modifying
bgql fmt --check schema.bgql

# Custom indentation
bgql fmt --indent 4 schema.bgql

# Use tabs
bgql fmt --tabs schema.bgql
```

### Before/After

```graphql
# Before
type User{id:ID name:String
email:String posts:List<Post>}

# After formatting
type User {
  id: ID
  name: String
  email: String
  posts: List<Post>
}
```

## bgql codegen

Generate code from your schema:

```bash
# Generate TypeScript
bgql codegen schema.bgql -o ./generated/types.ts

# Generate Rust
bgql codegen schema.bgql --lang rust -o ./generated/types.rs

# Generate Go
bgql codegen schema.bgql --lang go -o ./generated/types.go
```

### TypeScript Output

```typescript
// generated/types.ts

// Scalar types
export type ID = string;
export type DateTime = string;

// Interfaces
export interface Node {
  id: ID;
}

// Types
export interface User extends Node {
  id: ID;
  name: string;
  email: string;
  bio: string | null;
  posts: Post[];
}

// Input types
export interface CreateUserInput {
  name: string;
  email: string;
}

// Enums
export type UserRole = 'ADMIN' | 'USER' | 'GUEST';

// Unions
export type UserResult =
  | { __typename: 'User' } & User
  | { __typename: 'NotFoundError' } & NotFoundError;

// Type guards
export function isUser(value: UserResult): value is User & { __typename: 'User' } {
  return value.__typename === 'User';
}

// Document nodes
export const GetUserDocument: TypedDocumentNode<GetUserQuery, GetUserQueryVariables>;
```

### Watch Mode

```bash
# Regenerate on schema changes
bgql codegen schema.bgql -o ./generated/types.ts --watch
```

## bgql parse

Parse and display the AST (useful for debugging):

```bash
# Pretty print AST
bgql parse schema.bgql

# JSON output
bgql parse schema.bgql --format json
```

## bgql lsp

Start the Language Server Protocol server:

```bash
bgql lsp
```

This is typically called by your editor. See [Editor Integration](#editor-integration).

## Global Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output |
| `-q, --quiet` | Suppress non-error output |
| `--version` | Print version |
| `--help` | Print help |

## Configuration File

Create a `bgql.config.json` in your project root:

```json
{
  "schema": "./schema/mod.bgql",
  "codegen": {
    "typescript": {
      "output": "./src/generated/types.ts",
      "scalars": {
        "DateTime": "Date",
        "JSON": "Record<string, unknown>"
      }
    }
  },
  "check": {
    "strict": true,
    "maxDepth": 10,
    "maxComplexity": 1000
  },
  "format": {
    "indent": 2,
    "useTabs": false
  }
}
```

Then run commands without arguments:

```bash
bgql check    # Uses schema from config
bgql codegen  # Uses codegen config
```

## Editor Integration

### VS Code

Install the Better GraphQL extension:

```bash
code --install-extension ubugeeei.bgql-vscode
```

Or add to `.vscode/settings.json`:

```json
{
  "[bgql]": {
    "editor.defaultFormatter": "ubugeeei.bgql-vscode"
  },
  "bgql.schema": "./schema/mod.bgql"
}
```

### Neovim

With `nvim-lspconfig`:

```lua
require('lspconfig').bgql.setup {
  cmd = { 'bgql', 'lsp' },
  filetypes = { 'bgql', 'graphql' },
  root_dir = function(fname)
    return lspconfig.util.find_git_ancestor(fname)
  end,
}
```

### Other Editors

The LSP server works with any editor supporting the Language Server Protocol. Configure your editor to run `bgql lsp` for `.bgql` files.

## CI/CD Integration

### GitHub Actions

```yaml
name: Schema Check

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install bgql
        run: cargo install bgql

      - name: Check schema
        run: bgql check --strict schema/**/*.bgql

      - name: Check formatting
        run: bgql fmt --check schema/**/*.bgql
```

### Pre-commit Hook

```bash
# .husky/pre-commit
bgql check schema.bgql
bgql fmt --check schema.bgql
```

Or with lint-staged:

```json
{
  "lint-staged": {
    "*.bgql": ["bgql fmt", "bgql check"]
  }
}
```

## Next Steps

- [Code Generation](/cli/codegen)
- [Schema Types](/schema/types)
- [Backend Setup](/backend/quickstart)

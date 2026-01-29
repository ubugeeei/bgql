# Code Generation

Generate type-safe code from your Better GraphQL schema.

## Overview

Better GraphQL codegen generates TypeScript types, documents, and helpers from your schema and GraphQL operations.

## Basic Usage

```bash
bgql codegen schema.bgql -o ./generated
```

## Configuration

### Config File

Create `codegen.yaml` in your project root:

```yaml
schema: ./schema/mod.bgql
documents:
  - src/**/*.graphql
  - src/**/*.vue
  - src/**/*.tsx
generates:
  ./src/generated/graphql.ts:
    plugins:
      - typescript
      - typescript-operations
      - typed-document-node
```

### Running with Config

```bash
# Uses codegen.yaml by default
bgql codegen

# Or specify config file
bgql codegen --config custom-codegen.yaml
```

## Configuration Options

### Schema

```yaml
# Single file
schema: schema.bgql

# Multiple files
schema:
  - schema/users.bgql
  - schema/posts.bgql

# Glob pattern
schema: schema/**/*.bgql

# Remote schema
schema: http://localhost:4000/graphql
```

### Documents

```yaml
# Glob patterns
documents:
  - src/**/*.graphql
  - src/**/*.gql

# Include operations in components
documents:
  - src/**/*.vue
  - src/**/*.tsx
  - src/**/*.svelte

# Exclude patterns
documents:
  - src/**/*.graphql
  - "!src/**/*.test.graphql"
```

### Output Configuration

```yaml
generates:
  # Single output file
  ./src/generated/graphql.ts:
    plugins:
      - typescript
      - typescript-operations

  # Multiple outputs
  ./src/generated/types.ts:
    plugins:
      - typescript

  ./src/generated/operations.ts:
    plugins:
      - typescript-operations
    preset: import-types
    presetConfig:
      typesPath: ./types
```

## Plugins

### typescript

Generates TypeScript type definitions from schema.

```yaml
generates:
  ./generated/types.ts:
    plugins:
      - typescript
    config:
      # Use readonly types
      immutableTypes: true

      # Generate enums as union types
      enumsAsTypes: true

      # Strict scalar types
      strictScalars: true

      # Custom scalar mappings
      scalars:
        DateTime: string
        JSON: Record<string, unknown>
```

**Output:**

```typescript
export interface User {
  readonly __typename: 'User';
  readonly id: string;
  readonly name: string;
  readonly email: string;
  readonly createdAt: string;
}

export type Role = 'USER' | 'ADMIN' | 'SUPER_ADMIN';
```

### typescript-operations

Generates types for GraphQL operations.

```yaml
generates:
  ./generated/operations.ts:
    plugins:
      - typescript-operations
    config:
      # Skip __typename in response types
      skipTypename: false

      # Use Pick for operation types
      preResolveTypes: true
```

**Output:**

```typescript
export interface GetUserQuery {
  readonly user: {
    readonly __typename: 'User';
    readonly id: string;
    readonly name: string;
  } | {
    readonly __typename: 'NotFoundError';
    readonly message: string;
  };
}

export interface GetUserQueryVariables {
  readonly id: string;
}
```

### typed-document-node

Generates TypedDocumentNode for type-safe operations.

```yaml
generates:
  ./generated/graphql.ts:
    plugins:
      - typescript
      - typescript-operations
      - typed-document-node
```

**Output:**

```typescript
export const GetUserDocument: TypedDocumentNode<
  GetUserQuery,
  GetUserQueryVariables
> = {"kind":"Document","definitions":[...]};
```

## Custom Scalars

### Configuration

```yaml
generates:
  ./generated/graphql.ts:
    plugins:
      - typescript
    config:
      scalars:
        DateTime: Date
        JSON: Record<string, unknown>
        Upload: File
        BigInt: bigint
```

### With Custom Types

```yaml
config:
  scalars:
    DateTime: import('date-fns').Date
    Money: import('./types').Money
```

## Opaque Types

### Configuration

```yaml
generates:
  ./generated/graphql.ts:
    plugins:
      - typescript
    config:
      # Generate branded types for opaque types
      opaqueTypes: true
```

**Output:**

```typescript
// For: opaque UserId = ID
export type UserId = string & { readonly __brand: 'UserId' };

// For: opaque Email = String @email
export type Email = string & { readonly __brand: 'Email' };
```

## Watch Mode

### Basic Watch

```bash
bgql codegen --watch
```

### With Custom Debounce

```yaml
watch:
  debounce: 500  # ms
  ignorePatterns:
    - "**/node_modules/**"
    - "**/*.generated.ts"
```

## Integration

### With Build Tools

**package.json:**

```json
{
  "scripts": {
    "codegen": "bgql codegen",
    "codegen:watch": "bgql codegen --watch",
    "prebuild": "bun run codegen",
    "predev": "bun run codegen"
  }
}
```

### With Vite

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import { bgqlPlugin } from '@bgql/vite-plugin';

export default defineConfig({
  plugins: [
    bgqlPlugin({
      configFile: './codegen.yaml',
      watch: true,
    }),
  ],
});
```

### With TypeScript

**tsconfig.json:**

```json
{
  "compilerOptions": {
    "paths": {
      "@generated/*": ["./src/generated/*"]
    }
  },
  "include": ["src", "src/generated"]
}
```

## Best Practices

### 1. Run Codegen in CI

```yaml
# .github/workflows/ci.yml
jobs:
  check:
    steps:
      - run: bun run codegen
      - run: git diff --exit-code src/generated
```

### 2. Commit Generated Files

```gitignore
# .gitignore
# Don't ignore generated files - commit them
# !src/generated/
```

### 3. Use Strict Configuration

```yaml
config:
  strictScalars: true
  immutableTypes: true
  enumsAsTypes: true
  opaqueTypes: true
```

### 4. Organize by Feature

```yaml
generates:
  src/features/users/generated.ts:
    documents: src/features/users/**/*.graphql
    plugins:
      - typescript-operations

  src/features/posts/generated.ts:
    documents: src/features/posts/**/*.graphql
    plugins:
      - typescript-operations
```

## Troubleshooting

### Schema Not Found

```
Error: Schema file not found: schema.bgql
```

Check the path in your config or use absolute path.

### Unknown Scalar

```
Error: Unknown scalar type 'DateTime'
```

Add scalar mapping in config:

```yaml
config:
  scalars:
    DateTime: string
```

### Circular Dependencies

Use import-types preset to avoid circular imports between types and operations.

## Next Steps

- [CLI Commands](/cli/commands)
- [Type System](/guide/type-system)
- [Type Safety](/frontend/type-safety)

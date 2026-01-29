# CLI Commands

Better GraphQL CLI reference for all available commands.

## Installation

```bash
# Install globally
bun add -g @bgql/cli

# Or use with bunx
bunx @bgql/cli <command>
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `bgql check` | Validate schema files |
| `bgql codegen` | Generate TypeScript types |
| `bgql format` | Format schema files |
| `bgql lsp` | Start language server |
| `bgql init` | Initialize a new project |
| `bgql serve` | Start development server |

## bgql check

Validate schema files for errors.

### Usage

```bash
bgql check [options] <path>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<path>` | Path to schema file or directory |

### Options

| Option | Description |
|--------|-------------|
| `--strict` | Enable strict mode |
| `--warn` | Show warnings as errors |
| `--format <format>` | Output format: `text`, `json` |

### Examples

```bash
# Check single file
bgql check schema.bgql

# Check directory
bgql check ./schema

# Strict mode with JSON output
bgql check --strict --format json ./schema
```

### Output

```
Checking schema...

schema/users/mod.bgql:12:5 error: Unknown type 'Usre'
   12 |   author: Usre
      |           ^^^^

schema/posts/mod.bgql:8:3 warning: Field 'createdAt' missing Option wrapper
    8 |   createdAt: DateTime
      |   ^^^^^^^^^

Found 1 error and 1 warning
```

## bgql codegen

Generate TypeScript types from schema.

### Usage

```bash
bgql codegen [options] <schema>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<schema>` | Path to schema file or directory |

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <path>` | Output directory (default: `./generated`) |
| `--lang <language>` | Target language: `typescript`, `rust` |
| `--watch` | Watch for changes |
| `--documents <glob>` | GraphQL documents to include |
| `--config <path>` | Config file path |

### Examples

```bash
# Basic generation
bgql codegen schema.bgql -o ./generated

# With documents
bgql codegen schema.bgql --documents "src/**/*.graphql" -o ./generated

# Watch mode
bgql codegen schema.bgql -o ./generated --watch

# From config file
bgql codegen --config codegen.yaml
```

## bgql format

Format schema files.

### Usage

```bash
bgql format [options] <files...>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<files...>` | Files or glob patterns to format |

### Options

| Option | Description |
|--------|-------------|
| `--check` | Check if files are formatted (no write) |
| `--write` | Write formatted files (default) |
| `--stdin` | Read from stdin |

### Examples

```bash
# Format single file
bgql format schema.bgql

# Format multiple files
bgql format "schema/**/*.bgql"

# Check formatting (CI)
bgql format --check "schema/**/*.bgql"
```

## bgql lsp

Start the language server.

### Usage

```bash
bgql lsp [options]
```

### Options

| Option | Description |
|--------|-------------|
| `--stdio` | Use stdio transport (default) |
| `--socket <port>` | Use socket transport |
| `--log <level>` | Log level: `error`, `warn`, `info`, `debug` |

### Examples

```bash
# Start with stdio (for editors)
bgql lsp

# Start with socket
bgql lsp --socket 9000

# Debug mode
bgql lsp --log debug
```

### Editor Configuration

**VS Code** (via extension):
```json
{
  "bgql.lsp.enabled": true
}
```

**Neovim** (via nvim-lspconfig):
```lua
require('lspconfig').bgql.setup({
  cmd = { 'bgql', 'lsp' },
  filetypes = { 'bgql', 'graphql' },
})
```

## bgql init

Initialize a new Better GraphQL project.

### Usage

```bash
bgql init [options] [directory]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `[directory]` | Project directory (default: current) |

### Options

| Option | Description |
|--------|-------------|
| `--template <name>` | Project template |
| `--yes` | Skip prompts, use defaults |

### Templates

| Template | Description |
|----------|-------------|
| `basic` | Basic schema setup |
| `fullstack` | Full-stack with server and client |
| `api` | API server only |

### Examples

```bash
# Interactive setup
bgql init

# With template
bgql init --template fullstack my-project

# Quick setup with defaults
bgql init --yes
```

## bgql serve

Start a development server.

### Usage

```bash
bgql serve [options] <schema>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<schema>` | Path to schema file |

### Options

| Option | Description |
|--------|-------------|
| `-p, --port <port>` | Server port (default: 4000) |
| `--host <host>` | Server host (default: localhost) |
| `--mock` | Enable mock resolvers |
| `--watch` | Watch for schema changes |
| `--playground` | Enable GraphQL playground |

### Examples

```bash
# Start dev server
bgql serve schema.bgql

# Custom port with playground
bgql serve schema.bgql -p 3000 --playground

# Mock mode for frontend development
bgql serve schema.bgql --mock --watch
```

## Global Options

These options are available for all commands:

| Option | Description |
|--------|-------------|
| `-h, --help` | Show help |
| `-V, --version` | Show version |
| `--verbose` | Verbose output |
| `--quiet` | Suppress output |
| `--color` | Force color output |
| `--no-color` | Disable color output |

## Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Schema validation error |
| 3 | Configuration error |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `BGQL_CONFIG` | Config file path |
| `BGQL_LOG_LEVEL` | Log level |
| `NO_COLOR` | Disable colors |

## Next Steps

- [Code Generation](/cli/codegen)
- [Getting Started](/guide/getting-started)
- [Schema Types](/schema/types)

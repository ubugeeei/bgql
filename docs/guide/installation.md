# Installation

This guide covers all installation options for Better GraphQL.

## CLI

The CLI is used for schema validation, code generation, and running the language server.

### Using Cargo (Recommended)

```bash
cargo install bgql
```

### Using npm

```bash
npm install -g @bgql/cli
```

### Using Homebrew (macOS)

```bash
brew install ubugeeei/tap/bgql
```

### Verify Installation

```bash
bgql --version
```

## Server SDK

### Node.js

```bash
npm install @bgql/server
```

Or with other package managers:

```bash
# Yarn
yarn add @bgql/server

# pnpm
pnpm add @bgql/server

# Bun
bun add @bgql/server
```

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
bgql = "0.1"
bgql-server = "0.1"
```

## Client SDK

### JavaScript/TypeScript

```bash
npm install @bgql/client
```

### With Framework Integration

```bash
# Vue.js
npm install @bgql/client vue

# React (coming soon)
npm install @bgql/client @bgql/react
```

### Rust

```toml
[dependencies]
bgql-client = "0.1"
```

## Editor Support

### VS Code

Install from the marketplace:

```bash
code --install-extension ubugeeei.bgql-vscode
```

Or search for "Better GraphQL" in the VS Code extensions panel.

### Neovim

Using `lazy.nvim`:

```lua
{
  "ubugeeei/bgql.nvim",
  ft = { "bgql", "graphql" },
}
```

### JetBrains IDEs

Install the "Better GraphQL" plugin from the JetBrains marketplace.

## System Requirements

### CLI

- macOS 10.15+, Linux (glibc 2.17+), or Windows 10+
- 64-bit processor

### Node.js SDK

- Node.js 18+ (LTS recommended)
- npm 8+, yarn 3+, pnpm 8+, or bun

### Rust SDK

- Rust 1.75+

## Troubleshooting

### Cargo install fails

Make sure you have the latest Rust toolchain:

```bash
rustup update stable
```

### npm install fails on native modules

The SDK uses WebAssembly by default. If you need native bindings:

```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential

# Windows
npm install --global windows-build-tools
```

### LSP not working in editor

1. Verify the CLI is in your PATH:
   ```bash
   which bgql
   ```

2. Check the LSP output:
   ```bash
   bgql lsp 2>&1 | head -20
   ```

3. Ensure your editor is configured correctly (see [Editor Integration](/cli/overview#editor-integration))

## Next Steps

- [Getting Started](/guide/getting-started)
- [Backend Quick Start](/backend/quickstart)
- [Frontend Quick Start](/frontend/quickstart)

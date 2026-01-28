# Better GraphQL for VS Code

Better GraphQL language support for Visual Studio Code.

## Features

- Syntax highlighting for `.bgql` files
- Language Server Protocol support
  - Code completion
  - Go to definition
  - Find references
  - Hover documentation
  - Diagnostics and error reporting
- Formatting
- Code generation commands

## Installation

### From VS Code Marketplace

1. Open VS Code
2. Press `Ctrl+Shift+X` to open Extensions
3. Search for "Better GraphQL"
4. Click Install

### Manual Installation (VSIX)

1. Download the `.vsix` file from releases
2. Open VS Code
3. Press `Ctrl+Shift+P` and type "Install from VSIX"
4. Select the downloaded file

## Requirements

- VS Code 1.85.0 or higher
- `bgql` CLI tool (optional, for language server features)

## Commands

| Command | Description |
|---------|-------------|
| `Better GraphQL: Restart Language Server` | Restart the language server |
| `Better GraphQL: Format Document` | Format the current document |
| `Better GraphQL: Generate Types` | Generate TypeScript/Rust/Go types from schema |

## Configuration

```json
{
  "bgql.lsp.enabled": true,
  "bgql.lsp.path": "bgql",
  "bgql.format.tabSize": 2,
  "bgql.validation.enabled": true
}
```

### Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `bgql.lsp.enabled` | boolean | `true` | Enable the language server |
| `bgql.lsp.path` | string | `"bgql"` | Path to the bgql binary |
| `bgql.format.tabSize` | number | `2` | Tab size for formatting |
| `bgql.validation.enabled` | boolean | `true` | Enable schema validation |

## Development

```bash
# Install dependencies
npm install

# Compile
npm run compile

# Watch mode
npm run watch

# Package extension
npm run package
```

## Language Server

Install the Better GraphQL CLI to enable language server features:

```bash
cargo install bgql-cli
```

## License

MIT OR Apache-2.0

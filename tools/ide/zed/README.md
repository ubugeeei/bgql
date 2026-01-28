# Better GraphQL for Zed

Better GraphQL language support for [Zed](https://zed.dev).

## Features

- Syntax highlighting for `.bgql` files
- Language Server Protocol support via `bgql-lsp`
- Code completion
- Go to definition
- Hover documentation
- Diagnostics

## Installation

### From Zed Extensions

1. Open Zed
2. Press `Cmd+Shift+X` to open Extensions
3. Search for "Better GraphQL"
4. Click Install

### Manual Installation

1. Clone this repository to your Zed extensions directory:
   ```bash
   git clone https://github.com/ubugeeei/bgql ~/.config/zed/extensions/bgql
   ```

2. Restart Zed

## Requirements

- Zed editor
- `bgql-lsp` binary in your PATH (optional, for language server features)

## Language Server

Install the language server:

```bash
cargo install bgql-cli
```

The language server will be automatically detected when you open a `.bgql` file.

## Configuration

Add to your Zed settings (`~/.config/zed/settings.json`):

```json
{
  "languages": {
    "Better GraphQL": {
      "tab_size": 2
    }
  },
  "lsp": {
    "bgql": {
      "binary": {
        "path": "bgql",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## License

MIT OR Apache-2.0

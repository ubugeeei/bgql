/**
 * GraphQL Playground/GraphiQL integration.
 *
 * Provides an interactive IDE for exploring and testing GraphQL APIs.
 */

export interface PlaygroundConfig {
  /**
   * GraphQL endpoint URL.
   * @default '/graphql'
   */
  readonly endpoint?: string;

  /**
   * WebSocket endpoint for subscriptions.
   */
  readonly subscriptionEndpoint?: string;

  /**
   * Default headers to include in requests.
   */
  readonly headers?: Record<string, string>;

  /**
   * Initial query to show in the editor.
   */
  readonly defaultQuery?: string;

  /**
   * Page title.
   * @default 'bgql Playground'
   */
  readonly title?: string;

  /**
   * Theme: 'light' or 'dark'.
   * @default 'dark'
   */
  readonly theme?: 'light' | 'dark';
}

/**
 * Generates the playground HTML page.
 */
export function generatePlaygroundHTML(config: PlaygroundConfig = {}): string {
  const {
    endpoint = '/graphql',
    subscriptionEndpoint,
    headers = {},
    defaultQuery = '',
    title = 'bgql Playground',
    theme = 'dark',
  } = config;

  const headersJson = JSON.stringify(headers);
  const wsEndpoint = subscriptionEndpoint
    ? `'${subscriptionEndpoint}'`
    : 'null';

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${escapeHtml(title)}</title>
  <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🚀</text></svg>">
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.css" />
  <style>
    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }
    html, body, #graphiql {
      height: 100%;
      width: 100%;
    }
    .graphiql-container {
      height: 100vh;
    }
    ${theme === 'dark' ? `
    .graphiql-container {
      --color-base: #1e1e1e;
      --color-primary: #6fbf73;
    }
    ` : ''}
    .bgql-header {
      background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      color: white;
      padding: 8px 16px;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      font-size: 14px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }
    .bgql-header a {
      color: white;
      text-decoration: none;
      opacity: 0.8;
      transition: opacity 0.2s;
    }
    .bgql-header a:hover {
      opacity: 1;
    }
    .bgql-logo {
      font-weight: 600;
      font-size: 16px;
    }
  </style>
</head>
<body>
  <div class="bgql-header">
    <span class="bgql-logo">⚡ bgql Playground</span>
    <a href="https://github.com/ubugeeei/bgql" target="_blank" rel="noopener">GitHub</a>
  </div>
  <div id="graphiql">Loading...</div>

  <script crossorigin src="https://cdn.jsdelivr.net/npm/react@18/umd/react.production.min.js"></script>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/react-dom@18/umd/react-dom.production.min.js"></script>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/graphiql@3/graphiql.min.js"></script>
  <script crossorigin src="https://cdn.jsdelivr.net/npm/@graphiql/plugin-explorer/dist/index.umd.js"></script>

  <script>
    const ENDPOINT = '${endpoint}';
    const WS_ENDPOINT = ${wsEndpoint};
    const DEFAULT_HEADERS = ${headersJson};
    const DEFAULT_QUERY = ${JSON.stringify(defaultQuery || `# Welcome to bgql Playground!
#
# bgql is a better GraphQL - with improved type system,
# errors as values, and modern developer experience.
#
# Start typing your query here, or use the Explorer on the left.

query {

}
`)};

    // Create fetcher with default headers
    const fetcher = GraphiQL.createFetcher({
      url: ENDPOINT,
      headers: DEFAULT_HEADERS,
      wsConnectionParams: async () => DEFAULT_HEADERS,
    });

    // Initialize GraphiQL
    const root = ReactDOM.createRoot(document.getElementById('graphiql'));
    root.render(
      React.createElement(GraphiQL, {
        fetcher: fetcher,
        defaultQuery: DEFAULT_QUERY,
        defaultEditorToolsVisibility: true,
        plugins: [GraphiQLPluginExplorer.explorerPlugin()],
        isHeadersEditorEnabled: true,
        shouldPersistHeaders: true,
      })
    );
  </script>
</body>
</html>`;
}

/**
 * Creates a request handler for the playground.
 */
export function createPlaygroundHandler(config: PlaygroundConfig = {}): (req: Request) => Response {
  const html = generatePlaygroundHTML(config);

  return (_req: Request) => {
    return new Response(html, {
      headers: {
        'Content-Type': 'text/html; charset=utf-8',
        'Cache-Control': 'no-cache',
      },
    });
  };
}

/**
 * Express/Connect-style middleware for the playground.
 */
export function playgroundMiddleware(config: PlaygroundConfig = {}) {
  const html = generatePlaygroundHTML(config);

  return (_req: any, res: any, next?: () => void) => {
    if (res.setHeader) {
      // Express-style
      res.setHeader('Content-Type', 'text/html; charset=utf-8');
      res.send(html);
    } else if (typeof res === 'function') {
      // Some other middleware format
      next?.();
    } else {
      next?.();
    }
  };
}

/**
 * Checks if a request accepts HTML (for playground).
 */
export function acceptsHTML(headers: Headers): boolean {
  const accept = headers.get('Accept') || '';
  return accept.includes('text/html');
}

/**
 * Helper to escape HTML special characters.
 */
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

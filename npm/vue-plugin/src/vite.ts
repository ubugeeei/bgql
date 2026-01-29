/**
 * Vite plugin for Vue SFC GraphQL support.
 *
 * Uses Vite Environment API (Vite 6+) for proper SSR/client handling.
 * Supports Server Side Fragments for streaming SSR.
 *
 * @example vite.config.ts
 * ```ts
 * import { defineConfig } from 'vite';
 * import vue from '@vitejs/plugin-vue';
 * import { bgqlVuePlugin } from '@bgql/vue-plugin/vite';
 *
 * export default defineConfig({
 *   plugins: [
 *     bgqlVuePlugin({
 *       // Optional: path to generated types
 *       typesFrom: './src/generated/bgql.ts',
 *       // Enable Server Side Fragments
 *       serverSideFragments: true,
 *     }),
 *     vue(),
 *   ],
 * });
 * ```
 */

import type { Plugin } from "vite";
import { parseGqlBlock, generateTsCode, type ParsedOperation } from "./index";

export interface BgqlVuePluginOptions {
  /**
   * Path to generated GraphQL types.
   * If provided, operations will be typed with the generated types.
   */
  typesFrom?: string;

  /**
   * File extensions to process.
   * @default ['.vue']
   */
  include?: string[];

  /**
   * Custom block tag name.
   * @default 'gql'
   */
  blockTag?: string;

  /**
   * Enable Server Side Fragments support.
   * When enabled, fragments can be streamed during SSR.
   * @default false
   */
  serverSideFragments?: boolean;

  /**
   * Environment-specific configuration.
   */
  environments?: {
    client?: boolean;
    ssr?: boolean;
  };
}

/**
 * Server Side Fragment marker for streaming SSR.
 */
export interface ServerSideFragment {
  /**
   * Fragment identifier for hydration matching.
   */
  id: string;

  /**
   * GraphQL operation associated with this fragment.
   */
  operation: string;

  /**
   * Whether this fragment should be streamed.
   */
  streaming: boolean;
}

/**
 * Vite plugin for Vue SFC `<script lang="gql">` support.
 *
 * Uses Vite Environment API for proper SSR handling.
 */
export function bgqlVuePlugin(options: BgqlVuePluginOptions = {}): Plugin {
  const {
    typesFrom,
    include = [".vue"],
    blockTag = "gql",
    serverSideFragments = false,
    environments = { client: true, ssr: true },
  } = options;

  const gqlBlockRegex = new RegExp(
    `<script\\s+lang=["']${blockTag}["'][^>]*>([\\s\\S]*?)<\\/script>`,
    "g"
  );

  const virtualModulePrefix = "\0bgql-virtual:";
  const ssrFragmentPrefix = "\0bgql-ssr-fragment:";

  // Store parsed operations per file
  const operationsCache = new Map<string, ParsedOperation[]>();

  // Track SSR fragment state
  const ssrFragments = new Map<string, ServerSideFragment[]>();

  return {
    name: "bgql-vue-plugin",
    enforce: "pre",

    // Use Vite Environment API for environment-aware transformation
    applyToEnvironment(environment) {
      if (environment.name === "client" && !environments.client) {
        return false;
      }
      if (environment.name === "ssr" && !environments.ssr) {
        return false;
      }
      return true;
    },

    resolveId(id, _importer, _options) {
      // Handle ?gql imports
      if (id.includes("?gql")) {
        const basePath = id.replace("?gql", "");
        return virtualModulePrefix + basePath;
      }

      // Handle SSR fragment imports
      if (serverSideFragments && id.includes("?ssr-fragment")) {
        const basePath = id.replace("?ssr-fragment", "");
        return ssrFragmentPrefix + basePath;
      }

      return null;
    },

    async load(id) {
      // Load virtual gql module
      if (id.startsWith(virtualModulePrefix)) {
        const basePath = id.slice(virtualModulePrefix.length);
        const operations = operationsCache.get(basePath);

        if (operations && operations.length > 0) {
          return generateTsCode(operations, typesFrom);
        }

        return "// No gql block found";
      }

      // Load SSR fragment module
      if (id.startsWith(ssrFragmentPrefix)) {
        const basePath = id.slice(ssrFragmentPrefix.length);
        const fragments = ssrFragments.get(basePath);

        if (fragments && fragments.length > 0) {
          return generateSsrFragmentCode(fragments);
        }

        return "export const fragments = [];";
      }

      return null;
    },

    transform(code, id) {
      // Only process Vue files
      if (!include.some((ext) => id.endsWith(ext))) {
        return null;
      }

      // Check for gql block
      if (
        !code.includes(`lang="${blockTag}"`) &&
        !code.includes(`lang='${blockTag}'`)
      ) {
        return null;
      }

      // Extract operations
      const match = gqlBlockRegex.exec(code);
      gqlBlockRegex.lastIndex = 0;

      if (!match) {
        return null;
      }

      const gqlSource = match[1];
      const operations = parseGqlBlock(gqlSource);

      if (operations.length === 0) {
        return null;
      }

      // Cache operations
      operationsCache.set(id, operations);

      // Create SSR fragments if enabled
      if (serverSideFragments) {
        const fragments = operations.map((op, idx) => ({
          id: `${getFileName(id)}_${op.name}_${idx}`,
          operation: op.source,
          streaming: true,
        }));
        ssrFragments.set(id, fragments);
      }

      // Remove the gql script block from the Vue file
      let cleanedCode = code.replace(gqlBlockRegex, "<!-- gql block extracted -->");

      // Add auto-import for IDE support
      const hasSetupScript = /<script[^>]*setup[^>]*>/.test(cleanedCode);
      const fileName = getFileName(id);

      if (hasSetupScript) {
        const importComment = `// GraphQL operations: import { ${operations.map((o) => o.name).join(", ")} } from './${fileName}?gql';`;

        cleanedCode = cleanedCode.replace(
          /(<script[^>]*setup[^>]*>)/,
          `$1\n${importComment}\n`
        );

        // Add SSR fragment import if enabled
        if (serverSideFragments) {
          const ssrImportComment = `// SSR fragments: import { fragments } from './${fileName}?ssr-fragment';`;
          cleanedCode = cleanedCode.replace(
            /(<script[^>]*setup[^>]*>)/,
            `$1\n${ssrImportComment}\n`
          );
        }
      }

      return {
        code: cleanedCode,
        map: null,
      };
    },

    // Environment-specific hooks
    hotUpdate({ file, modules }) {
      if (include.some((ext) => file.endsWith(ext))) {
        // Clear cache on file change
        operationsCache.delete(file);
        ssrFragments.delete(file);

        // Invalidate virtual modules
        return modules.filter(
          (m) =>
            m.id?.startsWith(virtualModulePrefix) ||
            m.id?.startsWith(ssrFragmentPrefix)
        );
      }
      return;
    },
  };
}

/**
 * Generate code for SSR fragments.
 */
function generateSsrFragmentCode(fragments: ServerSideFragment[]): string {
  const fragmentsJson = JSON.stringify(fragments, null, 2);

  return `
/**
 * Server Side Fragments for streaming SSR.
 * Auto-generated from <script lang="gql"> block.
 */

export const fragments = ${fragmentsJson};

/**
 * Get fragment by operation name.
 */
export function getFragment(operationName) {
  return fragments.find(f => f.operation.includes(\`\${operationName}\`));
}

/**
 * Check if fragment should be streamed.
 */
export function shouldStream(fragmentId) {
  const fragment = fragments.find(f => f.id === fragmentId);
  return fragment?.streaming ?? false;
}

/**
 * Create streaming marker for SSR.
 */
export function createStreamingMarker(fragmentId) {
  return \`<!--ssr-fragment:\${fragmentId}-->\`;
}
`;
}

function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1];
}

// Export plugin as default for convenience
export default bgqlVuePlugin;

/**
 * Helper to create environment-specific plugin instances.
 */
export function createEnvironmentPlugin(
  options: BgqlVuePluginOptions = {}
): { client: Plugin; ssr: Plugin } {
  return {
    client: bgqlVuePlugin({
      ...options,
      environments: { client: true, ssr: false },
    }),
    ssr: bgqlVuePlugin({
      ...options,
      environments: { client: false, ssr: true },
    }),
  };
}

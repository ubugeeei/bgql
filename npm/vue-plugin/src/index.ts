/**
 * Vue SFC plugin for Better GraphQL.
 *
 * Provides support for `<script lang="gql">` blocks in Vue Single File Components.
 *
 * @example Vue SFC usage
 * ```vue
 * <script lang="gql">
 * query GetUser($id: ID!) {
 *   user(id: $id) {
 *     id
 *     name
 *     email
 *   }
 * }
 *
 * mutation UpdateUser($id: ID!, $input: UpdateUserInput!) {
 *   updateUser(id: $id, input: $input) {
 *     id
 *     name
 *   }
 * }
 * </script>
 *
 * <script setup lang="ts">
 * import { GetUser, UpdateUser } from './MyComponent.vue?gql';
 * import { useQuery, useMutation } from '@bgql/vue';
 *
 * const { data } = useQuery(GetUser, { id: '1' });
 * const { mutate } = useMutation(UpdateUser);
 * </script>
 * ```
 */

import { gql, type TypedDocumentNode } from "@bgql/sdk";

export interface ParsedOperation {
  name: string;
  kind: "query" | "mutation" | "subscription";
  source: string;
}

/**
 * Parse GraphQL operations from a script block.
 */
export function parseGqlBlock(source: string): ParsedOperation[] {
  const operations: ParsedOperation[] = [];

  // Match all operation definitions
  const operationRegex =
    /(query|mutation|subscription)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(\([^)]*\))?\s*\{/g;

  let match: RegExpExecArray | null;
  const positions: { kind: string; name: string; start: number }[] = [];

  while ((match = operationRegex.exec(source)) !== null) {
    positions.push({
      kind: match[1],
      name: match[2],
      start: match.index,
    });
  }

  // Extract each operation
  for (let i = 0; i < positions.length; i++) {
    const pos = positions[i];
    const start = pos.start;
    const end = i < positions.length - 1 ? positions[i + 1].start : source.length;

    // Find the matching closing brace
    let braceCount = 0;
    let operationEnd = start;
    let inOperation = false;

    for (let j = start; j < source.length && j < end + 1000; j++) {
      const char = source[j];
      if (char === "{") {
        braceCount++;
        inOperation = true;
      } else if (char === "}") {
        braceCount--;
        if (inOperation && braceCount === 0) {
          operationEnd = j + 1;
          break;
        }
      }
    }

    const operationSource = source.slice(start, operationEnd).trim();

    operations.push({
      name: pos.name,
      kind: pos.kind as "query" | "mutation" | "subscription",
      source: operationSource,
    });
  }

  return operations;
}

/**
 * Generate TypeScript code from parsed operations.
 */
export function generateTsCode(
  operations: ParsedOperation[],
  typeImportsFrom?: string
): string {
  const imports = ['import { gql } from "@bgql/sdk";'];

  if (typeImportsFrom) {
    const typeImports = operations.map((op) => `${op.name}Variables, ${op.name}Data`).join(", ");
    imports.push(`import type { ${typeImports} } from "${typeImportsFrom}";`);
  }

  const exports = operations.map((op) => {
    const varsType = typeImportsFrom ? `${op.name}Variables` : "Record<string, unknown>";
    const dataType = typeImportsFrom ? `${op.name}Data` : "unknown";

    return `export const ${op.name} = gql<${varsType}, ${dataType}>\`
${op.source}
\`;`;
  });

  return [imports.join("\n"), "", ...exports].join("\n");
}

/**
 * Transform a `<script lang="gql">` block into typed operations.
 */
export function transformGqlBlock(
  source: string,
  options?: {
    typeImportsFrom?: string;
  }
): string {
  const operations = parseGqlBlock(source);
  return generateTsCode(operations, options?.typeImportsFrom);
}

/**
 * Create typed document nodes from a gql block source.
 */
export function createOperations(
  source: string
): Record<string, TypedDocumentNode> {
  const operations = parseGqlBlock(source);
  const result: Record<string, TypedDocumentNode> = {};

  for (const op of operations) {
    result[op.name] = gql`${op.source}`;
  }

  return result;
}

// Re-export gql for convenience
export { gql, type TypedDocumentNode } from "@bgql/sdk";

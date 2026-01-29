/**
 * GraphQL template literal tag for type-safe query definitions.
 *
 * Works with codegen-generated types for full type inference.
 *
 * @example Basic usage
 * ```ts
 * import { gql } from '@bgql/sdk';
 *
 * // With codegen-generated types
 * const GetUser = gql<GetUserVariables, GetUserData>`
 *   query GetUser($id: ID!) {
 *     user(id: $id) {
 *       id
 *       name
 *       email
 *     }
 *   }
 * `;
 *
 * // Use with client
 * const result = await client.execute(GetUser, { id: '1' });
 * ```
 *
 * @example With fragments
 * ```ts
 * const UserFragment = gql`
 *   fragment UserFields on User {
 *     id
 *     name
 *   }
 * `;
 *
 * const GetUserWithFragment = gql<GetUserVariables, GetUserData>`
 *   ${UserFragment}
 *   query GetUser($id: ID!) {
 *     user(id: $id) {
 *       ...UserFields
 *     }
 *   }
 * `;
 * ```
 */

import { type TypedOperation, type OperationKind } from "./client";

/**
 * TypedDocumentNode-compatible type for GraphQL documents.
 */
export interface TypedDocumentNode<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
> extends TypedOperation<TVariables, TData> {
  readonly __apiType?: () => TData;
  readonly __variablesType?: () => TVariables;
}

/**
 * Document node that can be embedded in other queries (fragments).
 */
export interface DocumentNode {
  readonly operation: string;
  readonly operationName?: string | undefined;
  readonly kind?: OperationKind | undefined;
}

/**
 * Parse operation name and kind from GraphQL source.
 */
function parseOperationInfo(source: string): {
  operationName: string;
  kind: OperationKind;
} {
  const trimmed = source.trim();

  // Match operation definition: query/mutation/subscription OperationName
  const operationMatch = trimmed.match(
    /^(query|mutation|subscription)\s+([A-Za-z_][A-Za-z0-9_]*)/
  );

  if (operationMatch) {
    const [, kindStr, name] = operationMatch;
    return {
      operationName: name ?? "",
      kind: kindStr as OperationKind,
    };
  }

  // Anonymous query or fragment
  if (trimmed.startsWith("mutation")) {
    return { operationName: "", kind: "mutation" };
  }
  if (trimmed.startsWith("subscription")) {
    return { operationName: "", kind: "subscription" };
  }
  if (trimmed.startsWith("fragment")) {
    // Fragments don't have a kind, treat as query
    const fragmentMatch = trimmed.match(/^fragment\s+([A-Za-z_][A-Za-z0-9_]*)/);
    return {
      operationName: fragmentMatch?.[1] ?? "",
      kind: "query",
    };
  }

  return { operationName: "", kind: "query" };
}

/**
 * Create a typed GraphQL document from a template literal.
 *
 * @example
 * ```ts
 * const GetUser = gql<GetUserVariables, GetUserData>`
 *   query GetUser($id: ID!) {
 *     user(id: $id) { id name }
 *   }
 * `;
 * ```
 */
export function gql<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
>(
  strings: TemplateStringsArray,
  ...values: (DocumentNode | string)[]
): TypedDocumentNode<TVariables, TData> {
  // Build the full query string, interpolating fragments
  let operation = "";

  for (let i = 0; i < strings.length; i++) {
    operation += strings[i];
    if (i < values.length) {
      const value = values[i];
      if (typeof value === "string") {
        operation += value;
      } else if (value && typeof value === "object" && "operation" in value) {
        // It's a DocumentNode (fragment)
        operation += value.operation;
      }
    }
  }

  const { operationName, kind } = parseOperationInfo(operation);

  return {
    operation,
    operationName,
    kind,
  } as TypedDocumentNode<TVariables, TData>;
}

/**
 * Type helper to extract variables type from a document node.
 */
export type VariablesOf<T> = T extends TypedDocumentNode<infer V, unknown>
  ? V
  : never;

/**
 * Type helper to extract data type from a document node.
 */
export type DataOf<T> = T extends TypedDocumentNode<Record<string, unknown>, infer D>
  ? D
  : never;

/**
 * Create a fragment document node.
 *
 * @example
 * ```ts
 * const UserFields = fragment`
 *   fragment UserFields on User {
 *     id
 *     name
 *   }
 * `;
 * ```
 */
export function fragment(
  strings: TemplateStringsArray,
  ...values: (DocumentNode | string)[]
): DocumentNode {
  let operation = "";

  for (let i = 0; i < strings.length; i++) {
    operation += strings[i];
    if (i < values.length) {
      const value = values[i];
      if (typeof value === "string") {
        operation += value;
      } else if (value && typeof value === "object" && "operation" in value) {
        operation += value.operation;
      }
    }
  }

  const fragmentMatch = operation
    .trim()
    .match(/^fragment\s+([A-Za-z_][A-Za-z0-9_]*)/);

  return {
    operation,
    operationName: fragmentMatch?.[1] ?? "",
    kind: undefined,
  };
}

/**
 * Type-only marker for use with codegen.
 *
 * This is used by codegen tools to generate proper types.
 * At runtime, it's just the gql function.
 *
 * @example Codegen usage
 * ```ts
 * // In your schema codegen output:
 * export const GetUserDocument = graphql(`
 *   query GetUser($id: ID!) { user(id: $id) { id name } }
 * `);
 *
 * // The type is inferred from codegen output
 * ```
 */
export const graphql = gql;

/**
 * Type-safe document builder for better IDE support.
 *
 * @example
 * ```ts
 * const doc = document<GetUserVariables, GetUserData>()
 *   .query('GetUser')
 *   .source(`
 *     query GetUser($id: ID!) {
 *       user(id: $id) { id name }
 *     }
 *   `);
 * ```
 */
export function document<
  TVariables extends Record<string, unknown> = Record<string, unknown>,
  TData = unknown
>() {
  return {
    query(operationName: string) {
      return {
        source(operation: string): TypedDocumentNode<TVariables, TData> {
          return {
            operation,
            operationName,
            kind: "query" as const,
          };
        },
      };
    },
    mutation(operationName: string) {
      return {
        source(operation: string): TypedDocumentNode<TVariables, TData> {
          return {
            operation,
            operationName,
            kind: "mutation" as const,
          };
        },
      };
    },
    subscription(operationName: string) {
      return {
        source(operation: string): TypedDocumentNode<TVariables, TData> {
          return {
            operation,
            operationName,
            kind: "subscription" as const,
          };
        },
      };
    },
  };
}

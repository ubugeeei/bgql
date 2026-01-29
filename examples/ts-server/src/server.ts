/**
 * BGQL Example TypeScript Server
 *
 * Schema-first development with layered architecture.
 *
 * Build: bun run build
 * Dev:   bun run dev
 */

import {
  createServer,
  createBaseContext,
  mergeResolvers,
} from "@bgql/server";
import type { IncomingRequest } from "@bgql/server";
import {
  createDatabase,
  createContext,
  type Context,
} from "./presentation/context.js";
import { resolvers } from "./presentation/resolvers.js";
import { $readSchema } from "./macros/schema.ts" with { type: "macro" };

const PORT = process.env.PORT ? parseInt(process.env.PORT, 10) : 4000;

// Schema is inlined at build time via macro
const schema = $readSchema("dist/schema.graphql");

// ============================================
// Server Setup
// ============================================

const db = createDatabase();

const server = createServer<Context>({
  schema,
  resolvers: mergeResolvers<Context>(
    resolvers,
    {
      UserResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "User",
      },
      PostResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "Post",
      },
      CreateUserResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "User",
      },
      UpdateUserResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "User",
      },
      DeleteUserResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "DeleteSuccess",
      },
      CreatePostResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "Post",
      },
      UpdatePostResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "Post",
      },
      DeletePostResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "DeleteSuccess",
      },
      PublishPostResult: {
        __resolveType: (obj: { __typename?: string }) => obj.__typename ?? "Post",
      },
    }
  ),
  context: async (req: IncomingRequest): Promise<Context> => {
    const baseContext = createBaseContext(req);
    return createContext({
      db,
      currentUser: null,
      baseContext,
    });
  },
  options: {
    port: PORT,
    playground: true,
    introspection: true,
  },
});

// ============================================
// Start Server
// ============================================

server.listen().then((info) => {
  console.log(`
=====================================
  BGQL TypeScript Server
=====================================

Server: ${info.url}
GraphQL: ${info.url}/graphql
`);
});

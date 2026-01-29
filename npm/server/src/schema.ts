/**
 * Schema Loading
 *
 * Functions for loading and compiling bgql schemas.
 */

import { execSync } from "node:child_process";
import { readFileSync, existsSync } from "node:fs";
import { dirname, join } from "node:path";

/**
 * Load and compile a bgql schema file to GraphQL SDL.
 *
 * This function calls the bgql CLI to compile the schema.
 * The bgql CLI must be installed and available in PATH.
 *
 * @param schemaPath - Path to the .bgql schema file (usually mod.bgql)
 * @returns Compiled GraphQL SDL string
 * @throws Error if bgql CLI is not available or compilation fails
 *
 * @example
 * ```typescript
 * import { loadSchema } from '@bgql/server';
 *
 * const schema = loadSchema('./schema/mod.bgql');
 * const server = createServer({
 *   schema,
 *   resolvers,
 * });
 * ```
 */
export function loadSchema(schemaPath: string): string {
  if (!existsSync(schemaPath)) {
    throw new Error(`Schema file not found: ${schemaPath}`);
  }

  try {
    // Call bgql CLI to compile the schema
    // bgql build outputs compiled GraphQL SDL to stdout
    const result = execSync(`bgql build "${schemaPath}" --stdout`, {
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    });

    return result.trim();
  } catch (error) {
    // Check if bgql is not installed
    if (error instanceof Error && error.message.includes("ENOENT")) {
      throw new Error(
        "bgql CLI not found. Install it with: cargo install bgql"
      );
    }

    // Check for compilation errors
    if (error instanceof Error && "stderr" in error) {
      const stderr = (error as { stderr: string }).stderr;
      throw new Error(`Failed to compile schema: ${stderr}`);
    }

    throw error;
  }
}

/**
 * Load a pre-compiled GraphQL schema from a file.
 *
 * Use this for production deployments where the schema
 * has been pre-compiled via `bgql build`.
 *
 * @param schemaPath - Path to the compiled .graphql file
 * @returns GraphQL SDL string
 *
 * @example
 * ```typescript
 * import { loadCompiledSchema } from '@bgql/server';
 *
 * const schema = loadCompiledSchema('./dist/schema.graphql');
 * ```
 */
export function loadCompiledSchema(schemaPath: string): string {
  if (!existsSync(schemaPath)) {
    throw new Error(`Compiled schema not found: ${schemaPath}`);
  }

  return readFileSync(schemaPath, "utf-8");
}

/**
 * Load schema with fallback to pre-compiled.
 *
 * This function tries to:
 * 1. Load pre-compiled schema from dist/schema.graphql
 * 2. Compile .bgql schema on-the-fly (development only)
 *
 * @param bgqlPath - Path to the .bgql source file
 * @param compiledPath - Path to the pre-compiled .graphql file
 * @returns GraphQL SDL string
 *
 * @example
 * ```typescript
 * import { loadSchemaWithFallback } from '@bgql/server';
 *
 * const schema = loadSchemaWithFallback(
 *   './schema/mod.bgql',
 *   './dist/schema.graphql'
 * );
 * ```
 */
export function loadSchemaWithFallback(
  bgqlPath: string,
  compiledPath: string
): string {
  // In production, always use pre-compiled schema
  if (process.env.NODE_ENV === "production") {
    if (!existsSync(compiledPath)) {
      throw new Error(
        `Production requires pre-compiled schema at: ${compiledPath}\n` +
          `Run: bgql build ${bgqlPath} -o ${compiledPath}`
      );
    }
    return loadCompiledSchema(compiledPath);
  }

  // In development, prefer pre-compiled but fall back to on-the-fly compilation
  if (existsSync(compiledPath)) {
    console.log(`[bgql] Loading pre-compiled schema from ${compiledPath}`);
    return loadCompiledSchema(compiledPath);
  }

  if (existsSync(bgqlPath)) {
    console.log(`[bgql] Compiling schema from ${bgqlPath}`);
    return loadSchema(bgqlPath);
  }

  throw new Error(
    `No schema found. Expected either:\n` +
      `  - Pre-compiled: ${compiledPath}\n` +
      `  - Source: ${bgqlPath}`
  );
}

/**
 * Build-time macros for schema loading.
 *
 * These macros are evaluated at build time by unplugin-macros,
 * inlining the schema content directly into the bundle.
 */

import { readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { execSync } from "node:child_process";

/**
 * Reads and compiles a .bgql schema at build time.
 *
 * The bgql CLI is called during the build process,
 * and the compiled GraphQL SDL is inlined into the bundle.
 *
 * @param schemaPath - Path to the .bgql schema file (relative to project root)
 * @returns Compiled GraphQL SDL string
 */
export function $loadSchema(schemaPath: string): string {
  const absolutePath = join(process.cwd(), schemaPath);

  try {
    // Call bgql CLI to compile the schema at build time
    const result = execSync(`bgql build "${absolutePath}" --stdout`, {
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    });

    return result.trim();
  } catch (error) {
    throw new Error(`Failed to compile schema: ${schemaPath}`);
  }
}

/**
 * Reads a pre-compiled GraphQL schema at build time.
 *
 * @param schemaPath - Path to the .graphql file (relative to project root)
 * @returns GraphQL SDL string
 */
export function $readSchema(schemaPath: string): string {
  const absolutePath = join(process.cwd(), schemaPath);
  return readFileSync(absolutePath, "utf-8");
}

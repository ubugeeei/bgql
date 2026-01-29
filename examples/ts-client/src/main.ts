/**
 * BGQL Client Example
 *
 * Demonstrates type-safe GraphQL client usage with:
 * - Errors as values (no try-catch needed for business errors)
 * - Full type safety from schema
 * - Exhaustive error handling
 */

import { createClient } from "./client.js";
import { UserId, PostId } from "./types.js";

const ENDPOINT = process.env.BGQL_ENDPOINT ?? "http://localhost:4000/graphql";

async function main() {
  console.log(`
=====================================
  BGQL TypeScript Client Example
=====================================

Connecting to: ${ENDPOINT}
`);

  const client = createClient({ endpoint: ENDPOINT });

  // ============================================
  // Example 1: Get User (Typed Error Handling)
  // ============================================
  console.log("--- Example 1: Get User ---\n");

  const userResult = await client.getUser(UserId("user_1"));

  // Type-safe switch on __typename
  // TypeScript ensures all cases are handled
  switch (userResult.__typename) {
    case "User":
      console.log(`Found user: ${userResult.name} (${userResult.email})`);
      console.log(`  Role: ${userResult.role}`);
      console.log(`  Bio: ${userResult.bio ?? "(none)"}`);
      break;

    case "NotFoundError":
      console.log(`User not found: ${userResult.message}`);
      console.log(`  Resource ID: ${userResult.resourceId}`);
      break;

    case "UnauthorizedError":
      console.log(`Unauthorized: ${userResult.message}`);
      break;
  }

  // ============================================
  // Example 2: Get Non-existent User
  // ============================================
  console.log("\n--- Example 2: Get Non-existent User ---\n");

  const notFoundResult = await client.getUser(UserId("nonexistent"));

  // No try-catch needed - error is a value
  if (notFoundResult.__typename === "NotFoundError") {
    console.log(`Expected error: ${notFoundResult.message}`);
  }

  // ============================================
  // Example 3: List Users with Pagination
  // ============================================
  console.log("\n--- Example 3: List Users ---\n");

  const usersConnection = await client.listUsers({ first: 5 });

  console.log(`Total users: ${usersConnection.totalCount}`);
  console.log(`Has next page: ${usersConnection.pageInfo.hasNextPage}`);
  console.log("\nUsers:");
  for (const edge of usersConnection.edges) {
    console.log(`  - ${edge.node.name} (${edge.node.role})`);
  }

  // ============================================
  // Example 4: Create User with Validation
  // ============================================
  console.log("\n--- Example 4: Create User (Valid) ---\n");

  const createResult = await client.createUser({
    name: "Test User",
    email: "test@example.com",
    password: "securepassword123",
    bio: "A test user created by the example client",
  });

  if (createResult.__typename === "User") {
    console.log(`Created user: ${createResult.name} (ID: ${createResult.id})`);
  } else {
    // ValidationError
    console.log(`Validation failed: ${createResult.message}`);
    console.log(`  Field: ${createResult.field}`);
  }

  // ============================================
  // Example 5: Create User with Invalid Data
  // ============================================
  console.log("\n--- Example 5: Create User (Invalid) ---\n");

  const invalidResult = await client.createUser({
    name: "AB", // Too short (min 3)
    email: "not-an-email", // Invalid format
    password: "short", // Too short (min 8)
  });

  // Validation error is a normal return value, not an exception
  if (invalidResult.__typename === "ValidationError") {
    console.log(`Validation error (expected): ${invalidResult.message}`);
    console.log(`  Field: ${invalidResult.field}`);
    console.log(`  Constraint: ${invalidResult.constraint}`);
  }

  // ============================================
  // Example 6: Login Flow
  // ============================================
  console.log("\n--- Example 6: Login Flow ---\n");

  const loginResult = await client.login({
    email: "alice@example.com",
    password: "password123",
  });

  switch (loginResult.__typename) {
    case "AuthPayload":
      console.log(`Login successful!`);
      console.log(`  User: ${loginResult.user.name}`);
      console.log(`  Token: ${loginResult.token.slice(0, 20)}...`);
      console.log(`  Expires: ${loginResult.expiresAt}`);

      // Set token for authenticated requests
      client.setToken(loginResult.token);
      break;

    case "InvalidCredentialsError":
      console.log(`Login failed: ${loginResult.message}`);
      break;

    case "ValidationError":
      console.log(`Validation error: ${loginResult.message}`);
      break;
  }

  // ============================================
  // Example 7: Authenticated Request
  // ============================================
  console.log("\n--- Example 7: Authenticated Request ---\n");

  const me = await client.me();
  if (me) {
    console.log(`Current user: ${me.name} (${me.email})`);
  } else {
    console.log("Not authenticated");
  }

  // ============================================
  // Example 8: Create Post (Requires Auth)
  // ============================================
  console.log("\n--- Example 8: Create Post ---\n");

  const postResult = await client.createPost({
    title: "My First BGQL Post",
    content: "This is a post created using the BGQL TypeScript client.",
    status: "Draft",
  });

  switch (postResult.__typename) {
    case "Post":
      console.log(`Created post: "${postResult.title}"`);
      console.log(`  ID: ${postResult.id}`);
      console.log(`  Status: ${postResult.status}`);
      break;

    case "ValidationError":
      console.log(`Validation error: ${postResult.message}`);
      break;

    case "UnauthorizedError":
      console.log(`Unauthorized: ${postResult.message}`);
      break;
  }

  // ============================================
  // Example 9: List Posts
  // ============================================
  console.log("\n--- Example 9: List Posts ---\n");

  const postsConnection = await client.listPosts({ first: 5 });

  console.log(`Total posts: ${postsConnection.totalCount}`);
  console.log("\nPosts:");
  for (const edge of postsConnection.edges) {
    console.log(`  - [${edge.node.status}] ${edge.node.title}`);
  }

  // ============================================
  // Example 10: AbortController for Cancellation
  // ============================================
  console.log("\n--- Example 10: Request Cancellation ---\n");

  const controller = new AbortController();

  // Cancel after 10ms (will likely cancel the request)
  setTimeout(() => controller.abort(), 10);

  try {
    await client.listUsers({ first: 100 }, controller.signal);
    console.log("Request completed before cancellation");
  } catch (error) {
    if (error instanceof Error && error.name === "AbortError") {
      console.log("Request was cancelled (as expected)");
    } else {
      throw error;
    }
  }

  console.log(`
=====================================
  Examples Complete
=====================================
`);
}

// Run examples
main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});

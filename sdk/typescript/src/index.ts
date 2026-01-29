/**
 * Better GraphQL TypeScript SDK
 *
 * Strongly typed GraphQL client and server utilities.
 *
 * @example Client usage
 * ```ts
 * import { createClient, defineQuery } from '@bgql/sdk';
 *
 * interface GetUserVars { id: string }
 * interface GetUserData { user: { id: string; name: string } | null }
 *
 * const GetUser = defineQuery<GetUserVars, GetUserData>(
 *   'GetUser',
 *   'query GetUser($id: ID!) { user(id: $id) { id name } }'
 * );
 *
 * const client = createClient({ url: 'http://localhost:4000/bgql' });
 * const result = await client.execute(GetUser, { id: '1' });
 *
 * if (result._tag === 'Ok') {
 *   console.log(result.value.user?.name);
 * }
 * ```
 *
 * @example Server usage
 * ```ts
 * import { contextKey, buildContext, resolver } from '@bgql/sdk/server';
 *
 * const CurrentUser = contextKey<{ id: string; name: string }>('CurrentUser');
 *
 * const ctx = buildContext()
 *   .with(CurrentUser, { id: '1', name: 'Alice' })
 *   .build();
 *
 * const userResolver = resolver<{}, { id: string }, typeof ctx, User>(
 *   async (parent, args, context) => {
 *     const currentUser = context.require(CurrentUser);
 *     return { id: args.id, name: 'User' };
 *   }
 * );
 * ```
 */

// Error types
export {
  ErrorCode,
  type SdkError,
  sdkError,
  SdkError as SdkErrorFactory,
  isSdkError,
  isRetryable,
  isClientError,
  isServerError,
  type GraphQLError,
  isGraphQLError,
} from "./error";

// Result types
export {
  type Result,
  type Ok,
  type Err,
  type AsyncResult,
  ok,
  err,
  isOk,
  isErr,
  map,
  mapErr,
  flatMap,
  unwrap,
  unwrapOr,
  unwrapOrElse,
  match,
  fromPromise,
  toPromise,
  all,
} from "./result";

// Context types
export {
  TypedContext,
  type ContextKey,
  contextKey,
  createContext,
  buildContext,
  ContextBuilder,
  CurrentUserId,
  UserRoles,
  RequestId,
  RequestStartTime,
  type RolesHelper,
  createRolesHelper,
} from "./context";

// Client types
export {
  BgqlClient,
  createClient,
  type ClientConfig,
  type RequestOptions,
  type TypedOperation,
  type OperationKind,
  type VariablesOf,
  type DataOf,
  type GraphQLResponse,
  defineOperation,
  defineQuery,
  defineMutation,
} from "./client";

// GraphQL template literal
export {
  gql,
  graphql,
  fragment,
  document,
  type TypedDocumentNode,
  type DocumentNode,
  type VariablesOf as GqlVariablesOf,
  type DataOf as GqlDataOf,
} from "./gql";

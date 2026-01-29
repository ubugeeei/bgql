# Authentication

Better GraphQL provides flexible authentication patterns for securing your API.

## Context-Based Authentication

### Setting Up Auth Context

```typescript
import { serve } from '@bgql/server';
import { verifyToken } from './auth';

const server = await serve({
  schema: './schema.bgql',
  resolvers,
  context: async ({ request }) => {
    const token = request.headers.get('authorization')?.replace('Bearer ', '');

    let user = null;
    if (token) {
      try {
        const payload = await verifyToken(token);
        user = await db.users.findById(payload.userId);
      } catch (e) {
        // Invalid token - user remains null
      }
    }

    return {
      user,
      db,
      isAuthenticated: user !== null,
    };
  },
});
```

### Using Auth in Resolvers

```typescript
const resolvers = {
  Query: {
    me: async (_, __, ctx) => {
      if (!ctx.user) {
        return {
          __typename: 'AuthError',
          message: 'You must be logged in',
          code: 'UNAUTHENTICATED',
        };
      }
      return { __typename: 'User', ...ctx.user };
    },

    users: async (_, __, ctx) => {
      if (!ctx.isAuthenticated) {
        return {
          __typename: 'AuthError',
          message: 'Authentication required',
        };
      }
      return ctx.db.users.findAll();
    },
  },
};
```

## Auth Directive

### Schema Definition

```graphql
directive @auth on FIELD_DEFINITION
directive @requireRole(role: Role!) on FIELD_DEFINITION

enum Role {
  USER
  ADMIN
  SUPER_ADMIN
}

type Query {
  # Public - no auth required
  publicPosts: List<Post>

  # Requires authentication
  me: User | AuthError @auth

  # Requires specific role
  adminDashboard: AdminStats @auth @requireRole(role: ADMIN)
}
```

### Implementing Auth Directive

```typescript
import { createDirective } from '@bgql/server';

const authDirective = createDirective('auth', {
  field: async (resolve, source, args, ctx, info) => {
    if (!ctx.user) {
      return {
        __typename: 'AuthError',
        message: 'You must be logged in to access this resource',
        code: 'UNAUTHENTICATED',
      };
    }
    return resolve(source, args, ctx, info);
  },
});

const requireRoleDirective = createDirective('requireRole', {
  field: async (resolve, source, args, ctx, info, { role }) => {
    if (!ctx.user) {
      return {
        __typename: 'AuthError',
        message: 'Authentication required',
        code: 'UNAUTHENTICATED',
      };
    }

    if (!hasRole(ctx.user, role)) {
      return {
        __typename: 'AuthError',
        message: `This action requires ${role} role`,
        code: 'FORBIDDEN',
        requiredRole: role,
      };
    }

    return resolve(source, args, ctx, info);
  },
});

function hasRole(user: User, requiredRole: string): boolean {
  const roleHierarchy = ['USER', 'ADMIN', 'SUPER_ADMIN'];
  const userRoleIndex = roleHierarchy.indexOf(user.role);
  const requiredRoleIndex = roleHierarchy.indexOf(requiredRole);
  return userRoleIndex >= requiredRoleIndex;
}
```

## Authentication Strategies

### JWT Authentication

```typescript
import jwt from 'jsonwebtoken';

interface JWTPayload {
  userId: string;
  role: string;
  exp: number;
}

async function verifyToken(token: string): Promise<JWTPayload> {
  return jwt.verify(token, process.env.JWT_SECRET!) as JWTPayload;
}

function createToken(user: User): string {
  return jwt.sign(
    { userId: user.id, role: user.role },
    process.env.JWT_SECRET!,
    { expiresIn: '7d' }
  );
}

// In context
context: async ({ request }) => {
  const token = request.headers.get('authorization')?.replace('Bearer ', '');

  if (token) {
    try {
      const payload = await verifyToken(token);
      const user = await db.users.findById(payload.userId);
      return { user, isAuthenticated: true };
    } catch (e) {
      // Token expired or invalid
    }
  }

  return { user: null, isAuthenticated: false };
}
```

### Session-Based Authentication

```typescript
import { getSession } from './sessions';

context: async ({ request }) => {
  const sessionId = request.cookies.get('session_id');

  if (sessionId) {
    const session = await getSession(sessionId);
    if (session && !session.expired) {
      const user = await db.users.findById(session.userId);
      return { user, session, isAuthenticated: true };
    }
  }

  return { user: null, session: null, isAuthenticated: false };
}
```

### API Key Authentication

```typescript
context: async ({ request }) => {
  const apiKey = request.headers.get('x-api-key');

  if (apiKey) {
    const key = await db.apiKeys.findByKey(apiKey);
    if (key && key.active) {
      const user = await db.users.findById(key.userId);
      return {
        user,
        apiKey: key,
        isAuthenticated: true,
        authMethod: 'api_key',
      };
    }
  }

  return { user: null, isAuthenticated: false };
}
```

## Login/Logout Mutations

### Schema

```graphql
input LoginInput {
  email: String @email
  password: String @minLength(8)
}

type AuthPayload {
  token: String
  user: User
  expiresAt: DateTime
}

type InvalidCredentialsError {
  message: String
}

union LoginResult = AuthPayload | InvalidCredentialsError | ValidationError

type Mutation {
  login(input: LoginInput): LoginResult
  logout: Boolean @auth
  refreshToken: AuthPayload | AuthError @auth
}
```

### Implementation

```typescript
import { hash, compare } from 'bcrypt';

const resolvers = {
  Mutation: {
    login: async (_, { input }, ctx) => {
      const { email, password } = input;

      // Find user
      const user = await ctx.db.users.findByEmail(email);
      if (!user) {
        return {
          __typename: 'InvalidCredentialsError',
          message: 'Invalid email or password',
        };
      }

      // Verify password
      const valid = await compare(password, user.passwordHash);
      if (!valid) {
        return {
          __typename: 'InvalidCredentialsError',
          message: 'Invalid email or password',
        };
      }

      // Create token
      const token = createToken(user);
      const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);

      return {
        __typename: 'AuthPayload',
        token,
        user,
        expiresAt: expiresAt.toISOString(),
      };
    },

    logout: async (_, __, ctx) => {
      if (ctx.session) {
        await ctx.db.sessions.delete(ctx.session.id);
      }
      return true;
    },

    refreshToken: async (_, __, ctx) => {
      if (!ctx.user) {
        return {
          __typename: 'AuthError',
          message: 'Not authenticated',
        };
      }

      const token = createToken(ctx.user);
      const expiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);

      return {
        __typename: 'AuthPayload',
        token,
        user: ctx.user,
        expiresAt: expiresAt.toISOString(),
      };
    },
  },
};
```

## Authorization

### Field-Level Authorization

```graphql
type User {
  id: ID
  name: String
  email: String @auth  # Only visible to authenticated users
  privateNotes: String @auth @requireRole(role: ADMIN)
}
```

```typescript
const resolvers = {
  User: {
    email: async (user, _, ctx) => {
      // Only show email if viewing own profile or admin
      if (ctx.user?.id === user.id || ctx.user?.role === 'ADMIN') {
        return user.email;
      }
      return null;
    },

    privateNotes: async (user, _, ctx) => {
      if (ctx.user?.role !== 'ADMIN') {
        return null;
      }
      return user.privateNotes;
    },
  },
};
```

### Resource-Based Authorization

```typescript
const resolvers = {
  Mutation: {
    updatePost: async (_, { id, input }, ctx) => {
      const post = await ctx.db.posts.findById(id);

      if (!post) {
        return { __typename: 'NotFoundError', message: 'Post not found' };
      }

      // Check ownership
      if (post.authorId !== ctx.user?.id && ctx.user?.role !== 'ADMIN') {
        return {
          __typename: 'AuthError',
          message: 'You can only edit your own posts',
        };
      }

      const updated = await ctx.db.posts.update(id, input);
      return { __typename: 'Post', ...updated };
    },
  },
};
```

### Permission System

```typescript
// Define permissions
type Permission =
  | 'posts:read'
  | 'posts:write'
  | 'posts:delete'
  | 'users:read'
  | 'users:write'
  | 'admin:access';

const rolePermissions: Record<string, Permission[]> = {
  USER: ['posts:read', 'posts:write'],
  MODERATOR: ['posts:read', 'posts:write', 'posts:delete', 'users:read'],
  ADMIN: ['posts:read', 'posts:write', 'posts:delete', 'users:read', 'users:write', 'admin:access'],
};

function hasPermission(user: User | null, permission: Permission): boolean {
  if (!user) return false;
  return rolePermissions[user.role]?.includes(permission) ?? false;
}

// Use in resolvers
const resolvers = {
  Mutation: {
    deletePost: async (_, { id }, ctx) => {
      if (!hasPermission(ctx.user, 'posts:delete')) {
        return { __typename: 'AuthError', message: 'Insufficient permissions' };
      }
      // ...
    },
  },
};
```

## Client Integration

### Setting Auth Headers

```typescript
import { createClient } from '@bgql/client';

const client = createClient('http://localhost:4000/graphql', {
  headers: () => {
    const token = localStorage.getItem('token');
    return token ? { Authorization: `Bearer ${token}` } : {};
  },
});
```

### Login Flow

```typescript
async function login(email: string, password: string) {
  const result = await client.mutate(LoginDocument, {
    input: { email, password },
  });

  if (result.ok) {
    matchUnion(result.value.login, {
      AuthPayload: (payload) => {
        localStorage.setItem('token', payload.token);
        setUser(payload.user);
        router.push('/dashboard');
      },
      InvalidCredentialsError: (error) => {
        showError(error.message);
      },
      ValidationError: (error) => {
        setFieldError(error.field, error.message);
      },
    });
  }
}
```

### Auth State Management

```typescript
// Vue composition
import { ref, computed } from 'vue';

const user = ref<User | null>(null);
const token = ref<string | null>(localStorage.getItem('token'));

const isAuthenticated = computed(() => !!user.value);

async function initAuth() {
  if (token.value) {
    const result = await client.query(MeDocument);
    if (result.ok && result.value.me.__typename === 'User') {
      user.value = result.value.me;
    } else {
      // Token invalid, clear it
      localStorage.removeItem('token');
      token.value = null;
    }
  }
}

function logout() {
  localStorage.removeItem('token');
  token.value = null;
  user.value = null;
  router.push('/login');
}
```

## Best Practices

### 1. Always Return Typed Errors

```graphql
# Good: Typed auth errors
union UserResult = User | AuthError | NotFoundError

# Avoid: Throwing exceptions for auth failures
```

### 2. Use Middleware for Common Auth

```typescript
// Auth middleware
const authMiddleware = createMiddleware(async (resolve, root, args, ctx, info) => {
  // Check if field requires auth
  const requiresAuth = info.fieldNodes[0].directives?.some(
    d => d.name.value === 'auth'
  );

  if (requiresAuth && !ctx.user) {
    return {
      __typename: 'AuthError',
      message: 'Authentication required',
    };
  }

  return resolve(root, args, ctx, info);
});
```

### 3. Secure Token Storage

```typescript
// Use httpOnly cookies for sensitive apps
const server = await serve({
  // ...
  context: async ({ request, response }) => {
    // Set secure cookie on login
    if (shouldSetCookie) {
      response.headers.set(
        'Set-Cookie',
        `token=${token}; HttpOnly; Secure; SameSite=Strict; Max-Age=604800`
      );
    }
    // ...
  },
});
```

### 4. Rate Limit Auth Endpoints

```typescript
import { rateLimit } from './middleware';

const resolvers = {
  Mutation: {
    login: rateLimit({ max: 5, window: '15m' })(
      async (_, { input }, ctx) => {
        // Login logic
      }
    ),
  },
};
```

## Next Steps

- [Context](/backend/context)
- [Error Handling](/backend/errors)
- [Production](/backend/production)

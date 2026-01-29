# Production Deployment

Best practices for deploying Better GraphQL servers to production.

## Server Configuration

### Production Settings

```typescript
import { serve } from '@bgql/server';

const server = await serve({
  schema: './schema.bgql',
  resolvers,

  // Production settings
  introspection: process.env.NODE_ENV !== 'production',
  playground: process.env.NODE_ENV !== 'production',

  // Logging
  logging: {
    level: process.env.LOG_LEVEL ?? 'info',
    format: 'json',
  },

  // Performance
  cache: {
    enabled: true,
    ttl: 60000,
  },

  // Security
  cors: {
    origin: process.env.ALLOWED_ORIGINS?.split(',') ?? [],
    credentials: true,
  },

  // Limits
  limits: {
    maxDepth: 10,
    maxComplexity: 1000,
    maxTokens: 10000,
  },
});
```

### Environment Variables

```bash
# .env.production
NODE_ENV=production
PORT=4000
DATABASE_URL=postgres://user:pass@host:5432/db
REDIS_URL=redis://host:6379

# Security
JWT_SECRET=your-secret-key
ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com

# Logging
LOG_LEVEL=info

# Performance
CACHE_TTL=60000
MAX_BATCH_SIZE=10
```

## Health Checks

### Liveness Probe

```typescript
import { serve } from '@bgql/server';

const server = await serve({
  // ...
  health: {
    path: '/health',
    liveness: async () => {
      return { status: 'ok' };
    },
  },
});
```

### Readiness Probe

```typescript
const server = await serve({
  // ...
  health: {
    path: '/health',
    readiness: async (ctx) => {
      // Check database connection
      try {
        await ctx.db.$queryRaw`SELECT 1`;
      } catch (e) {
        return { status: 'error', message: 'Database unavailable' };
      }

      // Check Redis
      try {
        await ctx.redis.ping();
      } catch (e) {
        return { status: 'error', message: 'Redis unavailable' };
      }

      return { status: 'ok' };
    },
  },
});
```

### Kubernetes Configuration

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
spec:
  template:
    spec:
      containers:
        - name: api
          image: your-api:latest
          ports:
            - containerPort: 4000
          livenessProbe:
            httpGet:
              path: /health
              port: 4000
            initialDelaySeconds: 10
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 4000
            initialDelaySeconds: 5
            periodSeconds: 5
          resources:
            requests:
              memory: "256Mi"
              cpu: "250m"
            limits:
              memory: "512Mi"
              cpu: "500m"
```

## Logging

### Structured Logging

```typescript
import { serve, createLogger } from '@bgql/server';

const logger = createLogger({
  level: process.env.LOG_LEVEL ?? 'info',
  format: 'json',
  defaultMeta: {
    service: 'graphql-api',
    version: process.env.APP_VERSION,
  },
});

const server = await serve({
  // ...
  logger,
  logging: {
    // Log all queries
    queries: true,
    // Log slow queries (> 1s)
    slowQueryThreshold: 1000,
    // Log errors
    errors: true,
    // Exclude sensitive fields
    excludeFields: ['password', 'token', 'secret'],
  },
});
```

### Request Logging

```typescript
const server = await serve({
  // ...
  context: async ({ request }) => {
    const requestId = request.headers.get('x-request-id') ?? crypto.randomUUID();

    return {
      requestId,
      logger: logger.child({ requestId }),
    };
  },
});

// Use in resolvers
const resolvers = {
  Query: {
    user: async (_, { id }, ctx) => {
      ctx.logger.info({ userId: id }, 'Fetching user');
      const user = await ctx.db.users.findById(id);
      ctx.logger.info({ found: !!user }, 'User fetch complete');
      return user;
    },
  },
};
```

## Monitoring

### Metrics

```typescript
import { serve, createMetrics } from '@bgql/server';

const metrics = createMetrics({
  // Prometheus format
  format: 'prometheus',
  prefix: 'bgql_',
});

const server = await serve({
  // ...
  metrics,
  metricsPath: '/metrics',
});

// Collected metrics:
// - bgql_query_duration_seconds
// - bgql_query_count
// - bgql_error_count
// - bgql_resolver_duration_seconds
```

### APM Integration

```typescript
// DataDog
import tracer from 'dd-trace';
tracer.init();

// Or OpenTelemetry
import { trace } from '@opentelemetry/api';

const server = await serve({
  // ...
  tracing: {
    enabled: true,
    provider: 'opentelemetry',
    serviceName: 'graphql-api',
  },
});
```

### Custom Metrics

```typescript
const resolvers = {
  Mutation: {
    createUser: async (_, { input }, ctx) => {
      const start = Date.now();

      try {
        const user = await ctx.db.users.create(input);

        ctx.metrics.increment('users.created');
        ctx.metrics.timing('users.create.duration', Date.now() - start);

        return { __typename: 'User', ...user };
      } catch (e) {
        ctx.metrics.increment('users.create.error');
        throw e;
      }
    },
  },
};
```

## Security

### Rate Limiting

```typescript
import { serve, rateLimitMiddleware } from '@bgql/server';

const server = await serve({
  // ...
  middleware: [
    rateLimitMiddleware({
      windowMs: 60 * 1000,  // 1 minute
      max: 100,             // 100 requests per window
      keyGenerator: (ctx) => ctx.user?.id ?? ctx.ip,
      onLimit: (ctx) => {
        ctx.logger.warn({ userId: ctx.user?.id }, 'Rate limit exceeded');
      },
    }),
  ],
});
```

### Query Complexity Limiting

```typescript
const server = await serve({
  // ...
  limits: {
    // Maximum query depth
    maxDepth: 10,

    // Maximum query complexity
    maxComplexity: 1000,

    // Custom complexity calculator
    complexity: {
      defaultFieldComplexity: 1,
      defaultListMultiplier: 10,
      fields: {
        'Query.search': 50,
        'User.posts': 5,
      },
    },
  },
});
```

### Request Validation

```typescript
const server = await serve({
  // ...
  validation: {
    // Limit query size
    maxQuerySize: 10000,  // bytes

    // Limit variables size
    maxVariablesSize: 50000,  // bytes

    // Disable batching in production
    allowBatching: false,

    // Persisted queries only
    persistedQueriesOnly: process.env.NODE_ENV === 'production',
  },
});
```

## Caching

### Response Caching

```typescript
import { serve, responseCacheMiddleware } from '@bgql/server';
import Redis from 'ioredis';

const redis = new Redis(process.env.REDIS_URL);

const server = await serve({
  // ...
  middleware: [
    responseCacheMiddleware({
      cache: redis,
      ttl: 60,  // seconds
      // Cache only public queries
      shouldCache: (ctx, result) => {
        return !ctx.user && !result.errors;
      },
      // Generate cache key
      generateKey: (ctx) => {
        return `query:${hash(ctx.query)}:${hash(ctx.variables)}`;
      },
    }),
  ],
});
```

### Schema Caching

```typescript
const server = await serve({
  // ...
  schema: {
    path: './schema.bgql',
    // Cache parsed schema
    cache: true,
    // Watch for changes in development
    watch: process.env.NODE_ENV !== 'production',
  },
});
```

## Scaling

### Horizontal Scaling

```typescript
// Use Redis for session/cache sharing across instances
import { createRedisPubSub } from '@bgql/server/pubsub';

const pubsub = createRedisPubSub({
  publisher: new Redis(process.env.REDIS_URL),
  subscriber: new Redis(process.env.REDIS_URL),
});

const server = await serve({
  // ...
  pubsub,
  // Stateless sessions
  session: {
    store: 'redis',
    redis: new Redis(process.env.REDIS_URL),
  },
});
```

### Load Balancer Configuration

```nginx
# nginx.conf
upstream graphql {
    least_conn;
    server api-1:4000;
    server api-2:4000;
    server api-3:4000;
}

server {
    listen 80;

    location /graphql {
        proxy_pass http://graphql;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Request-ID $request_id;
    }
}
```

## Error Handling

### Error Masking

```typescript
const server = await serve({
  // ...
  errorHandling: {
    // Mask internal errors in production
    maskErrors: process.env.NODE_ENV === 'production',

    // Custom error formatter
    formatError: (error, ctx) => {
      // Log full error
      ctx.logger.error({ error }, 'GraphQL error');

      // Return sanitized error
      if (process.env.NODE_ENV === 'production') {
        return {
          message: error.extensions?.code === 'INTERNAL_ERROR'
            ? 'Internal server error'
            : error.message,
          extensions: {
            code: error.extensions?.code,
            requestId: ctx.requestId,
          },
        };
      }

      return error;
    },
  },
});
```

### Graceful Shutdown

```typescript
const server = await serve({
  // ...
  gracefulShutdown: {
    enabled: true,
    timeout: 30000,  // 30 seconds
    signals: ['SIGTERM', 'SIGINT'],
    onShutdown: async () => {
      // Close database connections
      await db.$disconnect();
      // Close Redis connections
      await redis.quit();
    },
  },
});

// Or manually
process.on('SIGTERM', async () => {
  console.log('Shutting down...');

  // Stop accepting new connections
  await server.drain();

  // Wait for in-flight requests
  await server.close();

  // Clean up
  await db.$disconnect();

  process.exit(0);
});
```

## Deployment Checklist

### Pre-deployment

- [ ] All tests passing
- [ ] Environment variables configured
- [ ] Database migrations applied
- [ ] Health checks working
- [ ] Logging configured
- [ ] Metrics/monitoring set up
- [ ] Rate limiting configured
- [ ] CORS configured
- [ ] Introspection disabled
- [ ] Playground disabled

### Post-deployment

- [ ] Health checks passing
- [ ] Metrics being collected
- [ ] Logs being ingested
- [ ] No error spikes
- [ ] Response times acceptable
- [ ] Memory usage stable

## Next Steps

- [Performance](/backend/performance)
- [Authentication](/backend/authentication)
- [Error Handling](/backend/errors)

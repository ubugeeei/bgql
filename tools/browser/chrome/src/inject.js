/**
 * bgql DevTools Request Interceptor
 *
 * Injected into the page to intercept fetch/XHR GraphQL requests.
 */

(function() {
  'use strict';

  // Check if already injected
  if (window.__bgqlDevToolsInjected) return;
  window.__bgqlDevToolsInjected = true;

  // GraphQL endpoint patterns
  const GRAPHQL_PATTERNS = [
    /graphql/i,
    /\/gql\/?$/i,
    /\/api\/graphql/i,
  ];

  // Check if URL is a GraphQL endpoint
  function isGraphQLEndpoint(url) {
    return GRAPHQL_PATTERNS.some(pattern => pattern.test(url));
  }

  // Check if body is a GraphQL request
  function isGraphQLRequest(body) {
    if (!body) return false;
    try {
      const parsed = typeof body === 'string' ? JSON.parse(body) : body;
      return parsed && (parsed.query || parsed.mutation);
    } catch {
      return false;
    }
  }

  // Parse GraphQL request body
  function parseGraphQLBody(body) {
    try {
      return typeof body === 'string' ? JSON.parse(body) : body;
    } catch {
      return null;
    }
  }

  // Extract operation name from query
  function extractOperationName(query) {
    if (!query) return null;
    const match = query.match(/(?:query|mutation|subscription)\s+(\w+)/);
    return match ? match[1] : null;
  }

  // Send intercepted query to content script
  function sendToDevTools(data) {
    window.postMessage({
      type: 'BGQL_QUERY_INTERCEPTED',
      data: data,
    }, '*');
  }

  // ============================================================================
  // Fetch Interceptor
  // ============================================================================

  const originalFetch = window.fetch;

  window.fetch = async function(input, init) {
    const url = typeof input === 'string' ? input : input.url;
    const method = init?.method || 'GET';
    const body = init?.body;

    // Check if this is a GraphQL request
    if (method === 'POST' && (isGraphQLEndpoint(url) || isGraphQLRequest(body))) {
      const startTime = performance.now();
      const parsedBody = parseGraphQLBody(body);

      try {
        const response = await originalFetch.apply(this, arguments);
        const endTime = performance.now();

        // Clone response to read body
        const clonedResponse = response.clone();
        const responseData = await clonedResponse.json().catch(() => null);

        sendToDevTools({
          url,
          method,
          query: parsedBody?.query,
          variables: parsedBody?.variables,
          operationName: parsedBody?.operationName || extractOperationName(parsedBody?.query),
          data: responseData?.data,
          errors: responseData?.errors,
          duration: Math.round(endTime - startTime),
          status: response.status,
          timestamp: Date.now(),
        });

        return response;
      } catch (error) {
        const endTime = performance.now();

        sendToDevTools({
          url,
          method,
          query: parsedBody?.query,
          variables: parsedBody?.variables,
          operationName: parsedBody?.operationName || extractOperationName(parsedBody?.query),
          errors: [{ message: error.message }],
          duration: Math.round(endTime - startTime),
          timestamp: Date.now(),
        });

        throw error;
      }
    }

    return originalFetch.apply(this, arguments);
  };

  // ============================================================================
  // XMLHttpRequest Interceptor
  // ============================================================================

  const originalXHROpen = XMLHttpRequest.prototype.open;
  const originalXHRSend = XMLHttpRequest.prototype.send;

  XMLHttpRequest.prototype.open = function(method, url, ...args) {
    this._bgqlUrl = url;
    this._bgqlMethod = method;
    return originalXHROpen.apply(this, [method, url, ...args]);
  };

  XMLHttpRequest.prototype.send = function(body) {
    const url = this._bgqlUrl;
    const method = this._bgqlMethod;

    if (method === 'POST' && (isGraphQLEndpoint(url) || isGraphQLRequest(body))) {
      const startTime = performance.now();
      const parsedBody = parseGraphQLBody(body);

      this.addEventListener('load', () => {
        const endTime = performance.now();

        try {
          const responseData = JSON.parse(this.responseText);

          sendToDevTools({
            url,
            method,
            query: parsedBody?.query,
            variables: parsedBody?.variables,
            operationName: parsedBody?.operationName || extractOperationName(parsedBody?.query),
            data: responseData?.data,
            errors: responseData?.errors,
            duration: Math.round(endTime - startTime),
            status: this.status,
            timestamp: Date.now(),
          });
        } catch {
          // Ignore parse errors
        }
      });

      this.addEventListener('error', () => {
        const endTime = performance.now();

        sendToDevTools({
          url,
          method,
          query: parsedBody?.query,
          variables: parsedBody?.variables,
          operationName: parsedBody?.operationName || extractOperationName(parsedBody?.query),
          errors: [{ message: 'Network error' }],
          duration: Math.round(endTime - startTime),
          timestamp: Date.now(),
        });
      });
    }

    return originalXHRSend.apply(this, arguments);
  };

  console.log('[bgql] Request interceptor injected');
})();

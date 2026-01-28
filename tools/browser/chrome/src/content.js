/**
 * bgql DevTools Content Script
 *
 * Intercepts GraphQL requests and sends them to the devtools panel.
 */

// Inject the request interceptor into the page
function injectInterceptor() {
  const script = document.createElement('script');
  script.src = chrome.runtime.getURL('src/inject.js');
  script.onload = () => script.remove();
  (document.head || document.documentElement).appendChild(script);
}

// Listen for messages from the injected script
window.addEventListener('message', (event) => {
  if (event.source !== window) return;

  if (event.data.type === 'BGQL_QUERY_INTERCEPTED') {
    // Forward to background script
    chrome.runtime.sendMessage({
      type: 'BGQL_QUERY',
      data: event.data.data,
    });
  }
});

// Inject the interceptor
injectInterceptor();

console.log('[bgql] Content script loaded');

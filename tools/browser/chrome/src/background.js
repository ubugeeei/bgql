/**
 * bgql DevTools Background Script
 *
 * Handles communication between content scripts and devtools panel.
 */

// Store queries temporarily
const queryStore = [];
const connectedPorts = new Map();

// Listen for connections from devtools
chrome.runtime.onConnect.addListener((port) => {
  if (port.name === 'bgql-devtools') {
    const tabId = port.sender?.tab?.id;
    if (tabId) {
      connectedPorts.set(tabId, port);
    }

    port.onMessage.addListener((message) => {
      if (message.type === 'GET_QUERIES') {
        // Send stored queries to the panel
        queryStore.forEach(query => {
          port.postMessage({ type: 'BGQL_QUERY', data: query });
        });
      }
    });

    port.onDisconnect.addListener(() => {
      if (tabId) {
        connectedPorts.delete(tabId);
      }
    });
  }
});

// Listen for messages from content scripts
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'BGQL_QUERY') {
    const tabId = sender.tab?.id;
    const query = message.data;

    // Store the query
    queryStore.push(query);
    if (queryStore.length > 100) {
      queryStore.shift();
    }

    // Forward to devtools panel if connected
    if (tabId && connectedPorts.has(tabId)) {
      connectedPorts.get(tabId).postMessage({
        type: 'BGQL_QUERY',
        data: query,
      });
    }

    sendResponse({ received: true });
  }

  return true; // Keep channel open for async response
});

// Clear queries when tab is closed
chrome.tabs.onRemoved.addListener((tabId) => {
  connectedPorts.delete(tabId);
});

// Clear queries when navigating
chrome.webNavigation?.onBeforeNavigate?.addListener((details) => {
  if (details.frameId === 0) {
    // Main frame navigation - optionally clear queries
  }
});

console.log('[bgql] Background script loaded');

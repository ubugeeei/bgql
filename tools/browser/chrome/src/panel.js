/**
 * bgql DevTools Panel
 *
 * Main panel logic for the DevTools extension.
 */

// State
const state = {
  queries: [],
  preserveLog: false,
  filter: '',
};

// DOM Elements
const queryList = document.getElementById('query-list');
const emptyState = document.getElementById('empty-state');
const filterInput = document.getElementById('filterInput');
const clearBtn = document.getElementById('clearBtn');
const preserveBtn = document.getElementById('preserveBtn');
const tabs = document.querySelectorAll('.tab');
const panels = document.querySelectorAll('.panel');

// Initialize
function init() {
  setupEventListeners();
  setupMessageListener();
  loadStoredQueries();
}

// Event Listeners
function setupEventListeners() {
  // Tab switching
  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      const panelId = tab.dataset.panel;
      switchPanel(panelId);
    });
  });

  // Filter input
  filterInput.addEventListener('input', (e) => {
    state.filter = e.target.value.toLowerCase();
    renderQueries();
  });

  // Clear button
  clearBtn.addEventListener('click', () => {
    state.queries = [];
    saveQueries();
    renderQueries();
  });

  // Preserve log toggle
  preserveBtn.addEventListener('click', () => {
    state.preserveLog = !state.preserveLog;
    preserveBtn.textContent = state.preserveLog ? 'Preserve Log ✓' : 'Preserve Log';
    preserveBtn.style.background = state.preserveLog ? 'var(--accent)' : '';
  });
}

// Message listener for content script communication
function setupMessageListener() {
  // Listen for messages from the background script
  chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'BGQL_QUERY') {
      addQuery(message.data);
      sendResponse({ received: true });
    }
  });

  // Also connect via port for real-time updates
  const port = chrome.runtime.connect({ name: 'bgql-devtools' });
  port.onMessage.addListener((message) => {
    if (message.type === 'BGQL_QUERY') {
      addQuery(message.data);
    }
  });

  // Request any stored queries
  port.postMessage({ type: 'GET_QUERIES' });
}

// Switch panel
function switchPanel(panelId) {
  tabs.forEach(tab => {
    tab.classList.toggle('active', tab.dataset.panel === panelId);
  });

  panels.forEach(panel => {
    panel.classList.toggle('hidden', panel.id !== `${panelId}-panel`);
  });
}

// Add a new query
function addQuery(query) {
  state.queries.unshift({
    id: Date.now(),
    ...query,
    timestamp: new Date().toISOString(),
  });

  // Keep max 100 queries
  if (state.queries.length > 100) {
    state.queries = state.queries.slice(0, 100);
  }

  saveQueries();
  renderQueries();
}

// Render queries
function renderQueries() {
  const filtered = state.queries.filter(q => {
    if (!state.filter) return true;
    return (
      q.operationName?.toLowerCase().includes(state.filter) ||
      q.query?.toLowerCase().includes(state.filter)
    );
  });

  if (filtered.length === 0) {
    emptyState.style.display = 'flex';
    queryList.innerHTML = '';
    return;
  }

  emptyState.style.display = 'none';
  queryList.innerHTML = filtered.map(renderQueryItem).join('');

  // Add click handlers
  queryList.querySelectorAll('.query-item').forEach(item => {
    item.querySelector('.query-header').addEventListener('click', () => {
      item.classList.toggle('expanded');
    });
  });
}

// Render a single query item
function renderQueryItem(query) {
  const type = getOperationType(query.query);
  const statusClass = query.errors?.length ? 'error' : 'success';
  const statusText = query.errors?.length
    ? `${query.errors.length} error(s)`
    : 'Success';
  const duration = query.duration ? `${query.duration}ms` : '';

  return `
    <div class="query-item" data-id="${query.id}">
      <div class="query-header">
        <span class="query-type ${type}">${type}</span>
        <span class="query-name">${query.operationName || 'Anonymous'}</span>
        <span class="query-status ${statusClass}">${statusText}</span>
        <span class="query-time">${duration}</span>
      </div>
      <div class="query-details">
        <div class="detail-section">
          <div class="detail-title">Query</div>
          <pre class="code-block">${escapeHtml(query.query || '')}</pre>
        </div>
        ${query.variables ? `
        <div class="detail-section">
          <div class="detail-title">Variables</div>
          <pre class="code-block">${escapeHtml(JSON.stringify(query.variables, null, 2))}</pre>
        </div>
        ` : ''}
        ${query.data ? `
        <div class="detail-section">
          <div class="detail-title">Response</div>
          <pre class="code-block">${escapeHtml(JSON.stringify(query.data, null, 2))}</pre>
        </div>
        ` : ''}
        ${query.errors ? `
        <div class="detail-section">
          <div class="detail-title">Errors</div>
          <pre class="code-block" style="color: var(--error);">${escapeHtml(JSON.stringify(query.errors, null, 2))}</pre>
        </div>
        ` : ''}
      </div>
    </div>
  `;
}

// Get operation type from query string
function getOperationType(query) {
  if (!query) return 'query';
  const trimmed = query.trim().toLowerCase();
  if (trimmed.startsWith('mutation')) return 'mutation';
  if (trimmed.startsWith('subscription')) return 'subscription';
  return 'query';
}

// Escape HTML
function escapeHtml(str) {
  if (!str) return '';
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

// Save queries to storage
function saveQueries() {
  chrome.storage.local.set({ bgqlQueries: state.queries });
}

// Load queries from storage
function loadStoredQueries() {
  chrome.storage.local.get(['bgqlQueries'], (result) => {
    if (result.bgqlQueries) {
      state.queries = result.bgqlQueries;
      renderQueries();
    }
  });
}

// Initialize when DOM is ready
document.addEventListener('DOMContentLoaded', init);

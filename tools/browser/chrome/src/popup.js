/**
 * bgql DevTools Popup
 */

document.addEventListener('DOMContentLoaded', () => {
  // Load stats from storage
  chrome.storage.local.get(['bgqlQueries'], (result) => {
    const queries = result.bgqlQueries || [];
    const errors = queries.filter(q => q.errors?.length > 0);

    document.getElementById('queryCount').textContent = queries.length;
    document.getElementById('errorCount').textContent = errors.length;
  });

  // Open DevTools button
  document.getElementById('openDevTools').addEventListener('click', (e) => {
    e.preventDefault();
    // Note: Can't programmatically open DevTools, show instruction
    alert('Open DevTools (F12) and click on the "bgql" tab to inspect GraphQL queries.');
  });
});

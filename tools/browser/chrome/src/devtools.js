/**
 * bgql DevTools - Creates the DevTools panel
 */

// Create a panel in the DevTools
chrome.devtools.panels.create(
  'bgql',                      // Panel title
  'icons/icon48.png',          // Icon path
  'panel.html',                // Panel HTML
  (panel) => {
    console.log('[bgql] DevTools panel created');

    // Called when the panel is shown
    panel.onShown.addListener((window) => {
      // Send message to panel that it's visible
      window.postMessage({ type: 'BGQL_PANEL_SHOWN' }, '*');
    });

    // Called when the panel is hidden
    panel.onHidden.addListener(() => {
      // Clean up if needed
    });
  }
);

// Create a sidebar in the Elements panel for bgql schema info
chrome.devtools.panels.elements.createSidebarPane(
  'bgql Schema',
  (sidebar) => {
    // Update the sidebar with schema info
    sidebar.setPage('sidebar.html');
  }
);

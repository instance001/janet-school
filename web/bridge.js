(() => {
  const isHttp = window.location.protocol === "http:" || window.location.protocol === "https:";
  if (!isHttp) return;

  window.__JANET_BRIDGE__ = {
    async loadState() {
      const response = await fetch("/api/state", {
        cache: "no-store",
        headers: {
          Accept: "application/json",
        },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      return response.json();
    },

    async loadBridgeStatus() {
      const response = await fetch("/api/bridge-status", {
        cache: "no-store",
        headers: {
          Accept: "application/json",
        },
      });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      return response.json();
    },

    async runGuiAction(payload) {
      const response = await fetch("/api/gui-action", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(payload),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `HTTP ${response.status}`);
      }
      return response.json();
    },

    async saveCompareExport(payload) {
      const response = await fetch("/api/compare-export", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
        },
        body: JSON.stringify(payload),
      });
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || `HTTP ${response.status}`);
      }
      return response.json();
    },
  };
})();

// DeskDash Sandboxed Widget SDK (dd-sdk.js)
// Communicates with the DeskDash Canvas/Bar host over postMessage.
(function () {
  if (window.dd) return;

  const params = new URLSearchParams(window.location.search);
  const INSTANCE_ID = params.get("instanceId") || window.name || "";
  const WIDGET_ID = params.get("widgetId") || "";

  let reqSeq = 1;
  const pending = new Map();
  const listeners = new Map();

  function emitLocal(topic, payload) {
    const set = listeners.get(topic);
    if (!set) return;
    for (const cb of set) {
      try {
        cb(payload);
      } catch (err) {
        console.error("[dd-sdk] listener error:", err);
      }
    }
  }

  function addListener(topic, cb) {
    if (!listeners.has(topic)) listeners.set(topic, new Set());
    listeners.get(topic).add(cb);
    return () => {
      const set = listeners.get(topic);
      if (set) set.delete(cb);
    };
  }

  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== "object") return;

    if (msg.__dd_resp && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.ok) resolve(msg.data);
      else reject(new Error(msg.error || "DeskDash SDK call failed"));
      return;
    }

    if (msg.__dd_event && typeof msg.topic === "string") {
      emitLocal(msg.topic, msg.payload);
    }
  });

  function invoke(method, callParams = {}) {
    return new Promise((resolve, reject) => {
      const id = reqSeq++;
      pending.set(id, { resolve, reject });
      window.parent.postMessage(
        {
          __dd: true,
          id,
          instanceId: INSTANCE_ID,
          widgetId: WIDGET_ID,
          method,
          params: callParams,
        },
        "*"
      );
      setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id);
          reject(new Error(`DeskDash SDK timeout for ${method}`));
        }
      }, 20000);
    });
  }

  function reportBarHitRects() {
    const nodes = document.querySelectorAll("dd-bar, [data-dd-interactive]");
    const rects = [];
    for (const el of nodes) {
      const r = el.getBoundingClientRect();
      if (r.width > 0 && r.height > 0) {
        rects.push({ x: r.left, y: r.top, width: r.width, height: r.height });
      }
    }
    window.parent.postMessage(
      {
        __dd_rects: true,
        instanceId: INSTANCE_ID,
        rects,
      },
      "*"
    );
  }

  let rectTimer = null;
  function scheduleRectReport() {
    if (rectTimer) cancelAnimationFrame(rectTimer);
    rectTimer = requestAnimationFrame(() => {
      rectTimer = null;
      reportBarHitRects();
    });
  }

  class DdBarElement extends HTMLElement {
    connectedCallback() {
      if (!this._ro && typeof ResizeObserver !== "undefined") {
        this._ro = new ResizeObserver(scheduleRectReport);
        this._ro.observe(this);
      }
      scheduleRectReport();
    }
    disconnectedCallback() {
      if (this._ro) {
        this._ro.disconnect();
        this._ro = null;
      }
      scheduleRectReport();
    }
  }

  if (typeof customElements !== "undefined" && !customElements.get("dd-bar")) {
    customElements.define("dd-bar", DdBarElement);
  }

  window.addEventListener("resize", scheduleRectReport);
  window.addEventListener("DOMContentLoaded", scheduleRectReport);

  const dd = {
    version: "0.1.0",
    instanceId: INSTANCE_ID,
    widgetId: WIDGET_ID,

    widget: {
      getConfig: () => invoke("widget.getConfig"),
      onConfigChange: (cb) => addListener("config", cb),
      storage: {
        get: (key) => invoke("storage.get", { key }),
        set: (key, value) => invoke("storage.set", { key, value }),
        delete: (key) => invoke("storage.delete", { key }),
        list: () => invoke("storage.list"),
      },
      reportHitRects: reportBarHitRects,
    },

    system: {
      getStats: () => invoke("system.getStats"),
      subscribe: (intervalMs, cb) => {
        const ms = Math.max(500, Number(intervalMs) || 1500);
        let active = true;
        const poll = async () => {
          if (!active) return;
          try {
            const stats = await invoke("system.getStats");
            if (active) cb(stats);
          } catch (_) {}
          if (active) setTimeout(poll, ms);
        };
        poll();
        return () => {
          active = false;
        };
      },
    },

    media: {
      getSession: () => invoke("media.getSession"),
      subscribe: (cb, intervalMs = 1500) => {
        let active = true;
        const poll = async () => {
          if (!active) return;
          try {
            const session = await invoke("media.getSession");
            if (active) cb(session);
          } catch (_) {}
          if (active) setTimeout(poll, Math.max(500, intervalMs));
        };
        poll();
        return () => {
          active = false;
        };
      },
      playPause: () => invoke("media.playPause"),
      next: () => invoke("media.next"),
      prev: () => invoke("media.prev"),
    },

    network: {
      fetch: (url, opts = {}) =>
        invoke("network.fetch", {
          url,
          method: opts.method || "GET",
          headers: opts.headers || {},
          body: opts.body || null,
        }),
    },

    clipboard: {
      readText: () => invoke("clipboard.readText"),
      writeText: (text) => invoke("clipboard.writeText", { text: String(text) }),
    },

    taskbar: {
      listWindows: () => invoke("taskbar.listWindows"),
      subscribeWindows: (cb, intervalMs = 1000) => {
        let active = true;
        const poll = async () => {
          if (!active) return;
          try {
            const windows = await invoke("taskbar.listWindows");
            if (active) cb(windows);
          } catch (_) {}
          if (active) setTimeout(poll, Math.max(500, intervalMs));
        };
        poll();
        return () => {
          active = false;
        };
      },
      focusWindow: (id) => invoke("taskbar.focusWindow", { id }),
      minimizeWindow: (id) => invoke("taskbar.minimizeWindow", { id }),
      closeWindow: (id) => invoke("taskbar.closeWindow", { id }),
      launchApp: (path, args = []) => invoke("launcher.launchApp", { path, args }),
      openStartMenu: () => invoke("taskbar.openStartMenu"),
    },

    theme: {
      get: () => invoke("theme.get"),
      onChange: (cb) => addListener("theme", cb),
    },
  };

  window.dd = dd;
})();

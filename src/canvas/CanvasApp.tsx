import React, { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Lock, Unlock, Trash2, Check } from "lucide-react";
import { api, widgetIframeUrl } from "../lib/tauri";
import type { AppConfig, InstalledWidget, WidgetInstance } from "../types";

export const CanvasApp: React.FC = () => {
  const monitorId = useMemo(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("monitorId") || "primary";
  }, []);

  const [config, setConfig] = useState<AppConfig | null>(null);
  const [installed, setInstalled] = useState<InstalledWidget[]>([]);
  const [dragging, setDragging] = useState<{
    id: string;
    mode: "move" | "resize";
    startX: number;
    startY: number;
    origX: number;
    origY: number;
    origW: number;
    origH: number;
  } | null>(null);

  const iframeRefs = useRef<Map<string, HTMLIFrameElement>>(new Map());

  const loadState = async () => {
    const [cfg, list] = await Promise.all([
      api.getConfig(),
      api.listInstalledWidgets(),
    ]);
    setConfig(cfg);
    setInstalled(list);
  };

  useEffect(() => {
    loadState();
    const unlistenCfg = listen<AppConfig>("deskdash:config-updated", (ev) => {
      setConfig(ev.payload);
      // Notify widget iframes about any updated settings
      for (const w of ev.payload.widgets) {
        const frame = iframeRefs.current.get(w.instanceId);
        frame?.contentWindow?.postMessage(
          { __dd_event: true, topic: "config", payload: w.settings },
          "*"
        );
      }
    });
    const unlistenEdit = listen<boolean>("deskdash:edit-mode-changed", (ev) => {
      setConfig((prev) => (prev ? { ...prev, editMode: ev.payload } : prev));
    });
    return () => {
      unlistenCfg.then((f) => f());
      unlistenEdit.then((f) => f());
    };
  }, []);

  // Bridge sandboxed widget iframe postMessage calls to permission-gated Rust dd_invoke
  useEffect(() => {
    const onMessage = async (event: MessageEvent) => {
      const msg = event.data;
      if (!msg || typeof msg !== "object" || !msg.__dd) return;
      const source = event.source as WindowProxy | null;
      try {
        const resp = await api.ddInvoke({
          instanceId: String(msg.instanceId || ""),
          method: String(msg.method || ""),
          params: (msg.params as Record<string, unknown>) || {},
        });
        source?.postMessage(
          {
            __dd_resp: true,
            id: msg.id,
            ok: resp.ok,
            data: resp.data,
            error: resp.error,
          },
          "*"
        );
      } catch (err) {
        source?.postMessage(
          {
            __dd_resp: true,
            id: msg.id,
            ok: false,
            error: String(err),
          },
          "*"
        );
      }
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, []);

  const monitorWidgets = useMemo(() => {
    if (!config) return [];
    return config.widgets
      .filter((w) => w.enabled && w.monitorId === monitorId)
      .sort((a, b) => a.zIndex - b.zIndex);
  }, [config, monitorId]);

  // Report interactive widget bounding rects to Win32 hit-test engine
  useEffect(() => {
    const label = getCurrentWindow().label;
    const scale = window.devicePixelRatio || 1;
    const rects = monitorWidgets.map((w) => ({
      x: w.x * scale,
      y: w.y * scale,
      width: w.width * scale,
      height: w.height * scale,
    }));
    api.updateInteractiveRects(label, rects).catch(() => {});
  }, [monitorWidgets]);

  const snap = (val: number) => {
    const step = config?.settings.gridSnap || 12;
    return step > 1 ? Math.round(val / step) * step : val;
  };

  useEffect(() => {
    if (!dragging || !config) return;

    const onMove = (e: PointerEvent) => {
      const dx = e.clientX - dragging.startX;
      const dy = e.clientY - dragging.startY;
      setConfig((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          widgets: prev.widgets.map((w) => {
            if (w.instanceId !== dragging.id) return w;
            if (dragging.mode === "move") {
              return {
                ...w,
                x: Math.max(0, snap(dragging.origX + dx)),
                y: Math.max(0, snap(dragging.origY + dy)),
              };
            }
            return {
              ...w,
              width: Math.max(140, snap(dragging.origW + dx)),
              height: Math.max(90, snap(dragging.origH + dy)),
            };
          }),
        };
      });
    };

    const onUp = () => {
      const updated = config.widgets.find((w) => w.instanceId === dragging.id);
      setDragging(null);
      if (updated) {
        api.updateWidgetInstance(updated).catch(() => {});
      }
    };

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  }, [dragging, config]);

  const manifestMap = useMemo(() => {
    const m = new Map<string, InstalledWidget>();
    for (const w of installed) m.set(w.manifest.id, w);
    return m;
  }, [installed]);

  const editMode = Boolean(config?.editMode);
  const gridStep = config?.settings.gridSnap || 12;

  return (
    <div
      className="relative h-screen w-screen overflow-hidden select-none"
      style={
        editMode
          ? {
              backgroundColor: "rgba(9, 11, 18, 0.38)",
              backgroundImage:
                "radial-gradient(rgba(148, 163, 184, 0.25) 1px, transparent 1px)",
              backgroundSize: `${gridStep * 2}px ${gridStep * 2}px`,
            }
          : { backgroundColor: "transparent" }
      }
    >
      {editMode && (
        <div className="fixed top-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-3 rounded-full border border-indigo-400/40 bg-zinc-950/90 px-5 py-2 text-xs font-medium text-zinc-100 shadow-2xl backdrop-blur-xl">
          <span className="inline-block h-2 w-2 rounded-full bg-indigo-400 animate-pulse" />
          <span>Edit Mode — Drag or resize widgets</span>
          <button
            onClick={() => api.setEditMode(false)}
            className="ml-2 inline-flex items-center gap-1 rounded-full bg-indigo-500 px-3 py-1 text-xs font-semibold text-white hover:bg-indigo-400 transition cursor-pointer"
          >
            <Check className="h-3.5 w-3.5" /> Done
          </button>
        </div>
      )}

      {monitorWidgets.map((inst: WidgetInstance) => {
        const meta = manifestMap.get(inst.widgetId);
        const entry = meta?.manifest.entry || "index.html";
        const src = widgetIframeUrl(inst.widgetId, entry, inst.instanceId);

        return (
          <div
            key={inst.instanceId}
            className="absolute"
            style={{
              left: inst.x,
              top: inst.y,
              width: inst.width,
              height: inst.height,
              zIndex: inst.zIndex,
              opacity: inst.opacity,
            }}
          >
            <iframe
              ref={(el) => {
                if (el) iframeRefs.current.set(inst.instanceId, el);
                else iframeRefs.current.delete(inst.instanceId);
              }}
              src={src}
              name={inst.instanceId}
              title={meta?.manifest.name || inst.widgetId}
              sandbox="allow-scripts allow-forms"
              className="h-full w-full border-0 bg-transparent"
              style={{ pointerEvents: editMode ? "none" : "auto" }}
            />

            {editMode && (
              <div
                onPointerDown={(e) => {
                  if (inst.locked) return;
                  e.stopPropagation();
                  setDragging({
                    id: inst.instanceId,
                    mode: "move",
                    startX: e.clientX,
                    startY: e.clientY,
                    origX: inst.x,
                    origY: inst.y,
                    origW: inst.width,
                    origH: inst.height,
                  });
                }}
                className="absolute inset-0 rounded-2xl border-2 border-indigo-400/80 bg-indigo-500/10 cursor-move transition hover:border-indigo-300"
              >
                <div className="flex items-center justify-between rounded-t-xl bg-zinc-950/85 px-2.5 py-1 text-[11px] font-medium text-zinc-200 backdrop-blur-md">
                  <span className="truncate">{meta?.manifest.name || inst.widgetId}</span>
                  <div className="flex items-center gap-1">
                    <button
                      type="button"
                      title={inst.locked ? "Unlock" : "Lock"}
                      onPointerDown={(e) => e.stopPropagation()}
                      onClick={() =>
                        api.updateWidgetInstance({ ...inst, locked: !inst.locked })
                      }
                      className="rounded p-1 hover:bg-zinc-800 cursor-pointer"
                    >
                      {inst.locked ? (
                        <Lock className="h-3 w-3 text-amber-400" />
                      ) : (
                        <Unlock className="h-3 w-3 text-zinc-400" />
                      )}
                    </button>
                    <button
                      type="button"
                      title="Remove widget"
                      onPointerDown={(e) => e.stopPropagation()}
                      onClick={() => api.removeWidgetInstance(inst.instanceId)}
                      className="rounded p-1 text-rose-400 hover:bg-rose-500/20 cursor-pointer"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  </div>
                </div>

                {!inst.locked && (
                  <div
                    onPointerDown={(e) => {
                      e.stopPropagation();
                      setDragging({
                        id: inst.instanceId,
                        mode: "resize",
                        startX: e.clientX,
                        startY: e.clientY,
                        origX: inst.x,
                        origY: inst.y,
                        origW: inst.width,
                        origH: inst.height,
                      });
                    }}
                    className="absolute bottom-1 right-1 h-4 w-4 cursor-nwse-resize rounded-br-lg border-r-2 border-b-2 border-indigo-300"
                  />
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

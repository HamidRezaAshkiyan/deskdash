import React, { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, widgetIframeUrl } from "../lib/tauri";
import type { BarInstance, InstalledWidget, Rect } from "../types";

export const BarApp: React.FC = () => {
  const instanceId = useMemo(() => {
    const params = new URLSearchParams(window.location.search);
    return params.get("instanceId") || "";
  }, []);

  const [bar, setBar] = useState<BarInstance | null>(null);
  const [entry, setEntry] = useState("index.html");
  const iframeRef = useRef<HTMLIFrameElement | null>(null);

  useEffect(() => {
    const init = async () => {
      const [cfg, installed] = await Promise.all([
        api.getConfig(),
        api.listInstalledWidgets(),
      ]);
      const found = cfg.bars.find((b) => b.instanceId === instanceId) || null;
      setBar(found);
      if (found) {
        const meta = installed.find(
          (w: InstalledWidget) => w.manifest.id === found.widgetId
        );
        if (meta?.manifest.entry) setEntry(meta.manifest.entry);
      }
    };
    init();

    const unlisten = listen<BarInstance>("deskdash:bar-updated", (ev) => {
      if (ev.payload.instanceId === instanceId) {
        setBar(ev.payload);
        iframeRef.current?.contentWindow?.postMessage(
          { __dd_event: true, topic: "config", payload: ev.payload.settings },
          "*"
        );
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [instanceId]);

  useEffect(() => {
    const label = getCurrentWindow().label;
    const onMessage = async (event: MessageEvent) => {
      const msg = event.data;
      if (!msg || typeof msg !== "object") return;

      // Forward <dd-bar> interactive island bounds to Win32 click-through engine
      if (msg.__dd_rects && Array.isArray(msg.rects)) {
        const scale = window.devicePixelRatio || 1;
        const rects: Rect[] = msg.rects.map((r: Rect) => ({
          x: r.x * scale,
          y: r.y * scale,
          width: r.width * scale,
          height: r.height * scale,
        }));
        api.updateInteractiveRects(label, rects).catch(() => {});
        return;
      }

      if (!msg.__dd) return;
      const source = event.source as WindowProxy | null;
      try {
        const resp = await api.ddInvoke({
          instanceId: String(msg.instanceId || instanceId),
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
  }, [instanceId]);

  if (!bar) return null;

  const src = widgetIframeUrl(bar.widgetId, entry, bar.instanceId);

  return (
    <div className="h-screen w-screen overflow-hidden bg-transparent">
      <iframe
        ref={iframeRef}
        src={src}
        name={bar.instanceId}
        title={bar.widgetId}
        sandbox="allow-scripts allow-forms"
        className="h-full w-full border-0 bg-transparent"
      />
    </div>
  );
};

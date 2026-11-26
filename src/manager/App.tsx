import React, { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  LayoutDashboard,
  Blocks,
  Store,
  Settings as SettingsIcon,
  Pencil,
  Monitor,
  Plus,
  Trash2,
  SlidersHorizontal,
  FolderInput,
  RotateCcw,
  Eye,
  EyeOff,
  PanelBottom,
  Shield,
  Download,
} from "lucide-react";
import { api } from "../lib/tauri";
import { useDeskDashStore } from "../store/useDeskDashStore";
import type {
  AppConfig,
  BarInstance,
  WidgetInstance,
  WidgetManifest,
  WidgetPermissions,
} from "../types";
import { PermissionModal } from "./components/PermissionModal";
import { WidgetSettingsDrawer } from "./components/WidgetSettingsDrawer";

type Tab = "dashboard" | "widgets" | "marketplace" | "settings";

export const App: React.FC = () => {
  const {
    config,
    monitors,
    installed,
    marketplace,
    toasts,
    refreshAll,
    refreshMarketplace,
    setConfig,
    pushToast,
    dismissToast,
  } = useDeskDashStore();

  const [tab, setTab] = useState<Tab>("dashboard");
  const [selectedMonitor, setSelectedMonitor] = useState<string>("primary");
  const [search, setSearch] = useState("");
  const [pendingAdd, setPendingAdd] = useState<WidgetManifest | null>(null);
  const [editingWidget, setEditingWidget] = useState<WidgetInstance | null>(null);
  const [editingBar, setEditingBar] = useState<BarInstance | null>(null);

  useEffect(() => {
    refreshAll();
    refreshMarketplace();
    const unlisten = listen<AppConfig>("deskdash:config-updated", (ev) => {
      setConfig(ev.payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const manifestById = useMemo(() => {
    const m = new Map<string, WidgetManifest>();
    for (const w of installed) m.set(w.manifest.id, w.manifest);
    return m;
  }, [installed]);

  const handleConfirmAdd = async (granted: WidgetPermissions) => {
    if (!pendingAdd) return;
    try {
      if (pendingAdd.kind === "bar") {
        await api.addBarInstance(pendingAdd.id, selectedMonitor, granted);
        pushToast(`Added taskbar "${pendingAdd.name}"`, "success");
      } else {
        await api.addWidgetInstance(pendingAdd.id, selectedMonitor, granted);
        pushToast(`Added widget "${pendingAdd.name}"`, "success");
      }
      setPendingAdd(null);
      await refreshAll();
    } catch (err) {
      pushToast(String(err), "error");
    }
  };

  const handleImportLocal = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Select Widget Folder (containing manifest.json)",
      });
      if (typeof selected === "string") {
        const manifest = await api.importLocalWidget(selected);
        pushToast(`Imported "${manifest.name}" v${manifest.version}`, "success");
        await refreshAll();
        await refreshMarketplace();
      }
    } catch (err) {
      pushToast(`Import failed: ${String(err)}`, "error");
    }
  };

  const toggleEditMode = async () => {
    if (!config) return;
    try {
      await api.setEditMode(!config.editMode);
    } catch (err) {
      pushToast(String(err), "error");
    }
  };

  const filteredMarketplace = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return marketplace;
    return marketplace.filter(
      (i) =>
        i.name.toLowerCase().includes(q) ||
        i.description.toLowerCase().includes(q) ||
        i.category.toLowerCase().includes(q)
    );
  }, [marketplace, search]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-zinc-950 text-zinc-100">
      {/* Sidebar */}
      <aside className="flex w-60 flex-col border-r border-zinc-800/80 bg-zinc-950 p-4">
        <div className="flex items-center gap-2.5 px-2 py-1">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-gradient-to-br from-indigo-500 to-sky-400 text-sm font-bold text-white shadow-lg shadow-indigo-500/20">
            DD
          </div>
          <div>
            <div className="text-sm font-bold tracking-tight">DeskDash</div>
            <div className="text-[11px] text-zinc-500">Windows Desktop Engine</div>
          </div>
        </div>

        <nav className="mt-6 flex-1 space-y-1">
          {[
            { id: "dashboard" as Tab, label: "Dashboard", icon: LayoutDashboard },
            { id: "widgets" as Tab, label: "Widgets & Bars", icon: Blocks },
            { id: "marketplace" as Tab, label: "Marketplace", icon: Store },
            { id: "settings" as Tab, label: "Settings", icon: SettingsIcon },
          ].map((item) => {
            const Icon = item.icon;
            const active = tab === item.id;
            return (
              <button
                key={item.id}
                onClick={() => setTab(item.id)}
                className={`flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-xs font-medium transition cursor-pointer ${
                  active
                    ? "bg-indigo-500/15 text-indigo-300"
                    : "text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200"
                }`}
              >
                <Icon className="h-4 w-4" />
                {item.label}
              </button>
            );
          })}
        </nav>

        <div className="space-y-2 border-t border-zinc-800/80 pt-4">
          <button
            onClick={toggleEditMode}
            className={`flex w-full items-center justify-center gap-2 rounded-xl px-3 py-2 text-xs font-semibold transition cursor-pointer ${
              config?.editMode
                ? "bg-indigo-500 text-white shadow-lg shadow-indigo-500/30"
                : "border border-zinc-800 bg-zinc-900 text-zinc-200 hover:bg-zinc-800"
            }`}
          >
            <Pencil className="h-3.5 w-3.5" />
            {config?.editMode ? "Exit Edit Mode" : "Edit Desktop Layout"}
          </button>

          <button
            onClick={() => api.openCanvasPreview().catch((e) => pushToast(String(e), "error"))}
            className="flex w-full items-center justify-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2 text-xs font-medium text-zinc-300 hover:bg-zinc-800 cursor-pointer"
          >
            <Monitor className="h-3.5 w-3.5" />
            Show Desktop Canvas
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto p-7">
        {/* Target Monitor Picker */}
        <div className="mb-6 flex items-center justify-between">
          <div>
            <h1 className="text-xl font-bold tracking-tight capitalize">
              {tab === "widgets" ? "Installed Widgets & Taskbars" : tab}
            </h1>
            <p className="text-xs text-zinc-400">
              Manage live wallpaper widgets, sandboxed permissions, and alternative taskbars.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-xs text-zinc-500">Target Monitor:</span>
            <select
              value={selectedMonitor}
              onChange={(e) => setSelectedMonitor(e.target.value)}
              className="rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-200"
            >
              {monitors.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.name} ({m.width}×{m.height})
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* DASHBOARD TAB */}
        {tab === "dashboard" && config && (
          <div className="space-y-6">
            <section>
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  Active Desktop Widgets ({config.widgets.length})
                </h2>
                <button
                  onClick={() => setTab("widgets")}
                  className="inline-flex items-center gap-1 rounded-lg bg-indigo-500/15 px-2.5 py-1 text-xs font-medium text-indigo-300 hover:bg-indigo-500/25 cursor-pointer"
                >
                  <Plus className="h-3.5 w-3.5" /> Add Widget
                </button>
              </div>
              <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                {config.widgets.map((inst) => {
                  const meta = manifestById.get(inst.widgetId);
                  return (
                    <div
                      key={inst.instanceId}
                      className="flex items-center justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4"
                    >
                      <div>
                        <div className="text-sm font-semibold text-zinc-100">
                          {meta?.name || inst.widgetId}
                        </div>
                        <div className="mt-0.5 text-[11px] text-zinc-400">
                          {inst.monitorId} • {Math.round(inst.x)},{Math.round(inst.y)} •{" "}
                          {Math.round(inst.width)}×{Math.round(inst.height)}
                        </div>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <button
                          onClick={() =>
                            api.updateWidgetInstance({ ...inst, enabled: !inst.enabled })
                          }
                          title={inst.enabled ? "Hide" : "Show"}
                          className="rounded-lg p-2 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 cursor-pointer"
                        >
                          {inst.enabled ? <Eye className="h-4 w-4" /> : <EyeOff className="h-4 w-4" />}
                        </button>
                        <button
                          onClick={() => setEditingWidget(inst)}
                          title="Configure"
                          className="rounded-lg p-2 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 cursor-pointer"
                        >
                          <SlidersHorizontal className="h-4 w-4" />
                        </button>
                        <button
                          onClick={() => api.removeWidgetInstance(inst.instanceId)}
                          title="Remove"
                          className="rounded-lg p-2 text-rose-400 hover:bg-rose-500/15 cursor-pointer"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                  );
                })}
                {config.widgets.length === 0 && (
                  <div className="col-span-2 rounded-2xl border border-dashed border-zinc-800 p-8 text-center text-xs text-zinc-500">
                    No active widgets yet. Add one from the Widgets & Bars tab.
                  </div>
                )}
              </div>
            </section>

            <section>
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-400">
                  Alternative Taskbars ({config.bars.length})
                </h2>
                <button
                  onClick={() =>
                    api
                      .restoreNativeTaskbar()
                      .then(() => pushToast("Restored Windows Taskbar", "success"))
                  }
                  className="inline-flex items-center gap-1.5 rounded-lg border border-zinc-800 px-2.5 py-1 text-xs text-zinc-300 hover:bg-zinc-800 cursor-pointer"
                >
                  <RotateCcw className="h-3.5 w-3.5" /> Restore Windows Taskbar
                </button>
              </div>
              <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                {config.bars.map((bar) => {
                  const meta = manifestById.get(bar.widgetId);
                  return (
                    <div
                      key={bar.instanceId}
                      className="flex items-center justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4"
                    >
                      <div className="flex items-center gap-3">
                        <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-sky-500/15 text-sky-400">
                          <PanelBottom className="h-4 w-4" />
                        </div>
                        <div>
                          <div className="text-sm font-semibold text-zinc-100">
                            {meta?.name || bar.widgetId}
                          </div>
                          <div className="text-[11px] text-zinc-400">
                            Edge: {bar.edge} • {bar.reserveSpace ? "AppBar reserved" : "Overlay"}
                            {bar.hideNativeTaskbar ? " • Native taskbar hidden" : ""}
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <button
                          onClick={() => setEditingBar(bar)}
                          className="rounded-lg p-2 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 cursor-pointer"
                        >
                          <SlidersHorizontal className="h-4 w-4" />
                        </button>
                        <button
                          onClick={() => api.removeBarInstance(bar.instanceId)}
                          className="rounded-lg p-2 text-rose-400 hover:bg-rose-500/15 cursor-pointer"
                        >
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    </div>
                  );
                })}
                {config.bars.length === 0 && (
                  <div className="col-span-2 rounded-2xl border border-dashed border-zinc-800 p-8 text-center text-xs text-zinc-500">
                    No alternative taskbars enabled. Try adding <strong>Floating Glass Dock</strong> or <strong>Top Status Rail</strong> from Widgets & Bars.
                  </div>
                )}
              </div>
            </section>
          </div>
        )}

        {/* WIDGETS & BARS TAB */}
        {tab === "widgets" && (
          <div>
            <div className="mb-4 flex justify-end">
              <button
                onClick={handleImportLocal}
                className="inline-flex items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2 text-xs font-medium text-zinc-200 hover:bg-zinc-800 cursor-pointer"
              >
                <FolderInput className="h-4 w-4 text-indigo-400" />
                Import Local Widget Folder
              </button>
            </div>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {installed.map(({ manifest, builtin }) => (
                <div
                  key={manifest.id}
                  className="flex flex-col justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4"
                >
                  <div>
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-sm font-semibold text-zinc-100">
                        {manifest.name}
                      </span>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold uppercase ${
                          manifest.kind === "bar"
                            ? "bg-sky-500/15 text-sky-300"
                            : "bg-indigo-500/15 text-indigo-300"
                        }`}
                      >
                        {manifest.kind}
                      </span>
                    </div>
                    <p className="mt-1.5 text-xs text-zinc-400 line-clamp-2">
                      {manifest.description}
                    </p>
                    <div className="mt-3 flex flex-wrap gap-1">
                      {manifest.permissions.system && (
                        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          system
                        </span>
                      )}
                      {manifest.permissions.media && (
                        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          media
                        </span>
                      )}
                      {manifest.permissions.taskbar && (
                        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          taskbar
                        </span>
                      )}
                      {manifest.permissions.storage && (
                        <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-300">
                          storage
                        </span>
                      )}
                      {manifest.permissions.network.length > 0 && (
                        <span className="rounded bg-sky-500/15 px-1.5 py-0.5 text-[10px] text-sky-300">
                          network ({manifest.permissions.network.length})
                        </span>
                      )}
                    </div>
                  </div>
                  <div className="mt-4 flex items-center justify-between border-t border-zinc-800/60 pt-3">
                    <span className="text-[11px] text-zinc-500">
                      v{manifest.version} {builtin ? "• Built-in" : ""}
                    </span>
                    <div className="flex items-center gap-1.5">
                      {!builtin && (
                        <button
                          onClick={() =>
                            api
                              .uninstallWidget(manifest.id)
                              .then(() => refreshAll())
                              .catch((e) => pushToast(String(e), "error"))
                          }
                          className="rounded-lg p-1.5 text-rose-400 hover:bg-rose-500/15 cursor-pointer"
                          title="Uninstall"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      )}
                      <button
                        onClick={() => setPendingAdd(manifest)}
                        className="inline-flex items-center gap-1 rounded-lg bg-indigo-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-indigo-400 cursor-pointer"
                      >
                        <Plus className="h-3.5 w-3.5" /> Add
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* MARKETPLACE TAB */}
        {tab === "marketplace" && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <input
                type="text"
                placeholder="Search widgets, taskbars, categories..."
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                className="flex-1 rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:border-indigo-500 focus:outline-none"
              />
              <button
                onClick={() => refreshMarketplace()}
                className="rounded-xl border border-zinc-800 bg-zinc-900 px-3.5 py-2 text-xs font-medium text-zinc-300 hover:bg-zinc-800 cursor-pointer"
              >
                Refresh Catalog
              </button>
            </div>
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {filteredMarketplace.map((item) => (
                <div
                  key={item.id}
                  className="flex flex-col justify-between rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-4"
                >
                  <div>
                    <div className="flex items-center justify-between">
                      <span className="text-sm font-semibold">{item.name}</span>
                      <span className="rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] text-zinc-300">
                        {item.category}
                      </span>
                    </div>
                    <p className="mt-1.5 text-xs text-zinc-400">{item.description}</p>
                    <div className="mt-2 flex items-center gap-1 text-[11px] text-zinc-500">
                      <Shield className="h-3 w-3 text-emerald-400" />
                      <span>by {item.author || "Community"} • v{item.version}</span>
                    </div>
                  </div>
                  <div className="mt-4 flex justify-end border-t border-zinc-800/60 pt-3">
                    {item.installed ? (
                      <span className="rounded-lg bg-emerald-500/15 px-3 py-1 text-xs font-medium text-emerald-300">
                        Installed
                      </span>
                    ) : item.downloadUrl ? (
                      <button
                        onClick={() =>
                          api
                            .installMarketplaceWidget(item.downloadUrl!, item.sha256)
                            .then(async (m) => {
                              pushToast(`Installed "${m.name}"`, "success");
                              await refreshAll();
                              await refreshMarketplace();
                            })
                            .catch((e) => pushToast(String(e), "error"))
                        }
                        className="inline-flex items-center gap-1.5 rounded-lg bg-indigo-500 px-3 py-1.5 text-xs font-semibold text-white hover:bg-indigo-400 cursor-pointer"
                      >
                        <Download className="h-3.5 w-3.5" /> Install
                      </button>
                    ) : null}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* SETTINGS TAB */}
        {tab === "settings" && config && (
          <div className="max-w-xl space-y-4 rounded-2xl border border-zinc-800/80 bg-zinc-900/60 p-5">
            <label className="flex items-center justify-between text-xs text-zinc-200">
              <span>Launch DeskDash on Windows startup</span>
              <input
                type="checkbox"
                checked={config.settings.autostart}
                onChange={(e) =>
                  api.updateSettings({ ...config.settings, autostart: e.target.checked })
                }
                className="accent-indigo-500"
              />
            </label>
            <label className="flex items-center justify-between text-xs text-zinc-200">
              <span>Auto-pause telemetry when a fullscreen app/game is active</span>
              <input
                type="checkbox"
                checked={config.settings.autoPauseFullscreen}
                onChange={(e) =>
                  api.updateSettings({
                    ...config.settings,
                    autoPauseFullscreen: e.target.checked,
                  })
                }
                className="accent-indigo-500"
              />
            </label>
            <label className="flex items-center justify-between text-xs text-zinc-200">
              <span>Minimize Manager to system tray on close</span>
              <input
                type="checkbox"
                checked={config.settings.minimizeToTray}
                onChange={(e) =>
                  api.updateSettings({
                    ...config.settings,
                    minimizeToTray: e.target.checked,
                  })
                }
                className="accent-indigo-500"
              />
            </label>
            <label className="block text-xs text-zinc-300">
              Edit Mode Grid Snap ({config.settings.gridSnap}px)
              <input
                type="range"
                min={1}
                max={32}
                value={config.settings.gridSnap}
                onChange={(e) =>
                  api.updateSettings({
                    ...config.settings,
                    gridSnap: Number(e.target.value),
                  })
                }
                className="mt-1.5 w-full accent-indigo-500"
              />
            </label>
            <div className="border-t border-zinc-800 pt-4">
              <button
                onClick={() =>
                  api
                    .restoreNativeTaskbar()
                    .then(() => pushToast("Windows Taskbar restored", "success"))
                }
                className="inline-flex items-center gap-2 rounded-xl border border-amber-500/30 bg-amber-500/10 px-3.5 py-2 text-xs font-medium text-amber-300 hover:bg-amber-500/20 cursor-pointer"
              >
                <RotateCcw className="h-3.5 w-3.5" />
                Emergency Restore Native Windows Taskbar
              </button>
            </div>
          </div>
        )}
      </main>

      {/* Modals & Drawers */}
      {pendingAdd && (
        <PermissionModal
          manifest={pendingAdd}
          onConfirm={handleConfirmAdd}
          onCancel={() => setPendingAdd(null)}
        />
      )}

      {editingWidget && manifestById.get(editingWidget.widgetId) && (
        <WidgetSettingsDrawer
          manifest={manifestById.get(editingWidget.widgetId)!}
          widgetInstance={editingWidget}
          onSaveWidget={(u) => {
            setEditingWidget(u);
            api.updateWidgetInstance(u).catch((e) => pushToast(String(e), "error"));
          }}
          onClose={() => setEditingWidget(null)}
        />
      )}

      {editingBar && manifestById.get(editingBar.widgetId) && (
        <WidgetSettingsDrawer
          manifest={manifestById.get(editingBar.widgetId)!}
          barInstance={editingBar}
          onSaveBar={(u) => {
            setEditingBar(u);
            api.updateBarInstance(u).catch((e) => pushToast(String(e), "error"));
          }}
          onClose={() => setEditingBar(null)}
        />
      )}

      {/* Toasts */}
      <div className="fixed bottom-4 right-4 z-50 space-y-2">
        {toasts.map((t) => (
          <div
            key={t.id}
            onClick={() => dismissToast(t.id)}
            className={`cursor-pointer rounded-xl border px-4 py-2.5 text-xs font-medium shadow-xl backdrop-blur-md ${
              t.kind === "error"
                ? "border-rose-500/40 bg-rose-950/90 text-rose-200"
                : t.kind === "success"
                ? "border-emerald-500/40 bg-emerald-950/90 text-emerald-200"
                : "border-zinc-700 bg-zinc-900/90 text-zinc-200"
            }`}
          >
            {t.message}
          </div>
        ))}
      </div>
    </div>
  );
};

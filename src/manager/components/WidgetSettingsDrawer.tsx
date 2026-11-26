import React from "react";
import { X } from "lucide-react";
import type {
  BarEdge,
  BarInstance,
  SettingField,
  WidgetInstance,
  WidgetManifest,
} from "../../types";

interface DrawerProps {
  manifest: WidgetManifest;
  widgetInstance?: WidgetInstance;
  barInstance?: BarInstance;
  onSaveWidget?: (updated: WidgetInstance) => void;
  onSaveBar?: (updated: BarInstance) => void;
  onClose: () => void;
}

export const WidgetSettingsDrawer: React.FC<DrawerProps> = ({
  manifest,
  widgetInstance,
  barInstance,
  onSaveWidget,
  onSaveBar,
  onClose,
}) => {
  const settings = widgetInstance?.settings || barInstance?.settings || {};

  const updateField = (key: string, value: unknown) => {
    const nextSettings = { ...settings, [key]: value };
    if (widgetInstance && onSaveWidget) {
      onSaveWidget({ ...widgetInstance, settings: nextSettings });
    } else if (barInstance && onSaveBar) {
      onSaveBar({ ...barInstance, settings: nextSettings });
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-black/50 backdrop-blur-xs">
      <div className="flex h-full w-full max-w-sm flex-col border-l border-zinc-800 bg-zinc-900 p-5 shadow-2xl">
        <div className="flex items-center justify-between border-b border-zinc-800 pb-4">
          <div>
            <h3 className="text-sm font-semibold text-zinc-100">{manifest.name}</h3>
            <p className="text-xs text-zinc-400">Instance Configuration</p>
          </div>
          <button
            onClick={onClose}
            className="rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="mt-4 flex-1 space-y-4 overflow-y-auto pr-1">
          {widgetInstance && onSaveWidget && (
            <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950/50 p-3.5">
              <div className="text-xs font-semibold text-zinc-300">Appearance & Layout</div>
              <label className="block text-xs text-zinc-400">
                Opacity ({Math.round(widgetInstance.opacity * 100)}%)
                <input
                  type="range"
                  min={0.2}
                  max={1}
                  step={0.05}
                  value={widgetInstance.opacity}
                  onChange={(e) =>
                    onSaveWidget({
                      ...widgetInstance,
                      opacity: Number(e.target.value),
                    })
                  }
                  className="mt-1 w-full accent-indigo-500"
                />
              </label>
              <label className="flex items-center justify-between text-xs text-zinc-300">
                <span>Lock position & size</span>
                <input
                  type="checkbox"
                  checked={widgetInstance.locked}
                  onChange={(e) =>
                    onSaveWidget({ ...widgetInstance, locked: e.target.checked })
                  }
                  className="accent-indigo-500"
                />
              </label>
            </div>
          )}

          {barInstance && onSaveBar && (
            <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950/50 p-3.5">
              <div className="text-xs font-semibold text-zinc-300">Taskbar Docking</div>
              <label className="block text-xs text-zinc-400">
                Screen Edge
                <select
                  value={barInstance.edge}
                  onChange={(e) =>
                    onSaveBar({ ...barInstance, edge: e.target.value as BarEdge })
                  }
                  className="mt-1 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-xs text-zinc-100"
                >
                  <option value="bottom">Bottom</option>
                  <option value="top">Top</option>
                  <option value="left">Left</option>
                  <option value="right">Right</option>
                </select>
              </label>
              <label className="flex items-center justify-between text-xs text-zinc-300">
                <span>Reserve desktop work area (AppBar)</span>
                <input
                  type="checkbox"
                  checked={barInstance.reserveSpace}
                  onChange={(e) =>
                    onSaveBar({ ...barInstance, reserveSpace: e.target.checked })
                  }
                  className="accent-indigo-500"
                />
              </label>
              <label className="flex items-center justify-between text-xs text-amber-300">
                <span>Hide native Windows taskbar</span>
                <input
                  type="checkbox"
                  checked={barInstance.hideNativeTaskbar}
                  onChange={(e) =>
                    onSaveBar({
                      ...barInstance,
                      hideNativeTaskbar: e.target.checked,
                    })
                  }
                  className="accent-amber-500"
                />
              </label>
            </div>
          )}

          {manifest.settingsSchema.length > 0 && (
            <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-950/50 p-3.5">
              <div className="text-xs font-semibold text-zinc-300">Widget Settings</div>
              {manifest.settingsSchema.map((field: SettingField) => {
                const val = settings[field.key] ?? field.default;
                if (field.type === "boolean") {
                  return (
                    <label
                      key={field.key}
                      className="flex items-center justify-between text-xs text-zinc-300"
                    >
                      <span>{field.label}</span>
                      <input
                        type="checkbox"
                        checked={Boolean(val)}
                        onChange={(e) => updateField(field.key, e.target.checked)}
                        className="accent-indigo-500"
                      />
                    </label>
                  );
                }
                if (field.type === "select" && field.options) {
                  return (
                    <label key={field.key} className="block text-xs text-zinc-400">
                      {field.label}
                      <select
                        value={String(val ?? "")}
                        onChange={(e) => updateField(field.key, e.target.value)}
                        className="mt-1 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-xs text-zinc-100"
                      >
                        {field.options.map((opt) => (
                          <option key={String(opt.value)} value={String(opt.value)}>
                            {opt.label}
                          </option>
                        ))}
                      </select>
                    </label>
                  );
                }
                if (field.type === "color") {
                  return (
                    <label
                      key={field.key}
                      className="flex items-center justify-between text-xs text-zinc-300"
                    >
                      <span>{field.label}</span>
                      <input
                        type="color"
                        value={String(val || "#6366f1")}
                        onChange={(e) => updateField(field.key, e.target.value)}
                        className="h-7 w-12 cursor-pointer rounded border border-zinc-700 bg-transparent"
                      />
                    </label>
                  );
                }
                if (field.type === "number") {
                  return (
                    <label key={field.key} className="block text-xs text-zinc-400">
                      {field.label}
                      <input
                        type="number"
                        min={field.min}
                        max={field.max}
                        value={Number(val ?? 0)}
                        onChange={(e) => updateField(field.key, Number(e.target.value))}
                        className="mt-1 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-xs text-zinc-100"
                      />
                    </label>
                  );
                }
                return (
                  <label key={field.key} className="block text-xs text-zinc-400">
                    {field.label}
                    <input
                      type="text"
                      value={String(val ?? "")}
                      onChange={(e) => updateField(field.key, e.target.value)}
                      className="mt-1 w-full rounded-lg border border-zinc-700 bg-zinc-900 px-2.5 py-1.5 text-xs text-zinc-100"
                    />
                  </label>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

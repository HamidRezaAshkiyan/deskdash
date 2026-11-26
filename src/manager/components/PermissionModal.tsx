import React from "react";
import { ShieldAlert, ShieldCheck, Globe, X } from "lucide-react";
import type { WidgetManifest, WidgetPermissions } from "../../types";

interface PermissionModalProps {
  manifest: WidgetManifest;
  onConfirm: (granted: WidgetPermissions) => void;
  onCancel: () => void;
}

const CAP_LABELS: Array<{ key: keyof Omit<WidgetPermissions, "network">; label: string; desc: string }> = [
  { key: "system", label: "System Telemetry", desc: "Read CPU, RAM, Disk, and Network stats" },
  { key: "media", label: "Media Controls", desc: "Read Now Playing track info and control playback (SMTC)" },
  { key: "taskbar", label: "Taskbar & Windows", desc: "List, focus, minimize, or close open windows and open Start" },
  { key: "launcher", label: "App Launcher", desc: "Launch installed applications" },
  { key: "storage", label: "Local Storage", desc: "Persist widget notes and state locally" },
  { key: "clipboard", label: "Clipboard", desc: "Read and write system clipboard text" },
  { key: "notifications", label: "Notifications", desc: "Show desktop notifications" },
];

export const PermissionModal: React.FC<PermissionModalProps> = ({
  manifest,
  onConfirm,
  onCancel,
}) => {
  const perms = manifest.permissions;
  const requestedCaps = CAP_LABELS.filter((c) => Boolean(perms[c.key]));
  const domains = perms.network || [];
  const hasAny = requestedCaps.length > 0 || domains.length > 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-indigo-500/15 text-indigo-400">
              {hasAny ? <ShieldAlert className="h-5 w-5" /> : <ShieldCheck className="h-5 w-5 text-emerald-400" />}
            </div>
            <div>
              <h3 className="text-base font-semibold text-zinc-100">
                Review Permissions
              </h3>
              <p className="text-xs text-zinc-400">
                {manifest.name} ({manifest.kind === "bar" ? "Alternative Taskbar" : "Desktop Widget"})
              </p>
            </div>
          </div>
          <button
            onClick={onCancel}
            className="rounded-lg p-1 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="mt-5 space-y-2.5">
          {!hasAny && (
            <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-3 text-xs text-emerald-300">
              This widget requests no system permissions or network access. It runs fully sandboxed.
            </div>
          )}

          {requestedCaps.map((cap) => (
            <div
              key={cap.key}
              className="rounded-xl border border-zinc-800 bg-zinc-950/60 px-3.5 py-2.5"
            >
              <div className="text-xs font-semibold text-zinc-200">{cap.label}</div>
              <div className="text-[11px] text-zinc-400">{cap.desc}</div>
            </div>
          ))}

          {domains.length > 0 && (
            <div className="rounded-xl border border-zinc-800 bg-zinc-950/60 px-3.5 py-2.5">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-zinc-200">
                <Globe className="h-3.5 w-3.5 text-sky-400" />
                Allowed Network Domains
              </div>
              <div className="mt-1.5 flex flex-wrap gap-1.5">
                {domains.map((d) => (
                  <span
                    key={d}
                    className="rounded-md bg-sky-500/15 px-2 py-0.5 font-mono text-[11px] text-sky-300"
                  >
                    {d}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="mt-6 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded-xl border border-zinc-700 px-4 py-2 text-xs font-medium text-zinc-300 hover:bg-zinc-800 cursor-pointer"
          >
            Cancel
          </button>
          <button
            onClick={() => onConfirm(perms)}
            className="rounded-xl bg-indigo-500 px-4 py-2 text-xs font-semibold text-white hover:bg-indigo-400 cursor-pointer"
          >
            Approve & Add
          </button>
        </div>
      </div>
    </div>
  );
};

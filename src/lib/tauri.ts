import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  AppSettings,
  BarEdge,
  BarInstance,
  DdInvokeRequest,
  DdInvokeResponse,
  InstalledWidget,
  MarketplaceItem,
  MonitorInfo,
  PermissionAudit,
  Rect,
  WidgetInstance,
  WidgetManifest,
  WidgetPermissions,
} from "../types";

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  getMonitors: () => invoke<MonitorInfo[]>("get_monitors"),
  listInstalledWidgets: () => invoke<InstalledWidget[]>("list_installed_widgets"),
  auditWidgetPermissions: (widgetId: string) =>
    invoke<PermissionAudit>("audit_widget_permissions", { widgetId }),

  addWidgetInstance: (
    widgetId: string,
    monitorId: string,
    grantedPermissions: WidgetPermissions
  ) =>
    invoke<WidgetInstance>("add_widget_instance", {
      widgetId,
      monitorId,
      grantedPermissions,
    }),

  updateWidgetInstance: (instance: WidgetInstance) =>
    invoke<void>("update_widget_instance", { instance }),

  removeWidgetInstance: (instanceId: string) =>
    invoke<void>("remove_widget_instance", { instanceId }),

  addBarInstance: (
    widgetId: string,
    monitorId: string,
    grantedPermissions: WidgetPermissions,
    edge?: BarEdge,
    hideNativeTaskbar?: boolean
  ) =>
    invoke<BarInstance>("add_bar_instance", {
      widgetId,
      monitorId,
      edge: edge ?? null,
      hideNativeTaskbar: hideNativeTaskbar ?? null,
      grantedPermissions,
    }),

  updateBarInstance: (bar: BarInstance) =>
    invoke<void>("update_bar_instance", { bar }),

  removeBarInstance: (instanceId: string) =>
    invoke<void>("remove_bar_instance", { instanceId }),

  setEditMode: (enabled: boolean) => invoke<void>("set_edit_mode", { enabled }),

  updateSettings: (settings: AppSettings) =>
    invoke<void>("update_settings", { settings }),

  updateInteractiveRects: (windowLabel: string, rects: Rect[]) =>
    invoke<void>("update_interactive_rects", { windowLabel, rects }),

  restoreNativeTaskbar: () => invoke<void>("restore_native_taskbar"),

  openDataDir: () => invoke<string>("open_data_dir"),

  openCanvasPreview: () => invoke<void>("open_canvas_preview"),

  listMarketplace: () => invoke<MarketplaceItem[]>("list_marketplace"),

  installMarketplaceWidget: (downloadUrl: string, sha256?: string) =>
    invoke<WidgetManifest>("install_marketplace_widget", {
      downloadUrl,
      sha256: sha256 ?? null,
    }),

  importLocalWidget: (path: string) =>
    invoke<WidgetManifest>("import_local_widget", { path }),

  uninstallWidget: (widgetId: string) =>
    invoke<void>("uninstall_widget", { widgetId }),

  ddInvoke: (req: DdInvokeRequest) => invoke<DdInvokeResponse>("dd_invoke", { req }),
};

export function widgetIframeUrl(
  widgetId: string,
  entry: string,
  instanceId: string
): string {
  const cleanEntry = entry.replace(/^\/+/, "") || "index.html";
  const isWindows =
    typeof navigator !== "undefined" && /Windows/i.test(navigator.userAgent);
  const base = isWindows
    ? `http://dd-widget.localhost/${widgetId}/${cleanEntry}`
    : `dd-widget://${widgetId}/${cleanEntry}`;
  return `${base}?instanceId=${encodeURIComponent(instanceId)}&widgetId=${encodeURIComponent(widgetId)}`;
}

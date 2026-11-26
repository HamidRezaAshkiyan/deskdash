export type SurfaceKind = "widget" | "bar";
export type BarEdge = "top" | "bottom" | "left" | "right";

export interface Size2D {
  width: number;
  height: number;
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WidgetPermissions {
  system: boolean;
  media: boolean;
  clipboard: boolean;
  taskbar: boolean;
  launcher: boolean;
  storage: boolean;
  notifications: boolean;
  network: string[];
}

export interface SettingOption {
  label: string;
  value: unknown;
}

export interface SettingField {
  key: string;
  label: string;
  type: "string" | "number" | "boolean" | "select" | "color";
  default: unknown;
  description?: string;
  options?: SettingOption[];
  min?: number;
  max?: number;
}

export interface BarConfig {
  edge: BarEdge;
  thickness: number;
  reserveSpace: boolean;
  hideNativeTaskbar: boolean;
}

export interface WidgetManifest {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  category: string;
  kind: SurfaceKind;
  entry: string;
  defaultSize: Size2D;
  minSize?: Size2D;
  maxSize?: Size2D;
  permissions: WidgetPermissions;
  settingsSchema: SettingField[];
  bar?: BarConfig;
}

export interface InstalledWidget {
  manifest: WidgetManifest;
  path: string;
  builtin: boolean;
}

export interface WidgetInstance {
  instanceId: string;
  widgetId: string;
  monitorId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  zIndex: number;
  opacity: number;
  locked: boolean;
  enabled: boolean;
  settings: Record<string, unknown>;
  grantedPermissions: WidgetPermissions;
}

export interface BarInstance {
  instanceId: string;
  widgetId: string;
  monitorId: string;
  edge: BarEdge;
  thickness: number;
  reserveSpace: boolean;
  hideNativeTaskbar: boolean;
  enabled: boolean;
  settings: Record<string, unknown>;
  grantedPermissions: WidgetPermissions;
}

export interface AppSettings {
  gridSnap: number;
  editModeHotkey: string;
  autostart: boolean;
  autoPauseFullscreen: boolean;
  minimizeToTray: boolean;
  registryUrls: string[];
}

export interface AppConfig {
  activeProfile: string;
  editMode: boolean;
  settings: AppSettings;
  widgets: WidgetInstance[];
  bars: BarInstance[];
}

export interface MonitorInfo {
  id: string;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scaleFactor: number;
  isPrimary: boolean;
}

export interface MarketplaceItem {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  category: string;
  kind: SurfaceKind;
  downloadUrl?: string;
  sha256?: string;
  permissions: WidgetPermissions;
  builtin: boolean;
  installed: boolean;
  installedVersion?: string;
}

export interface PermissionAudit {
  widgetId: string;
  widgetName: string;
  requested: WidgetPermissions;
  addedCapabilities: string[];
  addedDomains: string[];
}

export interface DdInvokeRequest {
  instanceId: string;
  method: string;
  params: Record<string, unknown>;
}

export interface DdInvokeResponse {
  ok: boolean;
  data?: unknown;
  error?: string;
}

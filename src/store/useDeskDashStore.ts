import { create } from "zustand";
import { api } from "../lib/tauri";
import type {
  AppConfig,
  InstalledWidget,
  MarketplaceItem,
  MonitorInfo,
} from "../types";

export interface Toast {
  id: number;
  message: string;
  kind: "info" | "success" | "error";
}

interface DeskDashStore {
  config: AppConfig | null;
  monitors: MonitorInfo[];
  installed: InstalledWidget[];
  marketplace: MarketplaceItem[];
  loading: boolean;
  toasts: Toast[];
  refreshAll: () => Promise<void>;
  refreshMarketplace: () => Promise<void>;
  setConfig: (config: AppConfig) => void;
  pushToast: (message: string, kind?: Toast["kind"]) => void;
  dismissToast: (id: number) => void;
}

let toastSeq = 1;

export const useDeskDashStore = create<DeskDashStore>((set, get) => ({
  config: null,
  monitors: [],
  installed: [],
  marketplace: [],
  loading: true,
  toasts: [],

  refreshAll: async () => {
    try {
      const [config, monitors, installed] = await Promise.all([
        api.getConfig(),
        api.getMonitors(),
        api.listInstalledWidgets(),
      ]);
      set({ config, monitors, installed, loading: false });
    } catch (err) {
      set({ loading: false });
      get().pushToast(`Failed to load DeskDash state: ${String(err)}`, "error");
    }
  },

  refreshMarketplace: async () => {
    try {
      const marketplace = await api.listMarketplace();
      set({ marketplace });
    } catch (err) {
      get().pushToast(`Failed to fetch marketplace: ${String(err)}`, "error");
    }
  },

  setConfig: (config) => set({ config }),

  pushToast: (message, kind = "info") => {
    const id = toastSeq++;
    set((s) => ({ toasts: [...s.toasts, { id, message, kind }] }));
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, 3600);
  },

  dismissToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

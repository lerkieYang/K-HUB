import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { AppMode } from "../config/modes";

interface AppState {
  mode: AppMode;
  isConfigured: boolean;
  hubUrl: string | null;
  setMode: (mode: AppMode) => void;
  setConfigured: (configured: boolean) => void;
  setHubUrl: (url: string | null) => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      mode: "standalone",
      isConfigured: true,
      hubUrl: null,
      setMode: (mode) => set({ mode }),
      setConfigured: (isConfigured) => set({ isConfigured }),
      setHubUrl: (hubUrl) => set({ hubUrl }),
    }),
    {
      name: "knowledgehub-app-store",
    }
  )
);

// 动态获取后端URL：优先用用户配置，否则用当前hostname
export function getBackendUrl(hubUrl: string | null): string {
  if (hubUrl) return hubUrl;
  // 浏览器在Windows运行，后端也在Windows，用127.0.0.1
  return 'http://127.0.0.1:8443';
}

export type AppMode = 'standalone' | 'hub' | 'client';

export interface AppConfig {
  appVersion: string;
  mode: AppMode;
  workspaceId?: string;
  deviceId?: string;
  hubUrl?: string;
  mcpUrl?: string;
  dataDir: string;
  services: {
    hub: { enabled: boolean; port: number };
    mcp: { enabled: boolean; port: number };
    collector: { enabled: boolean };
  };
}

export interface CollectorConfig {
  deviceId: string;
  hubUrl: string;
  pendingDb: string;
  sources: CollectorSource[];
}

export interface CollectorSource {
  id: string;
  name: string;
  path: string;
  sourceType: string;
  recursive: boolean;
  includeGlobs: string[];
  excludeGlobs: string[];
  scanMode: string;
  debounceSeconds: number;
}

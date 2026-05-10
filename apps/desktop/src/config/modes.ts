export type AppMode = "standalone" | "hub" | "client";

export interface ModeConfig {
  id: AppMode;
  label: string;
  description: string;
  features: string[];
}

export const MODES: Record<AppMode, ModeConfig> = {
  standalone: {
    id: "standalone",
    label: "Standalone",
    description: "Run everything locally on a single device",
    features: ["local-storage", "local-agents", "local-memory"],
  },
  hub: {
    id: "hub",
    label: "Hub",
    description: "Act as a central hub, managing devices and distributing tasks",
    features: [
      "local-storage",
      "local-agents",
      "local-memory",
      "device-management",
      "task-distribution",
      "agent-discovery",
    ],
  },
  client: {
    id: "client",
    label: "Client",
    description: "Connect to a remote hub as a lightweight client",
    features: ["remote-storage", "remote-agents", "remote-memory"],
  },
};

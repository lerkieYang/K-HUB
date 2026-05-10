export interface ApiResponse<T> {
  data?: T;
  error?: string;
  message?: string;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  perPage: number;
}

export interface ContextRequest {
  agentId: string;
  deviceId?: string;
  task: string;
  need?: string[];
  maxTokens?: number;
}

export interface ContextResponse {
  requestId: string;
  contextPack: {
    relevantMemory: any[];
    relevantDocs: any[];
    templates: any[];
    rules: string[];
    warnings: string[];
  };
  sources: string[];
  confidence: number;
  expiresAt: string;
}

export interface FileEventBatch {
  deviceId: string;
  events: FileEvent[];
}

export interface FileEvent {
  eventId: string;
  dataSourceId: string;
  eventType: string;
  path: string;
  sizeBytes?: number;
  mtime?: string;
  sha256?: string;
}

export interface SyncPushRequest {
  deviceId: string;
  memories?: any[];
  artifacts?: any[];
  fileEvents?: any[];
}

export interface SyncPullRequest {
  deviceId: string;
  since?: string;
}

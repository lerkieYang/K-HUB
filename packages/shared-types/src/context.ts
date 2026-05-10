// Context request/response types
export interface ContextRequest {
  query: string;
  workspaceId: string;
  deviceId: string;
  maxResults?: number;
  filters?: ContextFilters;
}

export interface ContextFilters {
  fileTypes?: string[];
  dateFrom?: string;
  dateTo?: string;
  tags?: string[];
  source?: string;
  minImportance?: number;
}

export interface ContextResponse {
  results: ContextResult[];
  totalCount: number;
  queryTimeMs: number;
}

export interface ContextResult {
  id: string;
  content: string;
  sourcePath?: string;
  relevanceScore: number;
  type: ContextResultType;
  metadata?: Record<string, unknown>;
}

export enum ContextResultType {
  Memory = 'memory',
  Artifact = 'artifact',
  Chunk = 'chunk',
}

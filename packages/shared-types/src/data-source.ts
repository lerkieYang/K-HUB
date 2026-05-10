export interface DataSource {
  id: string;
  workspaceId: string;
  deviceId: string;
  name: string;
  path: string;
  sourceType: 'local' | 'smb' | 'sync_folder' | 'agent_output';
  recursive: boolean;
  includeGlobs: string[];
  excludeGlobs: string[];
  scanMode: 'watch' | 'scheduled' | 'watch_and_scan';
  sensitivity: string;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateDataSourceRequest {
  name: string;
  path: string;
  sourceType: string;
  recursive?: boolean;
  includeGlobs?: string[];
  excludeGlobs?: string[];
  scanMode?: string;
  sensitivity?: string;
}

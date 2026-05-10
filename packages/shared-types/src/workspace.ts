export interface Workspace {
  id: string;
  name: string;
  mode: 'standalone' | 'hub';
  createdAt: string;
  updatedAt: string;
  encryptionKeyVersion: number;
}

export interface CreateWorkspaceRequest {
  name: string;
  mode: 'standalone' | 'hub';
}

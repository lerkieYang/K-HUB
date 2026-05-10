export interface Artifact {
  id: string;
  workspaceId: string;
  agentId?: string;
  deviceId?: string;
  title: string;
  artifactType: string;
  content?: string;
  filePath?: string;
  mimeType?: string;
  sha256?: string;
  status: 'submitted' | 'indexed' | 'archived' | 'rejected';
  createdAt: string;
  updatedAt: string;
}

export interface CreateArtifactRequest {
  agentId: string;
  deviceId?: string;
  title: string;
  artifactType: string;
  content?: string;
  mimeType?: string;
  sha256?: string;
}

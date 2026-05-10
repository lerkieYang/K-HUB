export interface Memory {
  id: string;
  workspaceId: string;
  scope: 'global' | 'project' | 'agent' | 'device' | 'session';
  type: 'preference' | 'semantic' | 'procedural' | 'decision' | 'episodic' | 'artifact' | 'warning' | 'case';
  content?: string;
  source: string;
  sourceId?: string;
  agentId?: string;
  deviceId?: string;
  confidence: number;
  visibility: 'private' | 'agent' | 'project' | 'shared';
  status: 'active' | 'deprecated' | 'archived' | 'conflict';
  expiresAt?: string;
  lastUsedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateMemoryRequest {
  scope: string;
  type: string;
  content: string;
  source: string;
  agentId?: string;
  confidence?: number;
  visibility?: string;
}

export interface MemorySearchParams {
  query?: string;
  scope?: string;
  type?: string;
  agentId?: string;
  status?: string;
  limit?: number;
}

export interface MemoryCandidate {
  id: string;
  workspaceId: string;
  proposedScope?: string;
  proposedType?: string;
  content?: string;
  reason?: string;
  sourceAgentId?: string;
  sourceDeviceId?: string;
  sourceArtifactId?: string;
  confidence: number;
  reviewStatus: 'candidate' | 'needs_review' | 'approved' | 'rejected' | 'merged' | 'auto_rejected';
  reviewNotes?: string;
  createdAt: string;
  reviewedAt?: string;
}

export interface SubmitCandidateRequest {
  agentId: string;
  deviceId?: string;
  taskId?: string;
  candidates: CandidateItem[];
}

export interface CandidateItem {
  scope: string;
  type: string;
  content: string;
  reason?: string;
  confidence?: number;
}

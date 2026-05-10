export interface Device {
  id: string;
  workspaceId: string;
  name: string;
  role: 'hub' | 'client' | 'standalone';
  status: 'pending' | 'approved' | 'online' | 'offline' | 'suspended' | 'revoked';
  publicKey?: string;
  fingerprint?: string;
  os?: string;
  appVersion?: string;
  lastSeenAt?: string;
  createdAt: string;
  revokedAt?: string;
}

export interface CreateDeviceRequest {
  inviteCode: string;
  deviceName: string;
  os: string;
  appVersion: string;
  publicKey?: string;
  fingerprint?: string;
}

export interface CreateInviteRequest {
  ttlSeconds?: number;
  allowedRole?: string;
  scopes?: string[];
}

export interface InviteResponse {
  inviteId: string;
  inviteCode: string;
  expiresAt: string;
  qrPayload: string;
}

export interface HeartbeatRequest {
  deviceId: string;
  appVersion: string;
  collectorStatus: string;
  pendingUploadCount: number;
  watchedSourceCount: number;
}

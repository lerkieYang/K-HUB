export interface Agent {
  agentId: string;
  name: string;
  path: string;
  hasConfig: boolean;
  hasSkillDir: boolean;
  status: 'detected' | 'configured' | 'unknown';
}

export interface ConfigureAgentRequest {
  agentId: string;
  injectMcp?: boolean;
  installSkill?: boolean;
  createOutputDir?: boolean;
  permissions?: string[];
}

export interface AgentConfig {
  agentId: string;
  token: string;
  permissions: string[];
  configuredAt: string;
}

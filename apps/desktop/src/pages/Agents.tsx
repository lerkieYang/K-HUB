import React, { useState, useEffect } from 'react';
import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';

interface AgentConfig {
  id: string;
  name: string;
  type: string;
  configured: boolean;
  config_path?: string;
  environment?: string;
  nickname?: string;
}

const AGENT_ICONS: Record<string, string> = {
  Hermes: '🤖', Codex: '💻', Gemini: '✨', OpenClaw: '🐾',
  OpenCode: '📝', ClaudeCode: '🧠', Aider: '🔧', Cursor: '🖱️',
  Windsurf: '🌊', Continue: '♻️',
};

const RESTART_INSTRUCTIONS: Record<string, string> = {
  hermes: 'hermes gateway restart',
  codex: 'agents.restartCodex',
  openclaw: 'openclaw restart',
  'claude-code': 'claude code restart',
  opencode: 'opencode restart',
};

export function Agents() {
  const { hubUrl } = useAppStore();
  const API_BASE = getBackendUrl(hubUrl);
  const { t } = useI18n();
  const toast = useToast();

  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [scanning, setScanning] = useState(false);
  const [configuringId, setConfiguringId] = useState<string | null>(null);
  const [unconfiguringId, setUnconfiguringId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [promptModal, setPromptModal] = useState<{ agentId: string; prompt: string } | null>(null);
  const [configModal, setConfigModal] = useState(false);
  const [hubStatus, setHubStatus] = useState<'idle' | 'testing' | 'ok' | 'fail'>('idle');
  const [hubDetail, setHubDetail] = useState('');
  const [agentLastSeen, setAgentLastSeen] = useState<Record<string, string>>({});
  const [nicknames, setNicknames] = useState<Record<string, string>>(() => {
    try {
      const saved = localStorage.getItem('khub-agent-nicknames');
      return saved ? JSON.parse(saved) : {};
    } catch { return {}; }
  });
  const [editingNickname, setEditingNickname] = useState<string | null>(null);
  const [nicknameInput, setNicknameInput] = useState('');
  const [nicknameError, setNicknameError] = useState<string | null>(null);

  useEffect(() => {
    fetchAgents();
    fetchAgentActivity();
    const interval = setInterval(fetchAgentActivity, 10000);
    return () => clearInterval(interval);
  }, []);

  const fetchAgentActivity = async () => {
    try {
      const res = await fetch(API_BASE + '/mcp/agent-activity');
      if (res.ok) {
        const data = await res.json();
        const m: Record<string, string> = {};
        if (data.agents) for (const item of data.agents) m[item.agent_id] = item.last_seen;
        setAgentLastSeen(m);
      }
    } catch {}
  };

  const fetchAgents = async () => {
    setScanning(true);
    setError(null);
    try {
      const res = await fetch(API_BASE + '/api/agents/scan', { method: 'POST' });
      if (!res.ok) throw new Error('HTTP ' + res.status);
      const data = await res.json();
      setAgents(data.agents || []);
    } catch (err: any) {
      setError(t('agents.scanFailed', { error: err.message }));
    } finally {
      setScanning(false);
    }
  };

  const handleConfigureAgent = async (agentId: string) => {
    setConfiguringId(agentId);
    try {
      const res = await fetch(API_BASE + '/api/agents/configure', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ agent_id: agentId }),
      });
      const data = await res.json();
      if (data.success) {
        const restartCmd = RESTART_INSTRUCTIONS[agentId];
        let message = t('agents.configureSuccess', { id: agentId });
        if (restartCmd) {
          const cmdText = restartCmd.startsWith('agents.') ? t(restartCmd) : restartCmd;
          message += '\n\n' + t('agents.restartHint') + '\n' + cmdText;
        }
        toast.success(message);
        fetchAgents();
      } else {
        toast.error(t('agents.configureFailed', { error: data.error }));
      }
    } catch (err: any) {
      toast.error(t('agents.configureFailed', { error: err.message }));
    } finally {
      setConfiguringId(null);
    }
  };

  const handleUnconfigureAgent = async (agentId: string) => {
    if (!await toast.confirm(t('agents.unconfigureConfirm', { id: agentId }))) return;
    setUnconfiguringId(agentId);
    try {
      const res = await fetch(API_BASE + '/api/agents/unconfigure', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ agent_id: agentId }),
      });
      const data = await res.json();
      if (data.success) {
        toast.success(t('agents.unconfigureSuccess', { id: agentId }));
        fetchAgents();
      } else {
        toast.error(t('agents.unconfigureFailed', { error: data.error }));
      }
    } catch (err: any) {
      toast.error(t('agents.unconfigureFailed', { error: err.message }));
    } finally {
      setUnconfiguringId(null);
    }
  };

  const handleShowPrompt = async (agentId: string) => {
    try {
      const res = await fetch(API_BASE + '/api/agents/' + agentId + '/config-prompt?hub_url=http://127.0.0.1:8443');
      const data = await res.json();
      setPromptModal({ agentId, prompt: data.prompt });
    } catch (err: any) {
      toast.error(t('agents.promptFailed', { error: err.message }));
    }
  };

  const testHubConnectivity = async () => {
    setHubStatus('testing');
    setHubDetail('');
    try {
      const res = await fetch(API_BASE + '/api/agents/test-connection', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      });
      const data = await res.json();
      if (data.all_ok) {
        setHubStatus('ok');
        setHubDetail(t('agents.khubTools', { version: 'v' + (data.mcp?.version || '?'), count: data.tools?.count || 0 }));
      } else {
        setHubStatus('fail');
        const issues = [];
        if (!data.health?.ok) issues.push(t('agents.healthCheckFailed'));
        if (!data.mcp?.ok) issues.push(t('agents.mcpProtocolError'));
        if (!data.tools?.ok) issues.push(t('agents.toolListError'));
        setHubDetail(issues.join('，') || t('dashboard.connectionFailed'));
      }
    } catch (err: any) {
      setHubStatus('fail');
      setHubDetail(err.message || t('agents.cannotConnect'));
    }
  };

  const copyToClipboard = (text: string) => {
    // navigator.clipboard 需要 HTTPS 或 localhost，IP 访问时用 fallback
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(() => toast.success(t('agents.copied')));
    } else {
      const textarea = document.createElement('textarea');
      textarea.value = text;
      textarea.style.position = 'fixed';
      textarea.style.left = '-9999px';
      document.body.appendChild(textarea);
      textarea.select();
      try {
        document.execCommand('copy');
        toast.success(t('agents.copied'));
      } catch {
        toast.error('复制失败，请手动复制');
      }
      document.body.removeChild(textarea);
    }
  };

  const saveNickname = (agentId: string) => {
    const name = nicknameInput.trim();
    setNicknameError(null);
    if (!name) {
      const newNicknames = { ...nicknames };
      delete newNicknames[agentId];
      setNicknames(newNicknames);
      localStorage.setItem('khub-agent-nicknames', JSON.stringify(newNicknames));
      setEditingNickname(null);
      return;
    }
    const duplicateId = Object.entries(nicknames).find(([id, n]) => n === name && id !== agentId)?.[0];
    if (duplicateId) {
      setNicknameError(t('agents.nicknameDuplicate', { name }));
      return;
    }
    const newNicknames = { ...nicknames, [agentId]: name };
    setNicknames(newNicknames);
    localStorage.setItem('khub-agent-nicknames', JSON.stringify(newNicknames));
    setEditingNickname(null);
  };

  const getAgentIcon = (type: string) => AGENT_ICONS[type] || '❓';
  const windowsAgents = agents.filter(a => a.environment === 'windows');
  const wslAgents = agents.filter(a => a.environment === 'wsl');

  return (
    <div className="py-6">
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-gray-900 flex items-center gap-3">🤖 {t('agents.title')}</h1>
      </div>

      {error && (
        <div className="mb-6 p-4 bg-red-50 border border-red-200 rounded-xl text-red-700 text-sm flex items-center gap-2">
          <span>⚠️</span> {error}
        </div>
      )}

      <div className="flex items-center gap-3 mb-6 flex-wrap">
        <button onClick={fetchAgents} disabled={scanning}
          className="inline-flex items-center gap-2 px-5 py-2.5 bg-indigo-600 text-white rounded-xl hover:bg-indigo-700 disabled:opacity-50 transition-colors font-medium shadow-sm">
          {scanning ? (<><svg className="animate-spin h-4 w-4" viewBox="0 0 24 24"><circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" /><path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" /></svg>{t('agents.scanning')}</>) : (<>{t('agents.scan')}</>)}
        </button>
        <button onClick={testHubConnectivity} disabled={hubStatus === 'testing'}
          className={"inline-flex items-center gap-2 px-5 py-2.5 rounded-xl font-medium shadow-sm transition-colors disabled:opacity-50 " + (hubStatus === 'ok' ? 'bg-emerald-600 text-white hover:bg-emerald-700' : hubStatus === 'fail' ? 'bg-red-600 text-white hover:bg-red-700' : 'bg-gray-600 text-white hover:bg-gray-700')}>
          {hubStatus === 'testing' ? t('agents.hubTesting') : hubStatus === 'ok' ? t('agents.hubConnected') : hubStatus === 'fail' ? t('agents.hubFailed') : t('agents.testConnection')}
        </button>
        <button onClick={() => setConfigModal(true)}
          className="inline-flex items-center gap-2 px-5 py-2.5 bg-gray-600 text-white rounded-xl hover:bg-gray-700 font-medium shadow-sm transition-colors">
          ⚙️ {t('agents.addConfigParams', { default: '添加配置参数' })}
        </button>
        {hubDetail && (<span className={"text-sm " + (hubStatus === 'ok' ? 'text-emerald-600' : 'text-red-500')}>{hubDetail}</span>)}
      </div>

      <div className="mb-6 p-3 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700">{t('agents.connectionHint')}</div>

      {agents.length === 0 && !scanning && (
        <div className="text-center py-16 bg-gray-50 rounded-2xl border-2 border-dashed border-gray-200">
          <span className="text-5xl mb-4 block">🔍</span>
          <p className="text-gray-500 text-lg">{t('agents.noAgents')}</p>
          <p className="text-gray-400 text-sm mt-1">{t('agents.noAgentsHint')}</p>
        </div>
      )}

      {windowsAgents.length > 0 && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold text-gray-700 mb-3 flex items-center gap-2">
            <span>🪟</span> {t('agents.windows')}
            <span className="text-sm font-normal text-gray-400">({windowsAgents.length})</span>
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {windowsAgents.map(agent => (
              <AgentCard key={agent.id + '-' + agent.environment} agent={agent} icon={getAgentIcon(agent.type)}
                lastSeen={agentLastSeen[agent.id]} nickname={nicknames[agent.id + '-' + agent.environment]}
                editingNickname={editingNickname} nicknameInput={nicknameInput} nicknameError={nicknameError}
                onStartEditNickname={(id, cur) => { setEditingNickname(id); setNicknameInput(cur || ''); setNicknameError(null); }}
                onSaveNickname={saveNickname} onCancelEditNickname={() => { setEditingNickname(null); setNicknameError(null); }}
                onNicknameInputChange={setNicknameInput} onUnconfigure={handleUnconfigureAgent}
                onShowPrompt={handleShowPrompt} unconfiguring={unconfiguringId === agent.id} />
            ))}
          </div>
        </div>
      )}

      {wslAgents.length > 0 && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold text-gray-700 mb-3 flex items-center gap-2">
            <span>🐧</span> {t('agents.wsl')}
            <span className="text-sm font-normal text-gray-400">({wslAgents.length})</span>
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {wslAgents.map(agent => (
              <AgentCard key={agent.id + '-' + agent.environment} agent={agent} icon={getAgentIcon(agent.type)}
                lastSeen={agentLastSeen[agent.id]} nickname={nicknames[agent.id + '-' + agent.environment]}
                editingNickname={editingNickname} nicknameInput={nicknameInput} nicknameError={nicknameError}
                onStartEditNickname={(id, cur) => { setEditingNickname(id); setNicknameInput(cur || ''); setNicknameError(null); }}
                onSaveNickname={saveNickname} onCancelEditNickname={() => { setEditingNickname(null); setNicknameError(null); }}
                onNicknameInputChange={setNicknameInput} onUnconfigure={handleUnconfigureAgent}
                onShowPrompt={handleShowPrompt} unconfiguring={unconfiguringId === agent.id} />
            ))}
          </div>
        </div>
      )}

      <div className="mt-8 p-5 bg-indigo-50 rounded-2xl border border-indigo-100">
        <h3 className="font-semibold text-indigo-900 mb-2">💡 {t('agents.help')}</h3>
        <ul className="text-sm text-indigo-700 space-y-1.5">
          <li dangerouslySetInnerHTML={{ __html: '• ' + t('agents.help.configure') }} />
          <li dangerouslySetInnerHTML={{ __html: '• ' + t('agents.help.promptInstruction') }} />
          <li dangerouslySetInnerHTML={{ __html: '• ' + t('agents.help.restartInstruction') }} />
          <li dangerouslySetInnerHTML={{ __html: '• ' + t('agents.help.mcpInstruction') }} />
          <li dangerouslySetInnerHTML={{ __html: '• ' + t('agents.help.testInstruction') }} />
        </ul>
      </div>

      {/* Config Modal */}
      {configModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4" onClick={() => setConfigModal(false)}>
          <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full overflow-hidden" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 border-b">
              <h3 className="text-lg font-semibold">⚙️ MCP 配置参数</h3>
              <button onClick={() => setConfigModal(false)} className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg">✕</button>
            </div>
            <div className="p-5 space-y-4">
              <div><label className="block text-sm font-medium text-gray-700 mb-1">服务名称</label><input type="text" value="k-hub" readOnly className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm bg-gray-50" /></div>
              <div><label className="block text-sm font-medium text-gray-700 mb-1">描述</label><input type="text" value="K-HUB Knowledge Hub MCP Server" readOnly className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm bg-gray-50" /></div>
              <div><label className="block text-sm font-medium text-gray-700 mb-1">传输类型</label><select disabled className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm bg-gray-50"><option>Server-Sent Events (SSE)</option></select></div>
              <div><label className="block text-sm font-medium text-gray-700 mb-1">服务 URL</label><input type="text" value="http://127.0.0.1:8443/mcp" readOnly className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm bg-gray-50 font-mono" /></div>
            </div>
            <div className="flex gap-2 justify-end p-4 border-t bg-gray-50">
              <button onClick={() => {
                const dynamicUrl = 'http://127.0.0.1:8443/mcp';
                const config = JSON.stringify({ mcpServers: { "k-hub": { url: dynamicUrl, type: "sse" } } }, null, 2);
                copyToClipboard(config);
              }} className="px-4 py-2 bg-indigo-600 text-white rounded-lg text-sm hover:bg-indigo-700">{t('agents.copy')}</button>
              <button onClick={() => setConfigModal(false)} className="px-4 py-2 bg-gray-100 rounded-lg text-sm">{t('common.close')}</button>
            </div>
          </div>
        </div>
      )}

      {/* Prompt Modal */}
      {promptModal && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4" onClick={() => setPromptModal(null)}>
          <div className="bg-white rounded-2xl shadow-2xl max-w-2xl w-full max-h-[80vh] overflow-hidden" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between p-5 border-b">
              <h3 className="text-lg font-semibold">📝 {promptModal.agentId} {t('agents.promptTitle')}</h3>
              <div className="flex items-center gap-2">
                <button onClick={() => copyToClipboard(promptModal.prompt)} className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-lg hover:bg-indigo-700">{t('agents.copy')}</button>
                <button onClick={() => setPromptModal(null)} className="p-1.5 text-gray-400 hover:text-gray-600 rounded-lg">✕</button>
              </div>
            </div>
            <div className="p-5 overflow-y-auto max-h-[60vh]">
              <pre className="whitespace-pre-wrap text-sm text-gray-700 bg-gray-50 p-4 rounded-xl font-mono leading-relaxed">{promptModal.prompt}</pre>
            </div>
            <div className="p-4 border-t bg-gray-50 text-xs text-gray-500">{t('agents.promptFooter')}</div>
          </div>
        </div>
      )}
    </div>
  );
}

function AgentCard({ agent, icon, lastSeen, nickname, editingNickname, nicknameInput, nicknameError,
  onStartEditNickname, onSaveNickname, onCancelEditNickname, onNicknameInputChange, onUnconfigure, onShowPrompt, unconfiguring,
}: {
  agent: AgentConfig; icon: string; lastSeen?: string; nickname?: string;
  editingNickname: string | null; nicknameInput: string; nicknameError: string | null;
  onStartEditNickname: (id: string, current?: string) => void; onSaveNickname: (id: string) => void;
  onCancelEditNickname: () => void; onNicknameInputChange: (value: string) => void;
  onUnconfigure: (id: string) => void; onShowPrompt: (id: string) => void; unconfiguring: boolean;
}) {
  const { t } = useI18n();
  const toast = useToast();
  const compositeKey = agent.id + '-' + agent.environment;
  const isEditing = editingNickname === compositeKey;

  return (
    <div className="bg-white rounded-2xl border border-gray-200 shadow-sm hover:shadow-md transition-shadow p-5 flex flex-col">
      {/* Row 1: Icon + Agent type + nickname button */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2 min-w-0">
          <div className="w-10 h-10 rounded-xl bg-gray-100 flex items-center justify-center text-xl flex-shrink-0">{icon}</div>
          <div className="min-w-0">
            <h3 className="font-semibold text-gray-900 truncate">{agent.type || agent.name || 'Unknown Agent'}</h3>
            {nickname && <p className="text-xs text-indigo-500 truncate">{nickname}</p>}
          </div>
        </div>
        {isEditing ? (
          <div className="flex items-center gap-1 flex-shrink-0">
            <input type="text" value={nicknameInput} onChange={e => onNicknameInputChange(e.target.value)}
              onKeyDown={e => { if (e.key === 'Enter') onSaveNickname(compositeKey); if (e.key === 'Escape') onCancelEditNickname(); }}
              placeholder={agent.name} className="w-24 px-2 py-0.5 text-xs border border-gray-300 rounded focus:outline-none focus:border-indigo-500" autoFocus maxLength={20} />
            <button onClick={() => onSaveNickname(compositeKey)} className="px-1 py-0.5 text-xs bg-indigo-600 text-white rounded hover:bg-indigo-700">✓</button>
            <button onClick={onCancelEditNickname} className="px-1 py-0.5 text-xs text-gray-500 hover:text-gray-700">✕</button>
          </div>
        ) : (
          <button onClick={() => onStartEditNickname(compositeKey, nickname)} className="text-gray-400 hover:text-indigo-600 transition-colors flex-shrink-0" title={t('agents.editNickname')}>
            ✏️
          </button>
        )}
      </div>
      {isEditing && nicknameError && <p className="text-xs text-red-500 -mt-2 mb-2">{nicknameError}</p>}

      {/* Row 2: Badges */}
      <div className="flex items-center gap-1.5 mb-3 flex-wrap h-[22px]">
        {agent.configured && (<span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-700 rounded-full whitespace-nowrap">✓ {t('agents.configured')}</span>)}
        {lastSeen ? (<span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-emerald-100 text-emerald-700 rounded-full whitespace-nowrap">🟢 {t('agents.connected')}</span>)
          : agent.configured ? (<span className="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-amber-100 text-amber-600 rounded-full whitespace-nowrap">⏳ {t('agents.waiting')}</span>) : null}
      </div>

      {/* Row 3: Config path — with spacing matching badge height */}
      <div className="mb-2 min-h-[28px]">
        {lastSeen && <p className="text-xs text-emerald-600 leading-5">{t('agents.lastActive')}: {lastSeen}</p>}
        <p className="text-xs text-gray-400 font-mono truncate leading-5" title={agent.config_path || ''}>
          {agent.config_path || '\u00A0'}
        </p>
      </div>

      {/* Row 4: Action buttons */}
      <div className="flex items-center gap-2 pt-2 border-t border-gray-100 mt-auto">
        <button onClick={() => onShowPrompt(agent.id)} className="flex-1 inline-flex items-center justify-center gap-1.5 px-3 py-2 text-sm font-medium border border-indigo-300 text-indigo-600 rounded-xl hover:bg-indigo-50 transition-colors" title={t('agents.helpPrompt')}>
          📝 {t('agents.prompt')}
        </button>
        {agent.configured ? (
          <button onClick={() => onUnconfigure(agent.id)} disabled={unconfiguring} className="inline-flex items-center justify-center gap-1.5 px-3 py-2 text-sm font-medium border border-gray-300 text-gray-600 rounded-xl hover:bg-gray-50 disabled:opacity-50 transition-colors">
            {unconfiguring ? '...' : t('agents.unconfigure')}
          </button>
        ) : (
          <div className="w-[90px]" />
        )}
      </div>
    </div>
  );
}

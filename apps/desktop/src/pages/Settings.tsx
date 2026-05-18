import React, { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';
import { invoke } from '@tauri-apps/api/core';

interface KnowledgeBaseConfig {
  id: string;
  name: string;
  paths: string[];
  file_patterns: string[];
  auto_index: boolean;
  auto_embed: boolean;
  last_indexed_at?: string;
}

interface ScheduledTask {
  id: string;
  label: string;
  description: string;
  enabled: boolean;
  time: string;
  nextRun?: string;
  lastRun?: string;
}

interface DataStats {
  memory_total: number;
  memory_active: number;
  memory_deleted: number;
  memory_archived: number;
  memory_candidate: number;
  memory_count: number;
  session_count: number;
  knowledge_total: number;
  artifacts: number;
  kb_configs: number;
  db_size: string;
  cache_size: string;
  embedding_size: string;
  total_vectorized: number;
  memory_vectorized: number;
  session_vectorized: number;
  knowledge_vectorized: number;
  all_deleted: number;
  all_archived: number;
}

const AI_PROVIDERS = [
  { id: 'openai', name: 'OpenAI', base_url: 'https://api.openai.com/v1' },
  { id: 'anthropic', name: 'Anthropic', base_url: 'https://api.anthropic.com' },
  { id: 'google', name: 'Google Gemini', base_url: 'https://generativelanguage.googleapis.com/v1beta' },
  { id: 'deepseek', name: 'DeepSeek', base_url: 'https://api.deepseek.com/v1' },
  { id: 'qwen', nameKey: 'settings.ai.providerQwen', base_url: 'https://dashscope.aliyuncs.com/compatible-mode/v1' },
  { id: 'zhipu', nameKey: 'settings.ai.providerZhipu', base_url: 'https://open.bigmodel.cn/api/paas/v4' },
  { id: 'moonshot', name: 'Moonshot', base_url: 'https://api.moonshot.cn/v1' },
  { id: 'baichuan', nameKey: 'settings.ai.providerBaichuan', base_url: 'https://api.baichuan-ai.com/v1' },
  { id: 'yi', nameKey: 'settings.ai.providerYi', base_url: 'https://api.lingyiwanwu.com/v1' },
  { id: 'spark', nameKey: 'settings.ai.providerSpark', base_url: 'https://spark-api-open.xf-yun.com/v1' },
  { id: 'doubao', nameKey: 'settings.ai.providerDoubao', base_url: 'https://ark.cn-beijing.volces.com/api/v3' },
  { id: 'mimo', name: 'MiMo', base_url: 'https://token-plan-sgp.xiaomimimo.com/v1' },
  { id: 'minimax', name: 'MiniMax', base_url: 'https://api.minimax.chat/v1' },
  { id: 'ollama', nameKey: 'settings.ai.providerOllama', base_url: 'http://localhost:11434/v1' },
  { id: 'custom', nameKey: 'settings.ai.providerCustom', base_url: '' },
];

const RESTART_COMMANDS: Record<string, string> = {
  hermes: 'hermes gateway restart',
  codex: 'agents.restartCodex',
  openclaw: 'openclaw restart',
  'claude-code': 'claude code restart',
  opencode: 'opencode restart',
};

const DEFAULT_TASKS: ScheduledTask[] = [
  { id: 'knowledge_index', label: 'settings.schedule.knowledgeIndex', description: 'settings.schedule.knowledgeIndexDesc', enabled: false, time: '02:00' },
  { id: 'memory_collect', label: 'settings.schedule.memoryCollect', description: 'settings.schedule.memoryCollectDesc', enabled: false, time: '03:00' },
  { id: 'session_pull', label: 'settings.schedule.sessionPull', description: 'settings.schedule.sessionPullDesc', enabled: false, time: '03:30' },
  { id: 'vectorize', label: 'settings.schedule.vectorize', description: 'settings.schedule.vectorizeDesc', enabled: false, time: '08:00' },
  { id: 'memory_vectorize', label: 'settings.schedule.memoryVectorize', description: 'settings.schedule.memoryVectorizeDesc', enabled: false, time: '04:00' },
  { id: 'session_vectorize', label: 'settings.schedule.sessionVectorize', description: 'settings.schedule.sessionVectorizeDesc', enabled: false, time: '04:30' },
];

function computeNextRun(time: string): string {
  const [hours, minutes] = time.split(':').map(Number);
  const now = new Date();
  const next = new Date(now);
  next.setHours(hours, minutes, 0, 0);
  if (next <= now) {
    next.setDate(next.getDate() + 1);
  }
  return next.toISOString();
}

export function Settings() {
  const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const API_BASE = getBackendUrl(hubUrl);

  const [searchParams, setSearchParams] = useSearchParams();
  const tabFromUrl = searchParams.get('tab') as 'general' | 'knowledge' | 'ai' | 'schedule' | 'data' | 'about' | null;
  const validTabs = ['general', 'knowledge', 'ai', 'schedule', 'data', 'about'] as const;
  const initialTab = tabFromUrl && validTabs.includes(tabFromUrl) ? tabFromUrl : 'general';
  const [activeTab, setActiveTab] = useState<'general' | 'knowledge' | 'ai' | 'schedule' | 'data' | 'about'>(initialTab);

  const handleTabChange = (tab: 'general' | 'knowledge' | 'ai' | 'schedule' | 'data' | 'about') => {
    setActiveTab(tab);
    setSearchParams({ tab }, { replace: true });
  };

  // General settings
  const [minimizeToTray, setMinimizeToTray] = useState(true);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [autoBackup, setAutoBackup] = useState(false);
  const [backupInterval, setBackupInterval] = useState(24);
  const [backupDir, setBackupDir] = useState('');
  const [backupList, setBackupList] = useState<string[]>([]);
  const [isBackingUp, setIsBackingUp] = useState(false);
  const [appVersion, setAppVersion] = useState('');
  const [dataDir, setDataDir] = useState('');

  // Knowledge base config
  const [knowledgeConfigs, setKnowledgeConfigs] = useState<KnowledgeBaseConfig[]>([]);
  const [newPath, setNewPath] = useState('');
  const [newName, setNewName] = useState('');
  const [kbLoading, setKbLoading] = useState(false);
  const [kbError, setKbError] = useState('');
  const [showFolderBrowser, setShowFolderBrowser] = useState(false);
  const [addingPath, setAddingPath] = useState(false);

  // AI config - Embedding
  const [embeddingProvider, setEmbeddingProvider] = useState('openai');
  const [embeddingModel, setEmbeddingModel] = useState('');
  const [embeddingApiKey, setEmbeddingApiKey] = useState('');
  const [embeddingBaseUrl, setEmbeddingBaseUrl] = useState('');
  const [embeddingFetchedModels, setEmbeddingFetchedModels] = useState<string[]>([]);
  const [embeddingFetchingModels, setEmbeddingFetchingModels] = useState(false);
  const [embeddingTestResult, setEmbeddingTestResult] = useState('');
  const [embeddingTesting, setEmbeddingTesting] = useState(false);

  // AI config - Chat
  const [chatProvider, setChatProvider] = useState('openai');
  const [chatModel, setChatModel] = useState('');
  const [chatApiKey, setChatApiKey] = useState('');
  const [chatBaseUrl, setChatBaseUrl] = useState('');
  const [chatFetchedModels, setChatFetchedModels] = useState<string[]>([]);
  const [chatFetchingModels, setChatFetchingModels] = useState(false);
  const [chatTestResult, setChatTestResult] = useState('');
  const [chatTesting, setChatTesting] = useState(false);
  const [aiSaving, setAiSaving] = useState(false);

  // Scheduled tasks
  const [scheduledTasks, setScheduledTasks] = useState<ScheduledTask[]>(DEFAULT_TASKS);

  // Data management
  const [dataStats, setDataStats] = useState<DataStats | null>(null);
  const [dataLoading, setDataLoading] = useState(false);
  const [clearingCache, setClearingCache] = useState(false);
  const [clearingEmbeddings, setClearingEmbeddings] = useState(false);
  const [clearingMemory, setClearingMemory] = useState(false);
  const [clearingSessions, setClearingSessions] = useState(false);
  const [clearingKnowledge, setClearingKnowledge] = useState(false);
  const [clearingAll, setClearingAll] = useState(false);
  const [clearingData, setClearingData] = useState(false);
  const [cleanupOld, setCleanupOld] = useState(false);

  // Folder browser
  const [folderPath, setFolderPath] = useState('');
  const [folderItems, setFolderItems] = useState<any[]>([]);
  const [folderLoading, setFolderLoading] = useState(false);

  useEffect(() => {
    loadKnowledgeConfigs();
    loadAIConfig();
    loadScheduleSettings();
    loadAppSettings();
  }, []);

  // Auto-load data stats when data tab is active
  useEffect(() => {
    if (activeTab === 'data') {
      loadDataStats();
    }
    if (activeTab === 'general') {
      loadBackupList();
    }
  }, [activeTab]);

  // Scheduled task executor — checks every 60s if any task should run
  useEffect(() => {
    const checkAndRunTasks = async () => {
      const now = new Date();
      const currentTime = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
      const today = now.toISOString().split('T')[0]; // YYYY-MM-DD

      for (const task of scheduledTasks) {
        if (!task.enabled) continue;
        if (task.time !== currentTime) continue;

        // Check if already ran today
        const lastRunKey = `task_last_run_${task.id}`;
        const lastRun = localStorage.getItem(lastRunKey);
        if (lastRun === today) continue;

        // Mark as ran today
        localStorage.setItem(lastRunKey, today);

        // Execute task
        try {
          let endpoint = '';
          switch (task.id) {
            case 'knowledge_index':
              endpoint = '/api/knowledge/reindex';
              break;
            case 'vectorize':
              endpoint = '/api/knowledge/light-embed';
              break;
            case 'memory_collect':
              endpoint = '/api/memory/pull';
              break;
            case 'session_pull':
              endpoint = '/api/session/pull';
              break;
            case 'memory_vectorize':
              endpoint = '/api/knowledge/light-embed?type=memory';
              break;
            case 'session_vectorize':
              endpoint = '/api/knowledge/light-embed?type=session';
              break;
            default:
              continue;
          }

          const res = await fetch(`${API_BASE}${endpoint}`, { method: 'POST' });
          if (res.ok) {
            console.log(`Scheduled task "${task.id}" executed successfully`);
          } else {
            console.error(`Scheduled task "${task.id}" failed: HTTP ${res.status}`);
          }
        } catch (e: any) {
          console.error(`Scheduled task "${task.id}" error:`, e);
        }
      }
    };

    // Check immediately on mount
    checkAndRunTasks();
    // Then check every 60 seconds
    const interval = setInterval(checkAndRunTasks, 60000);
    return () => clearInterval(interval);
  }, [scheduledTasks, API_BASE]);

  const loadKnowledgeConfigs = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/config`);
      if (res.ok) {
        const data = await res.json();
        setKnowledgeConfigs(data.configs || []);
      }
    } catch (e: any) {
      console.error('Failed to load knowledge configs:', e);
    }
  };

  const loadAIConfigFromLocal = () => {
    try {
      const local = localStorage.getItem("khub-ai-config");
      if (local) {
        const d = JSON.parse(local);
        if (d.embedding_provider) setEmbeddingProvider(d.embedding_provider);
        if (d.embedding_model) setEmbeddingModel(d.embedding_model);
        if (d.embedding_api_key) setEmbeddingApiKey(d.embedding_api_key);
        if (d.embedding_base_url) setEmbeddingBaseUrl(d.embedding_base_url);
        if (d.chat_provider) setChatProvider(d.chat_provider);
        if (d.chat_model) setChatModel(d.chat_model);
        if (d.chat_api_key) setChatApiKey(d.chat_api_key);
        if (d.chat_base_url) setChatBaseUrl(d.chat_base_url);
      }
    } catch {}
  };

  const loadAIConfig = async () => {
    try {
      const res = await fetch(`${API_BASE}/api/ai/config`);
      if (res.ok) {
        const data = await res.json();
        if (data.embedding_provider) setEmbeddingProvider(data.embedding_provider);
        if (data.embedding_model) setEmbeddingModel(data.embedding_model);
        if (data.embedding_api_key) setEmbeddingApiKey(data.embedding_api_key);
        if (data.embedding_base_url) setEmbeddingBaseUrl(data.embedding_base_url);
        if (data.chat_provider) setChatProvider(data.chat_provider);
        if (data.chat_model) setChatModel(data.chat_model);
        if (data.chat_api_key) setChatApiKey(data.chat_api_key);
        if (data.chat_base_url) setChatBaseUrl(data.chat_base_url);
      } else {
        loadAIConfigFromLocal();
      }
    } catch (e: any) {
      loadAIConfigFromLocal();
    }
  };

  const loadScheduleSettings = () => {
    try {
      const saved = localStorage.getItem('scheduled_tasks');
      if (saved) {
        const savedTasks: ScheduledTask[] = JSON.parse(saved);
        // Merge: always use default label/description (translation keys), keep saved enabled/time
        const merged = DEFAULT_TASKS.map(defaultTask => {
          const savedTask = savedTasks.find(s => s.id === defaultTask.id);
          if (savedTask) {
            return { ...defaultTask, enabled: savedTask.enabled, time: savedTask.time };
          }
          return defaultTask;
        });
        setScheduledTasks(merged);
      }
    } catch (e: any) {
      console.error('Failed to load schedule settings:', e);
    }
  };

  const loadAppSettings = async () => {
    try {
      // Check if running in Tauri
      if (window.__TAURI__) {
        const settings = await invoke('get_app_settings') as any;
        if (settings) {
          setMinimizeToTray(settings.minimize_to_tray);
          setAutostartEnabled(settings.autostart_enabled);
          setAutoBackup(settings.auto_backup);
          setBackupInterval(settings.backup_interval_hours);
          setBackupDir(settings.backup_dir);
          setAppVersion(settings.version);
          if (settings.data_dir) setDataDir(settings.data_dir);
        }
      }
    } catch (e: any) {
      console.error('Failed to load app settings:', e);
    }
  };

  const loadBackupList = async () => {
    try {
      if (window.__TAURI__) {
        const list = await invoke('get_backup_list') as string[];
        setBackupList(list || []);
      }
    } catch (e: any) {
      console.error('Failed to load backup list:', e);
    }
  };

  const toggleMinimizeToTray = async () => {
    const newValue = !minimizeToTray;
    setMinimizeToTray(newValue);
    try {
      if (window.__TAURI__) {
        await invoke('set_minimize_to_tray', { value: newValue });
      }
      if (window.__TAURI__) toast.success(t('settings.general.saved'));
    } catch (e: any) {
      toast.error(t('settings.general.saveFailed') + ': ' + (e?.message || e));
    }
  };

  const toggleAutostart = async () => {
    const newValue = !autostartEnabled;
    setAutostartEnabled(newValue);
    try {
      if (window.__TAURI__) {
        await invoke('set_autostart', { enabled: newValue });
      }
      if (window.__TAURI__) toast.success(t('settings.general.saved'));
    } catch (e: any) {
      toast.error(t('settings.general.saveFailed') + ': ' + (e?.message || e));
    }
  };

  const toggleAutoBackup = async () => {
    const newValue = !autoBackup;
    setAutoBackup(newValue);
    try {
      if (window.__TAURI__) {
        await invoke('set_auto_backup', { enabled: newValue });
      }
      if (window.__TAURI__) toast.success(t('settings.general.saved'));
    } catch (e: any) {
      toast.error(t('settings.general.saveFailed') + ': ' + (e?.message || e));
    }
  };

  const saveBackupInterval = async () => {
    try {
      if (window.__TAURI__) {
        await invoke('set_backup_interval', { hours: backupInterval });
      }
      if (window.__TAURI__) toast.success(t('settings.general.saved'));
    } catch (e: any) {
      toast.error(t('settings.general.saveFailed') + ': ' + (e?.message || e));
    }
  };

  const saveBackupDir = async () => {
    try {
      if (window.__TAURI__) {
        await invoke('set_backup_dir', { dir: backupDir });
      }
      if (window.__TAURI__) toast.success(t('settings.general.saved'));
    } catch (e: any) {
      toast.error(t('settings.general.saveFailed') + ': ' + (e?.message || e));
    }
  };

  const createBackup = async () => {
    setIsBackingUp(true);
    try {
      if (window.__TAURI__) {
        const path = await invoke('create_backup') as string;
        toast.success(t('settings.general.backupSuccess', { path: path as string }));
        loadBackupList();
      }
    } catch (e: any) {
      toast.error(t('settings.general.backupFailed', { error: e.message }));
    } finally {
      setIsBackingUp(false);
    }
  };

  const [restorePath, setRestorePath] = useState('');
  const [restoring, setRestoring] = useState(false);

  const restoreBackup = async () => {
    if (!restorePath.trim()) {
      toast.info('请输入备份文件路径');
      return;
    }
    if (!await toast.confirm('恢复将合并备份数据到当前数据目录。JSON配置会合并新字段，数据库和向量数据会直接覆盖。确定继续？', '恢复备份', { danger: true })) return;
    setRestoring(true);
    try {
      if (window.__TAURI__) {
        const result = await invoke('restore_backup', { path: restorePath }) as any;
        toast.success(`恢复完成！恢复了 ${result?.restored?.length || 0} 个文件，合并了 ${result?.merged?.length || 0} 个配置`);
        setRestorePath('');
      }
    } catch (e: any) {
      toast.error('恢复失败: ' + (e.message || e));
    } finally {
      setRestoring(false);
    }
  };

  const openLogsFolder = async () => {
    try {
      if (window.__TAURI__) {
        await invoke('open_logs_folder');
      }
    } catch (e: any) {
      console.error('Failed to open logs folder:', e);
    }
  };

  const addKnowledgeConfig = async () => {
    if (!newPath.trim()) return;
    setAddingPath(true);
    setKbError('');
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newName.trim() || t('settings.kb.newKbDefault'),
          paths: [newPath.trim()],
          file_patterns: ['*'],
          auto_index: true,
          auto_embed: false,
        }),
      });
      if (res.ok) {
        setNewPath('');
        setNewName('');
        loadKnowledgeConfigs();
      } else {
        const data = await res.json().catch(() => ({}));
        setKbError(t('settings.kb.addFailed', { error: data.error || `HTTP ${res.status}` }));
      }
    } catch (e: any) {
      setKbError(t('settings.kb.addFailed', { error: e.message }));
    } finally {
      setAddingPath(false);
    }
  };

  const deleteKnowledgeConfig = async (id: string) => {
    if (!await toast.confirm(t('settings.kb.deleteConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/config/${id}`, { method: 'DELETE' });
      if (res.ok) {
        loadKnowledgeConfigs();
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(t('settings.kb.deleteFailed', { error: data.error || `HTTP ${res.status}` }));
      }
    } catch (e: any) {
      toast.error(t('settings.kb.deleteFailed', { error: e.message }));
    }
  };

  const toggleAutoIndex = async (config: KnowledgeBaseConfig) => {
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/config/${config.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ auto_index: !config.auto_index }),
      });
      if (res.ok) loadKnowledgeConfigs();
    } catch (e: any) {
      console.error('Failed to toggle auto index:', e);
    }
  };

  const toggleAutoEmbed = async (config: KnowledgeBaseConfig) => {
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/config/${config.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ auto_embed: !config.auto_embed }),
      });
      if (res.ok) loadKnowledgeConfigs();
    } catch (e: any) {
      console.error('Failed to toggle auto embed:', e);
    }
  };

  const reindexAll = async () => {
    setKbLoading(true);
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/reindex`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        toast.success(t('settings.kb.reindexComplete', { count: data.updated || 0 }));
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(t('settings.kb.reindexFailed', { error: data.error || `HTTP ${res.status}` }));
      }
    } catch (e: any) {
      toast.error(t('settings.kb.reindexFailed', { error: e.message }));
    } finally {
      setKbLoading(false);
    }
  };

  const saveEmbeddingConfig = async () => {
    setAiSaving(true);
    try {
      const provider = AI_PROVIDERS.find(p => p.id === embeddingProvider);
      const embBaseUrl = embeddingBaseUrl || provider?.base_url || '';
      const chatProviderObj = AI_PROVIDERS.find(p => p.id === chatProvider);
      const chatBaseUrlVal = chatBaseUrl || chatProviderObj?.base_url || '';

      const res = await fetch(`${API_BASE}/api/ai/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          embedding_provider: embeddingProvider,
          embedding_model: embeddingModel,
          embedding_api_key: embeddingApiKey,
          embedding_base_url: embeddingBaseUrl,
          chat_provider: chatProvider,
          chat_model: chatModel,
          chat_api_key: chatApiKey,
          chat_base_url: chatBaseUrl,
        }),
      });
      if (res.ok) {
        toast.success(t('settings.ai.embeddingSaved', { default: '✅ Embedding 配置已保存' }));
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(t('settings.ai.saveFailed', { error: data.error || `HTTP ${res.status}` }));
      }
    } catch (e: any) {
      // Fallback: save to localStorage
      const cfg = {
        embedding_provider: embeddingProvider,
        embedding_model: embeddingModel,
        embedding_api_key: embeddingApiKey,
        embedding_base_url: embeddingBaseUrl,
        chat_provider: chatProvider,
        chat_model: chatModel,
        chat_api_key: chatApiKey,
        chat_base_url: chatBaseUrl,
      };
      localStorage.setItem("khub-ai-config", JSON.stringify(cfg));
      toast.warning(t('settings.ai.saveFailed', { error: e.message || 'Backend unavailable' }) + ' — saved locally');
    } finally {
      setAiSaving(false);
    }
  };

  const saveChatConfig = async () => {
    setAiSaving(true);
    try {
      const provider = AI_PROVIDERS.find(p => p.id === embeddingProvider);
      const embBaseUrl = embeddingBaseUrl || provider?.base_url || '';
      const chatProviderObj = AI_PROVIDERS.find(p => p.id === chatProvider);
      const chatBaseUrlVal = chatBaseUrl || chatProviderObj?.base_url || '';

      const res = await fetch(`${API_BASE}/api/ai/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          embedding_provider: embeddingProvider,
          embedding_model: embeddingModel,
          embedding_api_key: embeddingApiKey,
          embedding_base_url: embeddingBaseUrl,
          chat_provider: chatProvider,
          chat_model: chatModel,
          chat_api_key: chatApiKey,
          chat_base_url: chatBaseUrl,
        }),
      });
      if (res.ok) {
        toast.success(t('settings.ai.chatSaved', { default: '✅ Chat 配置已保存' }));
      } else {
        const data = await res.json().catch(() => ({}));
        toast.error(t('settings.ai.saveFailed', { error: data.error || `HTTP ${res.status}` }));
      }
    } catch (e: any) {
      // Fallback: save to localStorage
      const cfg = {
        embedding_provider: embeddingProvider,
        embedding_model: embeddingModel,
        embedding_api_key: embeddingApiKey,
        embedding_base_url: embeddingBaseUrl,
        chat_provider: chatProvider,
        chat_model: chatModel,
        chat_api_key: chatApiKey,
        chat_base_url: chatBaseUrl,
      };
      localStorage.setItem("khub-ai-config", JSON.stringify(cfg));
      toast.warning(t('settings.ai.saveFailed', { error: e.message || 'Backend unavailable' }) + ' — saved locally');
    } finally {
      setAiSaving(false);
    }
  };

  const testAIConnection = async (section: 'embedding' | 'chat') => {
    const provider = section === 'embedding' ? embeddingProvider : chatProvider;
    const model = section === 'embedding' ? embeddingModel : chatModel;
    const apiKey = section === 'embedding' ? embeddingApiKey : chatApiKey;
    const setTestResult = section === 'embedding' ? setEmbeddingTestResult : setChatTestResult;
    const setTesting = section === 'embedding' ? setEmbeddingTesting : setChatTesting;
    const baseUrl = section === 'embedding'
      ? (embeddingBaseUrl || AI_PROVIDERS.find(p => p.id === embeddingProvider)?.base_url || '')
      : (chatBaseUrl || AI_PROVIDERS.find(p => p.id === chatProvider)?.base_url || '');

    setTesting(true);
    setTestResult(t('settings.ai.testing'));
    try {
      const res = await fetch(`${API_BASE}/api/ai/test`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider,
          model: model || undefined,
          api_key: apiKey,
          base_url: baseUrl || undefined,
          mode: section,
        }),
      });
      const data = await res.json();
      if (data.success) {
        const msg = data.message || (section === 'embedding' ? t('settings.ai.embeddingConnectionSuccess') : t('settings.ai.chatConnectionSuccess'));
        setTestResult(`✅ ${msg}`);
      } else {
        setTestResult(`❌ ${data.error || t('common.error')}`);
      }
    } catch (error: any) {
      setTestResult(`❌ ${t('common.error')}: ${error.message}`);
    } finally {
      setTesting(false);
    }
  };

  const fetchModelsFromAPI = async (section: 'embedding' | 'chat') => {
    const provider = section === 'embedding' ? embeddingProvider : chatProvider;
    const apiKey = section === 'embedding' ? embeddingApiKey : chatApiKey;
    const setModels = section === 'embedding' ? setEmbeddingFetchedModels : setChatFetchedModels;
    const setFetching = section === 'embedding' ? setEmbeddingFetchingModels : setChatFetchingModels;
    const customBaseUrl = section === 'embedding' ? embeddingBaseUrl : chatBaseUrl;
    const providerObj = AI_PROVIDERS.find(p => p.id === provider);
    const baseUrl = customBaseUrl || providerObj?.base_url || '';

    if (!baseUrl) {
      toast.info(t('settings.ai.fillBaseUrl'));
      return;
    }
    if (!apiKey && provider !== 'ollama') {
      toast.info(t('settings.ai.fillApiKey'));
      return;
    }

    setFetching(true);
    try {
      const res = await fetch(`${API_BASE}/api/ai/fetch-models`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ provider, api_key: apiKey, base_url: baseUrl }),
      });
      const data = await res.json();
      if (data.success && Array.isArray(data.models) && data.models.length > 0) {
        // Backend returns model objects {id, name, created, owned_by}
        // but state expects string[]. Extract the id/name as strings.
        const modelNames = data.models
          .map((m: any) => {
            if (!m) return '';
            if (typeof m === 'string') return m;
            return m.id || m.name || String(m);
          })
          .filter((s: string) => s.length > 0);
        if (modelNames.length > 0) {
          setModels(modelNames);
        } else {
          toast.info(t('settings.ai.emptyModelList'));
        }
      } else {
        toast.error(data.error || t('settings.ai.emptyModelList'));
      }
    } catch (e: any) {
      toast.error(t('settings.ai.fetchFailed', { error: e.message }));
    } finally {
      setFetching(false);
    }
  };

  const saveScheduleSettings = () => {
    const tasksWithNextRun = scheduledTasks.map(task => ({
      ...task,
      nextRun: task.enabled ? computeNextRun(task.time) : undefined,
    }));
    setScheduledTasks(tasksWithNextRun);
    localStorage.setItem('scheduled_tasks', JSON.stringify(tasksWithNextRun));
    toast.info(t('settings.schedule.saved'));
  };

  const loadDataStats = async () => {
    setDataLoading(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/stats`);
      if (res.ok) {
        const data = await res.json();
        // Map nested backend response to flat frontend format
        setDataStats({
          memory_total: data.doc?.total || 0,
          memory_active: data.memory?.active || 0,
          memory_deleted: data.memory?.deleted || 0,
          memory_archived: data.memory?.archived || 0,
          memory_candidate: data.memory?.candidate || 0,
          memory_count: data.memory?.count || data.memory?.active || 0,
          session_count: data.sessions?.count || data.sessions?.total || 0,
          knowledge_total: data.doc?.knowledge || data.memory?.knowledge || 0,
          artifacts: data.artifact_count || 0,
          kb_configs: data.config_count || 0,
          db_size: `${data.db_size_mb || 0} MB`,
          cache_size: `${data.cache_size_mb || 0} MB`,
          embedding_size: `${data.embedding_size_mb || 0} MB`,
          total_vectorized: data.doc?.total_vectorized || 0,
          memory_vectorized: data.doc?.memory_vectorized || 0,
          session_vectorized: data.doc?.session_vectorized || 0,
          knowledge_vectorized: data.doc?.knowledge_vectorized || 0,
          all_deleted: data.doc?.all_deleted || 0,
          all_archived: data.doc?.all_archived || 0,
        });
      }
    } catch (e: any) {
      toast.error(t('settings.data.loadFailed', { error: e.message }));
    } finally {
      setDataLoading(false);
    }
  };

  const clearCache = async () => {
    if (!await toast.confirm(t('settings.data.clearCacheConfirm'))) return;
    setClearingCache(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_cache: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.cacheCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear cache:', e);
    } finally {
      setClearingCache(false);
    }
  };

  const clearEmbeddings = async () => {
    if (!await toast.confirm(t('settings.data.clearEmbeddingsConfirm'))) return;
    setClearingEmbeddings(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_embeddings: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.embeddingsCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear embeddings:', e);
    } finally {
      setClearingEmbeddings(false);
    }
  };

  const clearMemory = async () => {
    if (!await toast.confirm(t('settings.data.clearMemoryConfirm'))) return;
    setClearingMemory(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_memory: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.memoryCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear memory:', e);
    } finally {
      setClearingMemory(false);
    }
  };

  const clearSessions = async () => {
    if (!await toast.confirm(t('settings.data.clearSessionsConfirm'))) return;
    setClearingSessions(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_sessions: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.sessionsCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear sessions:', e);
    } finally {
      setClearingSessions(false);
    }
  };

  const clearKnowledge = async () => {
    if (!await toast.confirm(t('settings.data.clearKnowledgeConfirm'))) return;
    setClearingKnowledge(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_knowledge: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.knowledgeCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear knowledge:', e);
    } finally {
      setClearingKnowledge(false);
    }
  };

  const clearMemoryMetadata = async () => {
    if (!await toast.confirm(t('settings.data.clearMemoryMetadataConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_memory_metadata: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) { toast.success(t('settings.data.memoryMetadataCleared')); loadDataStats(); }
    } catch (e: any) {
      console.error(e);
    }
  };

  const clearSessionMetadata = async () => {
    if (!await toast.confirm(t('settings.data.clearSessionMetadataConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_session_metadata: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) { toast.success(t('settings.data.sessionMetadataCleared')); loadDataStats(); }
    } catch (e: any) {
      console.error(e);
    }
  };

  const clearKnowledgeMetadata = async () => {
    if (!await toast.confirm(t('settings.data.clearKnowledgeMetadataConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_knowledge_metadata: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) { toast.success(t('settings.data.knowledgeMetadataCleared')); loadDataStats(); }
    } catch (e: any) {
      console.error(e);
    }
  };

  const clearData = async (type: string) => {
    const confirmKey = `settings.data.clear${type.charAt(0).toUpperCase() + type.slice(1)}Confirm` as any;
    if (!await toast.confirm(t(confirmKey))) return;
    
    const stateKey = `clearing${type.charAt(0).toUpperCase() + type.slice(1)}` as any;
    const setter = eval(`set${stateKey.charAt(0).toUpperCase() + stateKey.slice(1)}`);
    setter(true);
    
    try {
      const body: any = { confirmation: 'CONFIRM_CLEAR' };
      body[`clear_${type}`] = true;
      
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      if (res.ok) {
        const resultKey = `settings.data.${type}Cleared` as any;
        toast.success(t(resultKey));
        loadDataStats();
      }
    } catch (e: any) {
      console.error(`Failed to clear ${type}:`, e);
    } finally {
      setter(false);
    }
  };

  const clearAll = async () => {
    if (!await toast.confirm(t('settings.data.clearAllConfirm'))) return;
    if (!await toast.confirm(t('settings.data.clearAllConfirmed'))) return;
    setClearingAll(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_all: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.allCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear all:', e);
    } finally {
      setClearingAll(false);
    }
  };

  const clearAllData = async () => {
    if (!await toast.confirm(t('settings.data.clearAllDataConfirm'))) return;
    setClearingData(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_memory: true, clear_sessions: true, clear_knowledge: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.allDataCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear all data:', e);
    } finally {
      setClearingData(false);
    }
  };

  const clearDeleted = async () => {
    if (!await toast.confirm(t('settings.data.clearDeletedConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/data/clear`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ clear_deleted: true, confirmation: 'CONFIRM_CLEAR' }),
      });
      if (res.ok) {
        toast.success(t('settings.data.deletedCleared'));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to clear deleted:', e);
    }
  };

  const restoreDeleted = async () => {
    if (!await toast.confirm(t('settings.data.restoreDeletedConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/knowledge/restore-deleted`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        toast.success(t('settings.data.deletedRestored', { count: data.restored || 0 }));
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to restore deleted:', e);
    }
  };

  const cleanupOldVersions = async () => {
    if (!await toast.confirm(t('settings.data.cleanupConfirm'))) return;
    setCleanupOld(true);
    try {
      const res = await fetch(`${API_BASE}/api/data/cleanup-old-versions`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        toast.success(data.message || 'Done');
        loadDataStats();
      }
    } catch (e: any) {
      console.error('Failed to cleanup old versions:', e);
    } finally {
      setCleanupOld(false);
    }
  };

  const browseFolder = async (path?: string) => {
    setFolderLoading(true);
    try {
      const url = path ? `${API_BASE}/api/filesystem/list?path=${encodeURIComponent(path)}` : `${API_BASE}/api/filesystem/roots`;
      const res = await fetch(url);
      if (res.ok) {
        const data = await res.json();
        if (path) {
          setFolderPath(path);
          setFolderItems(data.items || []);
        } else {
          setFolderItems(data.roots || []);
        }
      }
    } catch (e: any) {
      console.error('Failed to browse folder:', e);
    } finally {
      setFolderLoading(false);
    }
  };

  const selectFolder = () => {
    if (folderPath) {
      setNewPath(folderPath);
      setShowFolderBrowser(false);
    }
  };

  return (
    <div className="py-6">
      <h1 className="text-2xl font-bold mb-6">⚙️ {t('settings.title')}</h1>

      {/* Tabs */}
      <div className="flex border-b border-gray-200 mb-6">
        {[
          { key: 'general' as const, label: t('settings.tabGeneral') },
          { key: 'knowledge' as const, label: t('settings.tabKnowledge') },
          { key: 'ai' as const, label: t('settings.tabAI') },
          { key: 'schedule' as const, label: t('settings.tabSchedule') },
          { key: 'data' as const, label: t('settings.tabData') },
          { key: 'about' as const, label: t('settings.tabAbout') },
        ].map(tab => (
          <button
            key={tab.key}
            onClick={() => handleTabChange(tab.key)}
            className={`px-4 py-2 text-sm font-medium transition-colors ${
              activeTab === tab.key
                ? 'text-blue-600 border-b-2 border-blue-600'
                : 'text-gray-500 hover:text-gray-700'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* General Settings */}
      {activeTab === 'general' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.general.title')}</h2>
          <p className="text-sm text-gray-500 mb-6">{t('settings.general.description')}</p>

          <div className="space-y-6">
            {/* Window Behavior */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.general.windowTitle')}</h3>
              
              <div className="space-y-4">
                <label className="flex items-center justify-between">
                  <div>
                    <span className="font-medium">{t('settings.general.autostart')}</span>
                    <p className="text-xs text-gray-500 mt-1">{t('settings.general.autostartDesc')}</p>
                  </div>
                  <button
                    onClick={toggleAutostart}
                    className={`relative w-12 h-6 rounded-full transition-colors ${
                      autostartEnabled ? 'bg-blue-600' : 'bg-gray-300'
                    }`}
                  >
                    <div className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${
                      autostartEnabled ? 'translate-x-6' : ''
                    }`} />
                  </button>
                </label>

                <label className="flex items-center justify-between">
                  <div>
                    <span className="font-medium">{t('settings.general.minimizeToTray')}</span>
                    <p className="text-xs text-gray-500 mt-1">{t('settings.general.minimizeToTrayDesc')}</p>
                  </div>
                  <button
                    onClick={toggleMinimizeToTray}
                    className={`relative w-12 h-6 rounded-full transition-colors ${
                      minimizeToTray ? 'bg-blue-600' : 'bg-gray-300'
                    }`}
                  >
                    <div className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${
                      minimizeToTray ? 'translate-x-6' : ''
                    }`} />
                  </button>
                </label>
              </div>
            </div>

            {/* Backup Settings */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.general.backupTitle')}</h3>
              
              <div className="space-y-4">
                <label className="flex items-center justify-between">
                  <div>
                    <span className="font-medium">{t('settings.general.autoBackup')}</span>
                    <p className="text-xs text-gray-500 mt-1">{t('settings.general.autoBackupDesc')}</p>
                  </div>
                  <button
                    onClick={toggleAutoBackup}
                    className={`relative w-12 h-6 rounded-full transition-colors ${
                      autoBackup ? 'bg-blue-600' : 'bg-gray-300'
                    }`}
                  >
                    <div className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full transition-transform ${
                      autoBackup ? 'translate-x-6' : ''
                    }`} />
                  </button>
                </label>

                <div className="flex items-center gap-4">
                  <label className="text-sm font-medium whitespace-nowrap">
                    {t('settings.general.backupInterval')}
                  </label>
                  <input
                    type="number"
                    value={backupInterval}
                    onChange={e => setBackupInterval(Number(e.target.value))}
                    min="1"
                    max="720"
                    className="w-20 px-3 py-2 border rounded-lg text-sm"
                  />
                  <span className="text-sm text-gray-500">{t('settings.general.hours')}</span>
                  <button
                    onClick={saveBackupInterval}
                    className="px-3 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700"
                  >
                    {t('common.save')}
                  </button>
                </div>

                <div className="flex items-center gap-4">
                  <label className="text-sm font-medium whitespace-nowrap">
                    {t('settings.general.backupDir')}
                  </label>
                  <input
                    type="text"
                    value={backupDir}
                    onChange={e => setBackupDir(e.target.value)}
                    placeholder={t('settings.general.backupDirPlaceholder')}
                    className="flex-1 px-3 py-2 border rounded-lg text-sm"
                  />
                  <button
                    onClick={saveBackupDir}
                    className="px-3 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700"
                  >
                    {t('common.save')}
                  </button>
                </div>

                <div className="flex gap-2">
                  <button
                    onClick={createBackup}
                    disabled={isBackingUp}
                    className="px-4 py-2 bg-green-600 text-white rounded-lg text-sm hover:bg-green-700 disabled:opacity-50"
                  >
                    {isBackingUp ? t('settings.general.backingUp') : t('settings.general.createBackup')}
                  </button>
                </div>

                <div className="flex gap-2 mt-3">
                  <input
                    type="text"
                    value={restorePath}
                    onChange={e => setRestorePath(e.target.value)}
                    placeholder="备份文件路径（如 C:\backups\khub_backup_xxx.zip）"
                    className="flex-1 px-3 py-2 border rounded-lg text-sm"
                  />
                  <button
                    onClick={restoreBackup}
                    disabled={restoring || !restorePath.trim()}
                    className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50"
                  >
                    {restoring ? '恢复中...' : '恢复备份'}
                  </button>
                </div>

                {backupList.length > 0 && (
                  <div className="mt-4">
                    <h4 className="text-sm font-medium mb-2">{t('settings.general.backupHistory')}</h4>
                    <div className="max-h-40 overflow-y-auto border rounded-lg">
                      {backupList.slice(0, 10).map((backup, i) => (
                        <div key={i} className="px-3 py-2 text-sm border-b last:border-b-0">
                          {backup.split('/').pop() || backup.split('\\').pop()}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Data Directory */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.general.dataDirTitle')}</h3>
              <p className="text-sm text-gray-500 mb-2">{t('settings.general.dataDirDesc')}</p>
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={dataDir || (appVersion ? `C:\\Users\\...\\AppData\\Roaming\\KnowledgeHub` : '')}
                  disabled
                  className="flex-1 px-3 py-2 bg-gray-50 border rounded-lg text-sm text-gray-600"
                />
                <button
                  onClick={openLogsFolder}
                  className="px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm"
                >
                  {t('settings.general.openLogs')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Knowledge Base Config */}
      {activeTab === 'knowledge' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.kb.title')}</h2>
          <p className="text-sm text-gray-500 mb-4">{t('settings.kb.description')}</p>

          {/* Add new config */}
          <div className="flex gap-2 mb-4">
            <input
              type="text"
              value={newName}
              onChange={e => setNewName(e.target.value)}
              placeholder={t('settings.kb.namePlaceholder')}
              className="px-3 py-2 border border-gray-300 rounded-lg text-sm w-48"
            />
            <input
              type="text"
              value={newPath}
              onChange={e => setNewPath(e.target.value)}
              placeholder={t('settings.kb.pathPlaceholder')}
              className="flex-1 px-3 py-2 border border-gray-300 rounded-lg text-sm"
            />
            <button
              onClick={() => { setShowFolderBrowser(true); browseFolder(); }}
              className="px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm"
            >
              {t('settings.kb.browse')}
            </button>
            <button
              onClick={addKnowledgeConfig}
              disabled={addingPath || !newPath.trim()}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50"
            >
              {addingPath ? t('settings.kb.adding') : t('settings.kb.add')}
            </button>
          </div>
          {kbError && <p className="text-sm text-red-600 mb-4">{kbError}</p>}

          {/* Config list */}
          {knowledgeConfigs.length === 0 ? (
            <p className="text-gray-400">{t('settings.kb.noConfigs')}</p>
          ) : (
            <div className="space-y-3">
              {knowledgeConfigs.map(config => (
                <div key={config.id} className="bg-white border border-gray-200 rounded-lg p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="font-medium">{config.name}</h3>
                      <p className="text-xs text-gray-400 font-mono">{config.paths.join(', ')}</p>
                    </div>
                    <div className="flex items-center gap-3">
                      <label className="flex items-center gap-1 text-sm">
                        <input type="checkbox" checked={config.auto_index} onChange={() => toggleAutoIndex(config)} className="rounded" />
                        {t('settings.kb.autoIndex')}
                      </label>
                      <label className="flex items-center gap-1 text-sm">
                        <input type="checkbox" checked={config.auto_embed} onChange={() => toggleAutoEmbed(config)} className="rounded" />
                        {t('settings.kb.autoEmbed')}
                      </label>
                      <button onClick={() => deleteKnowledgeConfig(config.id)} className="text-red-500 hover:text-red-700 text-sm">
                        {t('common.delete')}
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          <button
            onClick={reindexAll}
            disabled={kbLoading}
            className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50"
          >
            {kbLoading ? t('common.loading') : t('settings.kb.reindexAll')}
          </button>
        </div>
      )}

      {/* AI Config */}
      {activeTab === 'ai' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.ai.title')}</h2>
          <p className="text-sm text-gray-500 mb-6">{t('settings.ai.description')}</p>

          {/* Embedding Config */}
          <div className="bg-white border border-gray-200 rounded-lg p-5 mb-4">
            <h3 className="font-semibold mb-3">{t('settings.ai.embeddingTitle')}</h3>
            <p className="text-xs text-gray-500 mb-4">{t('settings.ai.embeddingDesc')}</p>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.provider')}</label>
                <select value={embeddingProvider} onChange={e => setEmbeddingProvider(e.target.value)} className="w-full px-3 py-2 border rounded-lg text-sm">
                  {AI_PROVIDERS.map(p => <option key={p.id} value={p.id}>{p.nameKey ? t(p.nameKey) : p.name}</option>)}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.apiKey')}</label>
                <input type="password" value={embeddingApiKey} onChange={e => setEmbeddingApiKey(e.target.value)} className="w-full px-3 py-2 border rounded-lg text-sm" />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.baseUrl')}</label>
                <input type="text" value={embeddingBaseUrl} onChange={e => setEmbeddingBaseUrl(e.target.value)} placeholder={AI_PROVIDERS.find(p => p.id === embeddingProvider)?.base_url || ''} className="w-full px-3 py-2 border rounded-lg text-sm" />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.embeddingModel')}</label>
                <div className="flex gap-2">
                  <input type="text" value={embeddingModel} onChange={e => setEmbeddingModel(e.target.value)} placeholder={t('settings.ai.embeddingModelPlaceholder')} className="flex-1 px-3 py-2 border rounded-lg text-sm" />
                  <button onClick={() => fetchModelsFromAPI('embedding')} disabled={embeddingFetchingModels} className="px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm">
                    {embeddingFetchingModels ? t('settings.ai.fetchingModels') : t('settings.ai.fetchModels')}
                  </button>
                </div>
                {embeddingFetchedModels.length > 0 && (
                  <select onChange={e => setEmbeddingModel(e.target.value)} className="w-full mt-1 px-3 py-2 border rounded-lg text-sm">
                    <option value="">{t('settings.ai.selectModel')}</option>
                    {embeddingFetchedModels.map(m => <option key={m} value={m}>{m}</option>)}
                  </select>
                )}
              </div>
            </div>
            <div className="flex gap-2 mt-3">
              <button onClick={() => testAIConnection('embedding')} disabled={embeddingTesting} className="px-4 py-2 bg-green-600 text-white rounded-lg text-sm hover:bg-green-700 disabled:opacity-50">
                {embeddingTesting ? t('settings.ai.testing') : t('settings.ai.test')}
              </button>
              <button onClick={saveEmbeddingConfig} disabled={aiSaving} className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50">
                {aiSaving ? t('settings.ai.saving') : t('common.save')}
              </button>
              {embeddingTestResult && <span className="text-sm self-center">{embeddingTestResult}</span>}
            </div>
          </div>

          {/* Chat Config */}
          <div className="bg-white border border-gray-200 rounded-lg p-5 mb-4">
            <h3 className="font-semibold mb-3">{t('settings.ai.chatTitle')}</h3>
            <p className="text-xs text-gray-500 mb-4">{t('settings.ai.chatDesc')}</p>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.provider')}</label>
                <select value={chatProvider} onChange={e => setChatProvider(e.target.value)} className="w-full px-3 py-2 border rounded-lg text-sm">
                  {AI_PROVIDERS.map(p => <option key={p.id} value={p.id}>{p.nameKey ? t(p.nameKey) : p.name}</option>)}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.apiKey')}</label>
                <input type="password" value={chatApiKey} onChange={e => setChatApiKey(e.target.value)} className="w-full px-3 py-2 border rounded-lg text-sm" />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.baseUrl')}</label>
                <input type="text" value={chatBaseUrl} onChange={e => setChatBaseUrl(e.target.value)} placeholder={AI_PROVIDERS.find(p => p.id === chatProvider)?.base_url || ''} className="w-full px-3 py-2 border rounded-lg text-sm" />
              </div>
              <div>
                <label className="block text-sm font-medium mb-1">{t('settings.ai.chatModel')}</label>
                <div className="flex gap-2">
                  <input type="text" value={chatModel} onChange={e => setChatModel(e.target.value)} placeholder={t('settings.ai.chatModelPlaceholder')} className="flex-1 px-3 py-2 border rounded-lg text-sm" />
                  <button onClick={() => fetchModelsFromAPI('chat')} disabled={chatFetchingModels} className="px-3 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm">
                    {chatFetchingModels ? t('settings.ai.fetchingModels') : t('settings.ai.fetchModels')}
                  </button>
                </div>
                {chatFetchedModels.length > 0 && (
                  <select onChange={e => setChatModel(e.target.value)} className="w-full mt-1 px-3 py-2 border rounded-lg text-sm">
                    <option value="">{t('settings.ai.selectModel')}</option>
                    {chatFetchedModels.map(m => <option key={m} value={m}>{m}</option>)}
                  </select>
                )}
              </div>
            </div>
            <div className="flex gap-2 mt-3">
              <button onClick={() => testAIConnection('chat')} disabled={chatTesting} className="px-4 py-2 bg-green-600 text-white rounded-lg text-sm hover:bg-green-700 disabled:opacity-50">
                {chatTesting ? t('settings.ai.testing') : t('settings.ai.test')}
              </button>
              <button onClick={saveChatConfig} disabled={aiSaving} className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700 disabled:opacity-50">
                {aiSaving ? t('settings.ai.saving') : t('common.save')}
              </button>
              {chatTestResult && <span className="text-sm self-center">{chatTestResult}</span>}
            </div>
          </div>
        </div>
      )}

      {/* Scheduled Tasks */}
      {activeTab === 'schedule' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.schedule.title')}</h2>
          <p className="text-sm text-gray-500 mb-4">{t('settings.schedule.description')}</p>

          <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
            {scheduledTasks.map((task, index) => (
              <div key={task.id} className="bg-white border border-gray-200 rounded-lg p-5 flex flex-col min-h-[180px]">
                <div className="flex items-start gap-3 mb-3">
                  <input
                    type="checkbox"
                    checked={task.enabled}
                    onChange={() => {
                      const newTasks = [...scheduledTasks];
                      newTasks[index] = { ...newTasks[index], enabled: !newTasks[index].enabled };
                      setScheduledTasks(newTasks);
                    }}
                    className="rounded mt-1"
                  />
                  <div className="min-w-0">
                    <h3 className="font-semibold text-base">{t(task.label)}</h3>
                    <p className="text-sm text-gray-500 mt-1.5 leading-relaxed">{t(task.description)}</p>
                  </div>
                </div>
                <div className="mt-auto flex items-center gap-2 pt-3 border-t border-gray-100">
                  <label className="text-xs text-gray-400 whitespace-nowrap">{t('settings.schedule.execTime')}</label>
                  <input
                    type="time"
                    value={task.time}
                    onChange={e => {
                      const newTasks = [...scheduledTasks];
                      newTasks[index] = { ...newTasks[index], time: e.target.value };
                      setScheduledTasks(newTasks);
                    }}
                    className="flex-1 px-2 py-1.5 border rounded text-sm min-w-0"
                  />
                </div>
              </div>
            ))}
          </div>

          <button onClick={saveScheduleSettings} className="mt-4 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700">
            {t('settings.schedule.saveSchedule')}
          </button>
        </div>
      )}

      {/* Data Management */}
      {activeTab === 'data' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.data.title')}</h2>
          <p className="text-sm text-gray-500 mb-4">{t('settings.data.description')}</p>

          <button onClick={loadDataStats} disabled={dataLoading} className="px-4 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm mb-4">
            {dataLoading ? t('settings.data.loading') : t('settings.data.refreshStats')}
          </button>

          {/* Row 1: Overview */}
          <div className="grid grid-cols-4 gap-4 mb-4">
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.memoryTotal')}</p>
                <p className="text-xl font-bold">{dataStats?.memory_total ?? '--'}</p>
              </div>
              <button onClick={clearAllData} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.memoryCount')}</p>
                <p className="text-xl font-bold">{dataStats?.memory_count ?? '--'}</p>
              </div>
              <button onClick={clearMemory} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.sessionCount')}</p>
                <p className="text-xl font-bold">{dataStats?.session_count ?? '--'}</p>
              </div>
              <button onClick={clearSessions} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.knowledgeRecords')}</p>
                <p className="text-xl font-bold">{dataStats?.knowledge_total ?? '--'}</p>
              </div>
              <button onClick={clearKnowledge} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
          </div>

          {/* Row 2: Vectorization */}
          <div className="grid grid-cols-4 gap-4 mb-4">
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.totalVectorized')}</p>
                <p className="text-xl font-bold">{dataStats?.total_vectorized ?? '--'}</p>
              </div>
              <button onClick={clearEmbeddings} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.memoryVectorized')}</p>
                <p className="text-xl font-bold">{dataStats?.memory_vectorized ?? '--'}</p>
              </div>
              <button onClick={clearMemoryMetadata} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.sessionVectorized')}</p>
                <p className="text-xl font-bold">{dataStats?.session_vectorized ?? '--'}</p>
              </div>
              <button onClick={clearSessionMetadata} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.knowledgeVectorized')}</p>
                <p className="text-xl font-bold">{dataStats?.knowledge_vectorized ?? '--'}</p>
              </div>
              <button onClick={clearKnowledgeMetadata} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clear', { default: '清除' })}</button>
            </div>
          </div>

          {/* Row 3: Storage & Actions */}
          <div className="grid grid-cols-4 gap-4 mb-6">
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.deleted')}</p>
                <p className="text-xl font-bold">{dataStats?.all_deleted ?? '--'}</p>
                <p className="text-xs text-gray-400 mt-1">{t('settings.data.deletedDesc', { default: '通过 API 删除的记忆、会话和知识库记录会进入已删除状态，可恢复或永久清除' })}</p>
              </div>
              <div className="flex gap-1 mt-2">
                <button onClick={restoreDeleted} className="px-2 py-1 bg-blue-100 text-blue-700 rounded text-xs hover:bg-blue-200">{t('settings.data.restore')}</button>
                <button onClick={clearDeleted} className="px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200">{t('settings.data.clear')}</button>
              </div>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.oldVersions')}</p>
                <p className="text-xl font-bold">{dataStats?.all_archived ?? '--'}</p>
              </div>
              <button onClick={cleanupOldVersions} disabled={cleanupOld} className="mt-2 px-2 py-1 bg-yellow-100 text-yellow-800 rounded text-xs hover:bg-yellow-200 disabled:opacity-50 self-start">
                {cleanupOld ? t('settings.data.clearing') : t('settings.data.cleanupOld')}
              </button>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.dbSize')}</p>
                <p className="text-xl font-bold">{dataStats?.db_size ?? '--'}</p>
              </div>
            </div>
            <div className="bg-white border rounded-lg p-4 flex flex-col justify-between">
              <div>
                <p className="text-xs text-gray-500">{t('settings.data.cacheSize')}</p>
                <p className="text-xl font-bold">{dataStats?.cache_size ?? '--'}</p>
              </div>
              <button onClick={clearCache} className="mt-2 px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200 self-start">{t('settings.data.clearCache')}</button>
            </div>
          </div>

          {/* Danger Zone */}
          <div className="bg-red-50 border border-red-200 rounded-lg p-5">
            <h3 className="font-semibold text-red-800 mb-2">{t('settings.data.dangerZone')}</h3>
            <p className="text-xs text-red-600 mb-4">{t('settings.data.dangerDesc')}</p>
            <div className="flex gap-3">
              <div className="flex flex-col">
                <button onClick={clearAllData} disabled={clearingData} className="px-5 py-3 bg-red-100 text-red-700 rounded-lg text-sm hover:bg-red-200 disabled:opacity-50 transition font-medium">
                  {clearingData ? t('settings.data.clearing') : t('settings.data.clearAllData')}
                </button>
                <p className="text-xs text-red-500 mt-1">{t('settings.data.clearAllDataTooltip')}</p>
              </div>
              <div className="flex flex-col">
                <button onClick={clearAll} disabled={clearingAll} className="px-5 py-3 bg-red-600 text-white rounded-lg text-sm hover:bg-red-700 disabled:opacity-50 transition font-medium">
                  {clearingAll ? t('settings.data.clearing') : t('settings.data.clearAll')}
                </button>
                <p className="text-xs text-red-400 mt-1">{t('settings.data.clearAllTooltip')}</p>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* About */}
      {activeTab === 'about' && (
        <div>
          <h2 className="text-lg font-semibold mb-4">{t('settings.about.title')}</h2>
          <p className="text-sm text-gray-500 mb-6">{t('settings.about.description')}</p>

          <div className="space-y-6">
            {/* Version Info */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <div className="flex items-center gap-4 mb-4">
                <div className="w-16 h-16 bg-blue-600 rounded-xl flex items-center justify-center text-white text-3xl">
                  📚
                </div>
                <div>
                  <h3 className="text-xl font-bold">K-HUB</h3>
                  <p className="text-gray-500">{t('settings.about.subtitle')}</p>
                </div>
              </div>
              
              <div className="space-y-2 text-sm">
                <div className="flex justify-between py-2 border-b">
                  <span className="text-gray-500">{t('settings.about.version')}</span>
                  <span className="font-mono">2.5.2</span>
                </div>
                <div className="flex justify-between py-2 border-b">
                  <span className="text-gray-500">{t('settings.about.platform')}</span>
                  <span>{navigator.platform}</span>
                </div>
                <div className="flex justify-between py-2 border-b">
                  <span className="text-gray-500">{t('settings.about.electron')}</span>
                  <span className="font-mono">{(window as any).__TAURI_INTERNALS__ ? 'Tauri v2' : 'Browser'}</span>
                </div>
              </div>
            </div>

            {/* Links */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.about.links')}</h3>
              <div className="space-y-3">
                <a
                  href="https://github.com/lerkieYang/K-HUB"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-blue-600 hover:text-blue-800 text-sm"
                >
                  <span>📁</span>
                  <span>{t('settings.about.github')}</span>
                </a>
                <a
                  href="https://github.com/lerkieYang/K-HUB/releases"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-blue-600 hover:text-blue-800 text-sm"
                >
                  <span>📦</span>
                  <span>{t('settings.about.releases')}</span>
                </a>
                <a
                  href="https://github.com/lerkieYang/K-HUB/issues"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-blue-600 hover:text-blue-800 text-sm"
                >
                  <span>🐛</span>
                  <span>{t('settings.about.issues')}</span>
                </a>
                <a
                  href="https://github.com/lerkieYang/K-HUB/wiki"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-2 text-blue-600 hover:text-blue-800 text-sm"
                >
                  <span>📖</span>
                  <span>{t('settings.about.docs')}</span>
                </a>
              </div>
            </div>

            {/* Check for Updates */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.about.updates')}</h3>
              <p className="text-sm text-gray-500 mb-4">{t('settings.about.updatesDesc')}</p>
              <button
                onClick={() => window.open('https://github.com/lerkieYang/K-HUB/releases/latest', '_blank')}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm hover:bg-blue-700"
              >
                {t('settings.about.checkUpdates')}
              </button>
            </div>

            {/* Logs */}
            <div className="bg-white border border-gray-200 rounded-lg p-5">
              <h3 className="font-semibold mb-4">{t('settings.about.logs')}</h3>
              <p className="text-sm text-gray-500 mb-4">{t('settings.about.logsDesc')}</p>
              <button
                onClick={openLogsFolder}
                className="px-4 py-2 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm"
              >
                {t('settings.about.openLogs')}
              </button>
            </div>

            {/* Credits */}
            <div className="text-center text-sm text-gray-400 py-4">
              <p>{t('settings.about.credits')}</p>
              <p className="mt-1">{t('settings.about.license')}</p>
            </div>
          </div>
        </div>
      )}

      {/* Folder Browser Modal */}
      {showFolderBrowser && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 w-[520px] shadow-xl max-h-[70vh] flex flex-col">
            <h3 className="text-lg font-bold mb-4">{t('settings.folder.title')}</h3>
            <div className="flex gap-2 mb-3">
              <input
                type="text"
                value={folderPath}
                onChange={e => setFolderPath(e.target.value)}
                onKeyDown={e => e.key === 'Enter' && browseFolder(folderPath)}
                placeholder={t('settings.folder.inputPlaceholder')}
                className="flex-1 px-3 py-2 border rounded-lg text-sm"
              />
              <button onClick={() => browseFolder(folderPath)} className="px-3 py-2 bg-gray-100 rounded-lg text-sm">
                {t('common.search')}
              </button>
            </div>
            <div className="flex-1 overflow-y-auto border rounded-lg p-2 mb-4 min-h-[200px]">
              {folderLoading ? (
                <p className="text-center text-gray-400 py-4">{t('common.loading')}</p>
              ) : (
                folderItems.map((item, i) => (
                  <div
                    key={i}
                    onClick={() => item.is_directory && browseFolder(item.path)}
                    className="px-3 py-2 hover:bg-gray-100 rounded cursor-pointer flex items-center gap-2"
                  >
                    <span>{item.is_directory ? '📁' : '📄'}</span>
                    <span className="text-sm truncate">{item.name}</span>
                  </div>
                ))
              )}
            </div>
            <div className="flex gap-2 justify-end">
              <button onClick={() => setShowFolderBrowser(false)} className="px-4 py-2 bg-gray-100 rounded-lg text-sm">
                {t('common.cancel')}
              </button>
              <button onClick={selectFolder} className="px-4 py-2 bg-blue-600 text-white rounded-lg text-sm">
                {t('settings.folder.select')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

 import React, { useState, useEffect } from 'react';
 import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';

const SESSION_SOURCE_TYPES = ['hermes_session', 'codex_session', 'gemini_session', 'openclaw_session'];
const MEMORY_SOURCE_TYPES = ['hermes', 'codex', 'gemini', 'openclaw'];

interface Stats {
  hubStatus: 'running' | 'stopped' | 'error';
  mode: string;
  agentCount: number;

  kbStage: string;        // 'idle' | 'scanning' | 'indexing' | 'vectorizing' | 'done'
  kbScanCount: number;    // 已扫描文件数
  kbTotalFiles: number;
  kbIndexedCount: number;
  kbVectorizedCount: number;
  kbIndexProgress: number;
  kbVectorizeProgress: number;
  kbLastIndexed: string | null;

  knowledgeTotal: number;
  knowledgeEmbedded: number;
  knowledgeByType: Record<string, number>;

  memoryTotal: number;
  memoryBySource: Record<string, number>;
  memoryEmbedded: number;

  sessionsTotal: number;
  sessionsBySource: Record<string, number>;
  sessionsEmbedded: number;

  lastSyncTime: string | null;
  lastSyncSource: string | null;
}

 export function Dashboard() {
   const { mode, hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const [stats, setStats] = useState<Stats>({
    hubStatus: 'stopped',
    mode: 'standalone',
    agentCount: 0,
    kbStage: 'idle',
    kbScanCount: 0,
    kbTotalFiles: 0,
    kbIndexedCount: 0,
    kbVectorizedCount: 0,
    kbIndexProgress: 0,
    kbVectorizeProgress: 0,
    kbLastIndexed: null,
    knowledgeTotal: 0,
    knowledgeEmbedded: 0,
    knowledgeByType: {},
    memoryTotal: 0,
    memoryBySource: {},
    memoryEmbedded: 0,
    sessionsTotal: 0,
    sessionsBySource: {},
    sessionsEmbedded: 0,
    lastSyncTime: null,
    lastSyncSource: null,
  });
  const [loadingActions, setLoadingActions] = useState<Set<string>>(new Set());
  const isLoading = (action: string) => loadingActions.has(action);
  const startLoading = (action: string) => setLoadingActions(prev => new Set(prev).add(action));
  const stopLoading = (action: string) => setLoadingActions(prev => { const next = new Set(prev); next.delete(action); return next; });
  const getBaseUrl = () => getBackendUrl(hubUrl);
  const [pullResult, setPullResult] = useState<string | null>(null);
  const [memoryPullResult, setMemoryPullResult] = useState<string | null>(null);
  const [sessionPullResult, setSessionPullResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchStats();
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, [hubUrl]);

  // Agent scan runs on a separate 30-second interval (too aggressive at 2s)
  useEffect(() => {
    fetchAgentScan();
    const interval = setInterval(fetchAgentScan, 30000);
    return () => clearInterval(interval);
  }, []);


  const fetchStats = async () => {
    const baseUrl = getBaseUrl();

    try {
      const [healthRes, progressRes, memStatsRes, kbStatsRes, dataStatsRes] = await Promise.allSettled([
        fetch(`${baseUrl}/health`),
        fetch(`${baseUrl}/api/knowledge/progress`),
        fetch(`${baseUrl}/api/memory/stats`),
        fetch(`${baseUrl}/api/knowledge/stats`),
        fetch(`${baseUrl}/api/data/stats`),
      ]);

      const hubStatus = healthRes.status === 'fulfilled' && healthRes.value.ok ? 'running' : 'error';

      let kbData: any = {};
      if (progressRes.status === 'fulfilled' && progressRes.value.ok) {
        kbData = await progressRes.value.json();
      }

      let memData: any = {};
      if (memStatsRes.status === 'fulfilled' && memStatsRes.value.ok) {
        memData = await memStatsRes.value.json();
      }

      let kbStatsData: any = {};
      if (kbStatsRes.status === 'fulfilled' && kbStatsRes.value.ok) {
        kbStatsData = await kbStatsRes.value.json();
      }

      let dataStatsData: any = {};
      if (dataStatsRes.status === 'fulfilled' && dataStatsRes.value.ok) {
        dataStatsData = await dataStatsRes.value.json();
      }

      const allBySource: Record<string, number> = Object.fromEntries(
        (memData.by_source || []).map((s: any) => [s.source, s.count])
      );

      const knowledgeFromMemory = allBySource['knowledge'] || 0;

      // Split into Memory (agent sources) and Sessions
      let memTotal = 0;
      const memoryBySource: Record<string, number> = {};
      let sesTotal = 0;
      const sessionsBySource: Record<string, number> = {};

      for (const [source, count] of Object.entries(allBySource)) {
        if (source === 'knowledge') continue;
        if (MEMORY_SOURCE_TYPES.includes(source)) {
          memoryBySource[source] = count as number;
          memTotal += count as number;
        } else if (SESSION_SOURCE_TYPES.includes(source)) {
          sessionsBySource[source] = count as number;
          sesTotal += count as number;
        } else {
          memoryBySource[source] = count as number;
          memTotal += count as number;
        }
      }

      // Also merge session by_source from data stats API (sessions live in a separate table)
      if (dataStatsData.sessions?.by_source && Array.isArray(dataStatsData.sessions.by_source)) {
        for (const item of dataStatsData.sessions.by_source) {
          if (item.source && item.count) {
            sessionsBySource[item.source] = (sessionsBySource[item.source] || 0) + item.count;
          }
        }
        // Recalculate sesTotal from merged data if data stats has it
        if (dataStatsData.sessions?.count) {
          sesTotal = dataStatsData.sessions.count;
        } else {
          sesTotal = Object.values(sessionsBySource).reduce((a, b) => a + b, 0);
        }
      }

      const knowledgeTotal = kbData.total_files || kbStatsData.total_memories || knowledgeFromMemory;
      // 从 data_stats API 获取数据（基于 doc 表，与 Settings 数据页一致）
      const docStats = dataStatsData.doc || {};
      const knowledgeEmbedded = docStats.knowledge_vectorized || 0;
      // 使用 data_stats 的 memory/session 计数（与 Settings 页面一致）
      const finalMemTotal = dataStatsData.memory?.count || memTotal;
      const finalSesTotal = dataStatsData.sessions?.count || sesTotal;

      const knowledgeByType: Record<string, number> = {};
      if (kbStatsData.by_type) {
        for (const item of kbStatsData.by_type) {
          knowledgeByType[item.type || item.memory_type] = item.count;
        }
      }
      const lastSyncTime = memData.last_updated || null;
      const lastSyncSource = memData.last_source || null;

      setStats(prev => ({
        ...prev,
        hubStatus,
        mode,
        kbStage: kbData.stage || 'idle',
        kbScanCount: kbData.scan_count || 0,
        kbTotalFiles: kbData.total_files || 0,
        kbIndexedCount: kbData.indexed_count || 0,
        kbVectorizedCount: kbData.vectorized_count || 0,
        kbIndexProgress: kbData.index_progress || 0,
        kbVectorizeProgress: kbData.vectorize_progress || 0,
        kbLastIndexed: kbData.last_indexed || null,
        knowledgeTotal,
        knowledgeEmbedded,
        knowledgeByType,
        memoryTotal: finalMemTotal,
        memoryBySource,
        memoryEmbedded: docStats.memory_vectorized || 0,
        sessionsTotal: finalSesTotal,
        sessionsBySource,
        sessionsEmbedded: docStats.session_vectorized || 0,
        lastSyncTime,
        lastSyncSource,
      }));
      setLoading(false);
    } catch (error) {
      setStats(prev => ({ ...prev, hubStatus: 'error' }));
      setLoading(false);
    }
  };

  // Agent scan runs on its own interval (30s) to avoid aggressive POST requests
  const fetchAgentScan = async () => {
    const baseUrl = getBaseUrl();
    try {
      const res = await fetch(`${baseUrl}/api/agents/scan`, { method: 'POST' });
      if (res.ok) {
        const agentData = await res.json();
        setStats(prev => ({ ...prev, agentCount: agentData.agents?.length || 0 }));
      }
    } catch (error) {
      // Silently ignore agent scan failures
    }
  };

  const handleReindex = async () => {
    startLoading('indexing');
    try {
      await fetch(`${getBaseUrl()}/api/knowledge/reindex`, { method: 'POST' });
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Reindex failed:', error);
    } finally {
      setTimeout(() => stopLoading('indexing'), 3000);
    }
  };

  const handleVectorize = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    startLoading('vectorizing');
    try {
      const res = await fetch(`${getBaseUrl()}/api/knowledge/light-embed?type=knowledge`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        if (data.success) {
          const count = data.total || 0;
          if (count > 0) {
            setPullResult(t('dashboard.vectorizeStarted', { count }));
          } else {
            setPullResult(t('dashboard.vectorizeNothingToDo', { default: '所有记忆已有标签，无需处理' }));
          }
        } else {
          setPullResult(t('dashboard.vectorizeFailed', { error: data.error || t('common.error') }));
        }
      }
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Light embed failed:', error);
      setPullResult(t('dashboard.vectorizeRequestFailed'));
    } finally {
      setTimeout(() => stopLoading('vectorizing'), 3000);
    }
  };

  const handlePullAgents = async () => {
    startLoading('pulling');
    setPullResult(null);
    try {
      const res = await fetch(`${getBaseUrl()}/api/memory/pull`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        setPullResult(data.message || t('dashboard.memoryPullStarted'));
      }
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Pull agents failed:', error);
      setPullResult(t('dashboard.memoryPullFailed'));
    } finally {
      setTimeout(() => stopLoading('pulling'), 3000);
    }
  };

  const handlePullSessions = async () => {
    startLoading('pullingSessions');
    setSessionPullResult(null);
    try {
      const res = await fetch(`${getBaseUrl()}/api/session/pull`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        setSessionPullResult(data.message || t('dashboard.sessionPullStarted'));
      }
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Pull sessions failed:', error);
      setSessionPullResult(t('dashboard.sessionPullFailed'));
    } finally {
      setTimeout(() => stopLoading('pullingSessions'), 3000);
    }
  };

  const handleMemoryVectorize = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    startLoading('memoryVectorizing');
    try {
      const res = await fetch(`${getBaseUrl()}/api/knowledge/light-embed?type=memory`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        if (data.success) {
          const count = data.total || 0;
          if (count > 0) {
            setMemoryPullResult(t('dashboard.memoryVectorizeStarted', { count }));
          } else {
            setMemoryPullResult(t('dashboard.vectorizeNothingToDo', { default: '所有记忆已有标签，无需处理' }));
          }
        } else {
          setMemoryPullResult(t('dashboard.vectorizeFailed', { error: data.error || t('common.error') }));
        }
      }
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Memory vectorize failed:', error);
      setMemoryPullResult(t('dashboard.memoryVectorizeFailed'));
    } finally {
      setTimeout(() => stopLoading('memoryVectorizing'), 3000);
    }
  };

  const handleSessionVectorize = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    startLoading('sessionVectorizing');
    try {
      const res = await fetch(`${getBaseUrl()}/api/knowledge/light-embed?type=session`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        if (data.success) {
          const count = data.total || 0;
          if (count > 0) {
            setSessionPullResult(t('dashboard.sessionVectorizeStarted', { count }));
          } else {
            setSessionPullResult(t('dashboard.vectorizeNothingToDo', { default: '所有记忆已有标签，无需处理' }));
          }
        } else {
          setSessionPullResult(t('dashboard.vectorizeFailed', { error: data.error || t('common.error') }));
        }
      }
      setTimeout(fetchStats, 2000);
    } catch (error) {
      console.error('Session vectorize failed:', error);
      setSessionPullResult(t('dashboard.sessionVectorizeFailed'));
    } finally {
      setTimeout(() => stopLoading('sessionVectorizing'), 3000);
    }
  };

  const statusColor = {
    running: 'text-green-500',
    stopped: 'text-red-500',
    error: 'text-yellow-500',
  };

  const statusText: Record<string, string> = {
    running: t('dashboard.running'),
    stopped: t('dashboard.stopped'),
    error: t('dashboard.connectionFailed'),
  };

  if (loading) {
    return (
      <div className="py-6 flex items-center justify-center h-64">
        <div className="text-center">
          <div className="animate-spin inline-block w-8 h-8 border-2 border-gray-300 border-t-blue-500 rounded-full mb-3"></div>
          <p className="text-gray-500">{t('dashboard.connecting')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="py-6">

      {/* Status cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">Hub Service</p>
              <p className={`text-lg font-semibold ${statusColor[stats.hubStatus]}`}>
                {statusText[stats.hubStatus]}
              </p>
            </div>
            <div className="w-12 h-12 bg-blue-100 rounded-full flex items-center justify-center">
              <span className="text-2xl">🖥️</span>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">{t('dashboard.mode')}</p>
              <p className="text-lg font-semibold">
                {mode === 'standalone' ? t('nav.mode.standalone') : mode === 'hub' ? t('nav.mode.hub') : t('nav.mode.client')}
              </p>
            </div>
            <div className="w-12 h-12 bg-purple-100 rounded-full flex items-center justify-center">
              <span className="text-2xl">⚙️</span>
            </div>
          </div>
        </div>

        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm text-gray-500">{t('dashboard.agentCount')}</p>
              <p className="text-lg font-semibold">{stats.agentCount}</p>
            </div>
            <div className="w-12 h-12 bg-green-100 rounded-full flex items-center justify-center">
              <span className="text-2xl">🤖</span>
            </div>
          </div>
        </div>
      </div>

      {/* Last sync info */}
      {stats.lastSyncTime && (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4 mb-6 flex items-center gap-3">
          <span className="text-xl">🔄</span>
          <div>
            <p className="text-sm text-gray-500">{t('dashboard.lastSync')}</p>
            <p className="text-sm font-medium text-gray-700">
              {new Date(stats.lastSyncTime).toLocaleString('zh-CN')}
              {stats.lastSyncSource && <span className="ml-2 text-gray-400">{t('dashboard.sourcePrefix')} {stats.lastSyncSource}</span>}
            </p>
          </div>
        </div>
      )}

      {/* 知识库 — 全宽 */}
      <div className="mb-4">
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5 flex flex-col" style={{ minHeight: '220px' }}>
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-gray-900 whitespace-nowrap">{t('dashboard.knowledge')}</h2>
            <div className="flex gap-2">
              <button
                onClick={handleReindex}
                disabled={isLoading('indexing')}
                className="h-8 px-4 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('indexing') ? t('dashboard.indexing') : t('dashboard.index')}
              </button>
              <button
                onClick={handleVectorize}
                disabled={isLoading('vectorizing')}
                className="h-8 px-4 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('vectorizing') ? t('dashboard.vectorizing') : t('dashboard.vectorize')}
              </button>
            </div>
          </div>

          <div className="flex items-end gap-2 mb-3 mt-auto">
            <span className="text-4xl font-bold text-blue-600">{stats.knowledgeTotal}</span>
            <span className="text-sm text-gray-400 mb-1">{t('dashboard.knowledgeCount')}</span>
            {stats.knowledgeEmbedded > 0 && (
              <span className="text-sm text-gray-500 ml-2">
                ({t('dashboard.vectorizedPrefix')} {stats.knowledgeEmbedded})
              </span>
            )}
          </div>

          {Object.keys(stats.knowledgeByType).length > 0 && (
            <div className="flex flex-wrap gap-2 mb-3">
              {Object.entries(stats.knowledgeByType).map(([type, count]) => (
                <span key={type} className="px-3 py-1 bg-blue-50 text-blue-700 rounded-full text-xs font-medium whitespace-nowrap">
                  {type}: {count}
                </span>
              ))}
            </div>
          )}

          <div className="flex-1 grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <div className="flex justify-between mb-2">
                <span className="text-sm text-gray-600">
                  {stats.kbStage === 'scanning' ? t('dashboard.scanning') :
                   stats.kbStage === 'indexing' ? t('dashboard.indexingLocal') :
                   stats.kbStage === 'done' ? t('dashboard.allDone') :
                   t('dashboard.indexProgress')}
                </span>
                <span className="text-sm font-medium">{stats.kbIndexProgress}%</span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-3 mb-2">
                <div
                  className={`h-3 rounded-full transition-all duration-500 ${
                    stats.kbStage === 'scanning' ? 'bg-yellow-500' :
                    stats.kbStage === 'indexing' ? 'bg-blue-500' :
                    'bg-gray-400'
                  }`}
                  style={{ width: `${stats.kbIndexProgress}%` }}
                />
              </div>
              <div className="flex justify-between text-xs text-gray-400">
                {stats.kbStage === 'scanning' ? (
                  <span>{t('dashboard.scanned')} {stats.kbScanCount}</span>
                ) : (
                  <span>{t('dashboard.indexed')} {stats.kbIndexedCount}</span>
                )}
                <span>{t('dashboard.totalFiles')} {stats.kbTotalFiles}</span>
              </div>
            </div>
            <div>
              <div className="flex justify-between mb-2">
                <span className="text-sm text-gray-600">{t('dashboard.vectorizeProgress')}</span>
                <span className="text-sm font-medium">{stats.kbVectorizeProgress}%</span>
              </div>
              <div className="w-full bg-gray-200 rounded-full h-3 mb-2">
                <div
                  className="bg-green-500 h-3 rounded-full transition-all duration-500"
                  style={{ width: `${stats.kbVectorizeProgress}%` }}
                />
              </div>
              <div className="flex justify-between text-xs text-gray-400">
                <span>{t('dashboard.vectorized')} {stats.kbVectorizedCount}</span>
                <span>{t('dashboard.totalFiles')} {stats.kbTotalFiles}</span>
              </div>
            </div>
          </div>

          {stats.kbLastIndexed && (
            <p className="text-xs text-gray-400 mt-3">
              {t('dashboard.lastIndexed')} {new Date(stats.kbLastIndexed).toLocaleString('zh-CN')}
            </p>
          )}
        </div>
      </div>

      {/* 记忆 + 会话 并排等高 */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        {/* Memory section */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5 flex flex-col h-full">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-gray-900 whitespace-nowrap">{t('dashboard.memory')}</h2>
            <div className="flex gap-2">
              <button
                onClick={handlePullAgents}
                disabled={isLoading('pulling')}
                className="h-8 px-4 text-sm bg-purple-600 text-white rounded-lg hover:bg-purple-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('pulling') ? t('dashboard.pulling') : t('dashboard.pull')}
              </button>
              <button
                onClick={handleMemoryVectorize}
                disabled={isLoading('memoryVectorizing')}
                className="h-8 px-4 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('memoryVectorizing') ? t('dashboard.vectorizing') : t('dashboard.vectorize')}
              </button>
            </div>
          </div>

          <div className="flex items-end gap-2 mb-3 mt-auto">
            <span className="text-4xl font-bold text-purple-600">{stats.memoryTotal}</span>
            <span className="text-sm text-gray-400 mb-1">{t('dashboard.memoryCount')}</span>
            {stats.memoryEmbedded > 0 && (
              <span className="text-sm text-gray-500 mb-1 ml-2">({t('dashboard.vectorizedPrefix')} {stats.memoryEmbedded})</span>
            )}
          </div>

          {memoryPullResult && (
            <p className="text-xs text-green-600 mb-2 bg-green-50 rounded px-2 py-1">{memoryPullResult}</p>
          )}

          {Object.keys(stats.memoryBySource).length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {Object.entries(stats.memoryBySource).map(([source, count]) => (
                <span key={source} className="px-2 py-0.5 bg-purple-50 text-purple-700 rounded-full text-xs font-medium whitespace-nowrap">
                  {source}: {count}
                </span>
              ))}
            </div>
          )}

          {Object.keys(stats.memoryBySource).length === 0 && stats.memoryTotal === 0 && (
            <p className="text-xs text-gray-400">{t('dashboard.noMemoryData')}</p>
          )}
        </div>

        {/* Sessions section */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5 flex flex-col h-full">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold text-gray-900 whitespace-nowrap">{t('dashboard.sessions')}</h2>
            <div className="flex gap-2">
              <button
                onClick={handlePullSessions}
                disabled={isLoading('pullingSessions')}
                className="h-8 px-4 text-sm bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('pullingSessions') ? t('dashboard.sessionsPulling') : t('dashboard.sessionsPull')}
              </button>
              <button
                onClick={handleSessionVectorize}
                disabled={isLoading('sessionVectorizing')}
                className="h-8 px-4 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 transition whitespace-nowrap inline-flex items-center"
              >
                {isLoading('sessionVectorizing') ? t('dashboard.vectorizing') : t('dashboard.vectorize')}
              </button>
            </div>
          </div>

          <div className="flex items-end gap-2 mb-3 mt-auto">
            <span className="text-4xl font-bold text-indigo-600">{stats.sessionsTotal}</span>
            <span className="text-sm text-gray-400 mb-1">{t('dashboard.sessionCount')}</span>
            {stats.sessionsEmbedded > 0 && (
              <span className="text-sm text-gray-500 mb-1 ml-2">({t('dashboard.vectorizedPrefix')} {stats.sessionsEmbedded})</span>
            )}
          </div>

          {sessionPullResult && (
            <p className="text-xs text-green-600 mb-2 bg-green-50 rounded px-2 py-1">{sessionPullResult}</p>
          )}

          {Object.keys(stats.sessionsBySource).length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {Object.entries(stats.sessionsBySource).map(([source, count]) => (
                <span key={source} className="px-2 py-0.5 bg-indigo-50 text-indigo-700 rounded-full text-xs font-medium whitespace-nowrap">
                  {source}: {count}
                </span>
              ))}
            </div>
          )}

          {Object.keys(stats.sessionsBySource).length === 0 && stats.sessionsTotal === 0 && (
            <p className="text-xs text-gray-400">{t('dashboard.noSessionData')}</p>
          )}
        </div>
      </div>

      {/* 数据总览 */}
      <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-5 mb-6">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">{t('dashboard.dataOverview')}</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {/* Agent 活跃度 */}
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-3">{t('dashboard.agentActivity')}</h3>
            <div className="space-y-2">
              {['hermes', 'codex', 'gemini', 'openclaw'].map(agent => {
                const memCount = stats.memoryBySource[agent] || 0;
                const sesCount = stats.sessionsBySource[`${agent}_session`] || 0;
                const total = memCount + sesCount;
                if (total === 0) return null;
                return (
                  <div key={agent} className="flex items-center gap-3">
                    <span className="text-sm font-medium text-gray-600 w-20 capitalize">{agent}</span>
                    <div className="flex-1 flex items-center gap-2">
                      <div className="flex-1 bg-gray-100 rounded-full h-2 overflow-hidden">
                        <div
                          className="bg-purple-500 h-2 rounded-full"
                          style={{ width: `${Math.min(100, (memCount / Math.max(total, 1)) * 100)}%` }}
                        />
                      </div>
                      <span className="text-xs text-gray-500 w-24 text-right whitespace-nowrap">
                        🧠{memCount} / 💬{sesCount}
                      </span>
                    </div>
                  </div>
                );
              })}
              {Object.keys(stats.memoryBySource).length === 0 && Object.keys(stats.sessionsBySource).length === 0 && (
                <p className="text-sm text-gray-400">{t('common.noData')}</p>
              )}
            </div>
          </div>

          {/* 向量化覆盖 */}
          <div>
            <h3 className="text-sm font-medium text-gray-700 mb-3">{t('dashboard.vectorizeCoverage')}</h3>
            <div className="space-y-2">
              {[
                { label: t('dashboard.knowledge').replace('📚 ', ''), embedded: stats.knowledgeEmbedded, total: stats.knowledgeTotal, color: 'bg-blue-500' },
                { label: t('memory.title'), embedded: stats.memoryEmbedded, total: stats.memoryTotal, color: 'bg-purple-500' },
                { label: t('sessions.title'), embedded: stats.sessionsEmbedded, total: stats.sessionsTotal, color: 'bg-indigo-500' },
              ].map(item => {
                const pct = item.total > 0 ? Math.round(item.embedded / item.total * 100) : 0;
                return (
                  <div key={item.label} className="flex items-center gap-3">
                    <span className="text-xs text-gray-600 w-14 flex-shrink-0">{item.label}</span>
                    <div className="flex-1 bg-gray-100 rounded-full h-2 overflow-hidden">
                      <div
                        className={`${item.color} h-2 rounded-full transition-all duration-700`}
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                    <span className="text-xs text-gray-500 w-24 text-right flex-shrink-0">
                      {item.embedded}/{item.total}
                      <span className={`ml-1 ${pct >= 80 ? 'text-green-600' : pct >= 40 ? 'text-yellow-600' : 'text-gray-400'}`}>
                        {pct}%
                      </span>
                    </span>
                  </div>
                );
              })}

              {/* 总体覆盖率 */}
              {(() => {
                const totalAll = stats.knowledgeTotal + stats.memoryTotal + stats.sessionsTotal;
                const embeddedAll = stats.knowledgeEmbedded + stats.memoryEmbedded + stats.sessionsEmbedded;
                if (totalAll === 0) return null;
                const overallPct = Math.round(embeddedAll / totalAll * 100);
                return (
                  <div className="pt-2 mt-1 border-t border-gray-100 flex items-center gap-3">
                    <span className="text-xs font-medium text-gray-700 w-14 flex-shrink-0">{t('dashboard.grandTotal')}</span>
                    <div className="flex-1 bg-gray-100 rounded-full h-2.5 overflow-hidden">
                      <div
                        className={`h-2.5 rounded-full transition-all duration-700 ${overallPct >= 80 ? 'bg-green-500' : overallPct >= 40 ? 'bg-yellow-500' : 'bg-gray-400'}`}
                        style={{ width: `${overallPct}%` }}
                      />
                    </div>
                    <span className={`text-xs font-bold w-24 text-right flex-shrink-0 ${overallPct >= 80 ? 'text-green-600' : overallPct >= 40 ? 'text-yellow-600' : 'text-gray-500'}`}>
                      {embeddedAll}/{totalAll} {overallPct}%
                    </span>
                  </div>
                );
              })()}
            </div>
          </div>
        </div>

        {/* 知识分类分布 */}
        {Object.keys(stats.knowledgeByType).length > 0 && (
          <div className="mt-5 pt-4 border-t border-gray-100">
            <h3 className="text-sm font-medium text-gray-700 mb-3">{t('dashboard.knowledgeTypeDistribution')}</h3>
            <div className="flex flex-wrap gap-3">
              {Object.entries(stats.knowledgeByType).map(([type, count]) => (
                <div key={type} className="flex items-center gap-2 px-3 py-2 bg-gray-50 rounded-lg">
                  <span className="text-sm text-gray-700">{type}</span>
                  <span className="text-sm font-semibold text-blue-600">{count}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

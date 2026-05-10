 import React, { useState, useEffect } from 'react';
 import { useSearchParams } from 'react-router-dom';
 import { useAppStore, getBackendUrl } from '../stores/appStore';
 import { Knowledge } from './Knowledge';
 import { Memory } from './Memory';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';

type Tab = 'knowledge' | 'memory' | 'sessions';

interface SessionRecord {
  id: string;
  title: string;
  content?: string;
  content_preview?: string;
  source_type: string;
  memory_type: string;
  created_at: string;
  updated_at: string;
  tags: string[] | string;
  status: string;
}

const SOURCE_LABELS: Record<string, { label: string; icon: string }> = {
  hermes_session: { label: 'Hermes', icon: '⚡' },
  codex_session: { label: 'Codex', icon: '🔧' },
  gemini_session: { label: 'Gemini', icon: '💎' },
  openclaw_session: { label: 'OpenClaw', icon: '🐾' },
};

const TABS: { key: Tab; label: string; icon: string }[] = [
  // Note: These labels will be overridden in the Resources component via t()
  { key: 'knowledge', label: 'resources.knowledge', icon: '📚' },
  { key: 'memory', label: 'resources.memory', icon: '🧠' },
  { key: 'sessions', label: 'resources.sessions', icon: '💬' },
];

 function Sessions() {
   const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const [sessions, setSessions] = useState<SessionRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedSession, setSelectedSession] = useState<SessionRecord | null>(null);
  const [sessionContent, setSessionContent] = useState<string>('');
  const [contentLoading, setContentLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterSource, setFilterSource] = useState<string>('all');
  const [filterType, setFilterType] = useState<string>('all');
  const [pullLoading, setPullLoading] = useState(false);
  const [exportLoading, setExportLoading] = useState(false);
  const [exportFormat, setExportFormat] = useState('json');
  const [showExportModal, setShowExportModal] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [isSemanticSearch, setIsSemanticSearch] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState({ title: '', content: '', tags: '' });
  const [saving, setSaving] = useState(false);

  const SESSION_SOURCE_TYPES = 'hermes_session,codex_session,gemini_session,openclaw_session';

  useEffect(() => {
    fetchSessions();
  }, []);

  const fetchSessions = async () => {
    setLoading(true);
    setError(null);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const params = new URLSearchParams({
        source_type: SESSION_SOURCE_TYPES,
        limit: '10000',
      });
      const res = await fetch(`${baseUrl}/api/memory/search?${params}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setSessions(data.results || []);
    } catch (err: any) {
      console.error('Failed to fetch sessions:', err);
      setError(err.message || t('sessions.loadFailed'));
    } finally {
      setLoading(false);
    }
  };

  const fetchSessionContent = async (id: string) => {
    setContentLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/memory/${id}`);
      const data = await res.json();
      setSessionContent(data.content || data.content_preview || '');
    } catch (err) {
      setSessionContent(t('common.loadingFailed'));
    } finally {
      setContentLoading(false);
    }
  };

  // 批量选择相关
  const toggleSelect = (id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredSessions.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredSessions.map(s => s.id)));
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      fetchSessions();
      return;
    }

    if (isSemanticSearch) {
      await handleSemanticSearch();
      return;
    }

    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const params = new URLSearchParams({
        query: searchQuery,
        source_type: SESSION_SOURCE_TYPES,
        limit: '50',
      });
      const res = await fetch(`${baseUrl}/api/memory/search?${params}`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setSessions(data.results || []);
    } catch (err: any) {
      setError(err.message || t('sessions.searchFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleSemanticSearch = async () => {
    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/search/semantic`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: searchQuery,
          top_k: 50,
          source_type: 'hermes_session,codex_session,gemini_session,openclaw_session',
        }),
      });

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }

      const data = await res.json();
      const results = Array.isArray(data.results) ? data.results : [];
      setSessions(results);
    } catch (err: any) {
      console.error('Failed to semantic search:', err);
      setError(err.message || t('sessions.semanticSearchFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handlePullSessions = async () => {
    setPullLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/session/pull`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      const data = await res.json();
      if (data.success !== false) {
        toast.info(data.message || t('sessions.pullSuccess'));
        fetchSessions();
      } else {
        toast.error(t('sessions.pullFailed') + ': ' + (data.error || ''));
      }
    } catch (error) {
      console.error('Failed to pull sessions:', error);
      toast.error(t('sessions.pullFailed'));
    } finally {
      setPullLoading(false);
    }
  };

  const handleExport = async () => {
    setExportLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      // Determine which IDs to export
      let idsToExport: string[] = [];
      if (selectedIds.size > 0) {
        idsToExport = Array.from(selectedIds);
      } else {
        idsToExport = sessions.map(s => s.id);
      }
      const res = await fetch(`${baseUrl}/api/export/memory`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          format: exportFormat,
          ids: idsToExport.length > 0 ? idsToExport : undefined,
        }),
      });
      const data = await res.json();
      if (data.success) {
        let blobContent: string;
        if (exportFormat === 'json') {
          blobContent = JSON.stringify(data.data, null, 2);
        } else if (exportFormat === 'csv') {
          blobContent = data.data;
        } else {
          blobContent = data.data;
        }
        const blob = new Blob([blobContent], {
          type: exportFormat === 'json' ? 'application/json' :
                exportFormat === 'csv' ? 'text/csv' : 'text/markdown'
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `sessions-export.${exportFormat}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        toast.success(t('sessions.exportSuccess', { count: data.count }));
      } else {
        toast.error(t('sessions.exportFailed') + ': ' + data.error);
      }
    } catch (error) {
      console.error('Failed to export sessions:', error);
      toast.error(t('sessions.exportFailed'));
    } finally {
      setExportLoading(false);
    }
  };

  const handleVectorize = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/light-embed?type=session`, {
        method: 'POST',
      });
      const data = await res.json();
      if (data.success) {
        const count = data.total || 0;
        if (count > 0) {
          toast.info(t('sessions.vectorizeStarted', { count }));
        } else {
          toast.info(t('dashboard.vectorizeNothingToDo', { default: '所有记录已有标签，无需处理' }));
        }
      } else {
        toast.error(t('sessions.vectorizeFailed') + ': ' + (data.error || ''));
      }
    } catch (error) {
      console.error('Failed to vectorize:', error);
      toast.error(t('sessions.vectorizeFailed'));
    }
  };

  const handleSelectSession = (session: SessionRecord) => {
    setSelectedSession(session);
    fetchSessionContent(session.id);
  };

  const handleDelete = async (id: string) => {
    if (!await toast.confirm(t('sessions.deleteConfirm'))) return;
    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' });
      if (selectedSession?.id === id) {
        setSelectedSession(null);
        setSessionContent('');
      }
      fetchSessions();
    } catch (err) {
      console.error('Failed to delete session:', err);
      toast.error(t('common.deleteFailed'));
    }
  };

  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    if (!await toast.confirm(`确定删除选中的 ${selectedIds.size} 条会话？`)) return;
    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const ids = Array.from(selectedIds);
      await Promise.all(ids.map(id =>
        fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' })
      ));
      setSelectedIds(new Set());
      if (selectedSession && selectedIds.has(selectedSession.id)) {
        setSelectedSession(null);
        setSessionContent('');
      }
      fetchSessions();
    } catch (error) {
      toast.error(t('common.deleteFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleStartEdit = () => {
    if (!selectedSession) return;
    setEditForm({
      title: selectedSession.title || '',
      content: sessionContent || '',
      tags: parseTags(selectedSession.tags).join(', '),
    });
    setIsEditing(true);
  };

  const handleSaveEdit = async () => {
    if (!selectedSession) return;
    setSaving(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const tagsArray = editForm.tags
        ? editForm.tags.split(',').map(t => t.trim()).filter(t => t.length > 0)
        : [];
      const res = await fetch(`${baseUrl}/api/memory/${selectedSession.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: editForm.title,
          content: editForm.content,
          tags: tagsArray,
        }),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setIsEditing(false);
      setSelectedSession({ ...selectedSession, title: editForm.title, tags: tagsArray });
      setSessionContent(editForm.content);
      fetchSessions();
    } catch (error) {
      console.error('Failed to save:', error);
      toast.error(t('common.saveFailed'));
    } finally {
      setSaving(false);
    }
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    if (days === 0) return t('common.today');
    if (days === 1) return t('common.yesterday');
    if (days < 7) return t('common.daysAgo', { days });
    return date.toLocaleDateString('zh-CN');
  };

  const parseTags = (tags: string[] | string | undefined): string[] => {
    if (!tags) return [];
    if (Array.isArray(tags)) return tags;
    if (typeof tags === 'string') {
      try {
        const parsed = JSON.parse(tags);
        return Array.isArray(parsed) ? parsed : [];
      } catch {
        return [];
      }
    }
    return [];
  };

  const filteredSessions = sessions.filter(s => {
    if (filterSource !== 'all' && s.source_type !== filterSource) return false;
    if (filterType !== 'all' && s.memory_type !== filterType) return false;
    return true;
  });

  const groupedBySource = filteredSessions.reduce((acc, session) => {
    const src = session.source_type || 'unknown';
    if (!acc[src]) acc[src] = [];
    acc[src].push(session);
    return acc;
  }, {} as Record<string, SessionRecord[]>);

  return (
    <div className="flex" style={{ height: '100%' }}>
      {/* Left panel: session list */}
      <div className="w-1/2 border-r bg-white flex flex-col" style={{ overflow: 'hidden' }}>
        <div className="p-4 border-b">
          {/* Action buttons with count */}
          <div className="flex items-center gap-2 mb-3">
            <button
              onClick={() => {/* TODO: import sessions */}}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('sessions.importTitle')}
            >
              {t('sessions.import')}
            </button>
            <button
              onClick={handleExport}
              disabled={exportLoading}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('sessions.exportTitle')}
            >
              {t('sessions.export')}
            </button>
            <button
              onClick={handleVectorize}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('sessions.vectorizeTitle')}
            >
              {t('sessions.vectorize')}
            </button>
            <button
              onClick={handlePullSessions}
              disabled={pullLoading}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition disabled:opacity-50 whitespace-nowrap"
              title={t('sessions.pullSessionsTitle')}
            >
              {t('sessions.pullSessions')}
            </button>
            <span className="ml-auto text-sm text-gray-500">
              {t('common.total')} {filteredSessions.length} {t('common.items')}
            </span>
          </div>

          {/* Search bar */}
          <div className="flex gap-2 mb-3">
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleSearch()}
              placeholder={t('sessions.searchPlaceholder')}
              className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition"
            />
            <button
              onClick={handleSearch}
              className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm transition"
            >
              {t('common.search')}
            </button>
          </div>

          {/* Filters */}
          <div className="flex gap-2 items-center">
            <select
              value={filterSource}
              onChange={(e) => setFilterSource(e.target.value)}
              className="px-3 py-1.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
            >
              <option value="all">{t('sessions.allSources')}</option>
              {Object.entries(SOURCE_LABELS).map(([key, { label, icon }]) => (
                <option key={key} value={key}>{icon} {label}</option>
              ))}
            </select>

            <label className="flex items-center gap-1 text-sm">
              <input
                type="checkbox"
                checked={isSemanticSearch}
                onChange={(e) => setIsSemanticSearch(e.target.checked)}
                className="rounded"
              />
              {t('sessions.semanticSearch')}
            </label>

            <label className="flex items-center gap-1 text-sm ml-auto cursor-pointer">
              <input
                type="checkbox"
                checked={filteredSessions.length > 0 && filteredSessions.every(s => selectedIds.has(s.id))}
                onChange={toggleSelectAll}
                className="rounded"
              />
              {t('sessions.selectAll')}
            </label>
         </div>
       </div>

        {/* Batch operations bar (like Memory page) */}
        {selectedIds.size > 0 && (
          <div className="mx-4 mb-3 p-2 bg-blue-50 border border-blue-200 rounded-lg flex items-center justify-between">
            <span className="text-sm text-blue-700">
              {t('memory.batchSelected', { count: selectedIds.size })}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => setSelectedIds(new Set())}
                className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
              >
                {t('memory.deselectAll')}
              </button>
              <button
                onClick={handleExport}
                className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition"
              >
                {t('common.export')}
              </button>
              <button
                onClick={handleBatchDelete}
                className="px-3 py-1.5 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 transition"
              >
                {t('memory.batchDelete')}
              </button>
            </div>
          </div>
        )}

        {/* Session list */}
        <div className="flex-1 overflow-y-auto p-2">
          {loading && (
            <div className="flex items-center justify-center py-8 text-gray-500">
              <span className="animate-spin mr-2">⏳</span> {t('common.loading')}
            </div>
          )}
          {error && (
            <div className="p-4 text-red-600 bg-red-50 rounded-lg m-2">{error}</div>
          )}
          {!loading && !error && filteredSessions.length === 0 && (
            <div className="flex flex-col items-center justify-center py-12 text-gray-400">
              <span className="text-4xl mb-2">💬</span>
              <p>{t('sessions.noSessionRecords')}</p>
            </div>
          )}

          {Object.entries(groupedBySource).map(([sourceType, items]) => (
            <div key={sourceType} className="mb-4">
              <div className="px-3 py-2 text-xs font-semibold text-gray-500 uppercase tracking-wider flex items-center gap-1.5">
                <span>{SOURCE_LABELS[sourceType]?.icon || '📄'}</span>
                {SOURCE_LABELS[sourceType]?.label || sourceType}
                <span className="ml-auto bg-gray-200 text-gray-600 px-2 py-0.5 rounded-full text-xs font-normal">
                  {items.length}
                </span>
              </div>
              {items.map(session => (
                <div
                  key={session.id}
                  className={`p-3 rounded-lg cursor-pointer transition mb-1 ${
                    selectedSession?.id === session.id
                      ? 'bg-blue-50 border border-blue-200'
                      : 'hover:bg-gray-50 border border-transparent'
                  }`}
                >
                  <div className="flex items-start gap-2">
                    <input
                      type="checkbox"
                      checked={selectedIds.has(session.id)}
                      onChange={() => toggleSelect(session.id)}
                      onClick={e => e.stopPropagation()}
                      className="mt-1 rounded"
                    />
                    <div className="flex-1 min-w-0" onClick={() => handleSelectSession(session)}>
                      <div className="flex items-start justify-between">
                        <div className="flex-1 min-w-0">
                          <h3 className="text-sm font-medium text-gray-900 truncate">
                            {session.title || t('sessions.unnamedSession')}
                          </h3>
                          <p className="text-xs text-gray-500 mt-1 line-clamp-2">
                            {session.content_preview || t('sessions.noPreview')}
                          </p>
                        </div>
                        <span className="text-xs text-gray-400 ml-2 whitespace-nowrap">
                          {formatDate(session.created_at)}
                        </span>
                      </div>
                      {parseTags(session.tags).length > 0 && (
                        <div className="flex gap-1 mt-2 flex-wrap">
                          {parseTags(session.tags).slice(0, 3).map((tag, i) => (
                            <span key={i} className="px-2 py-0.5 text-xs bg-gray-100 text-gray-600 rounded-full">
                              {tag}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ))}
        </div>
      </div>

      {/* Right panel: session detail */}
      <div className="w-1/2 bg-gray-50 flex flex-col" style={{ overflow: 'hidden' }}>
        {selectedSession ? (
          <div className="flex flex-col h-full">
            <div className="p-4 bg-white border-b">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-bold truncate min-w-0 mr-4">{selectedSession.title || t('sessions.unnamedSession')}</h2>
                <div className="flex gap-2 flex-shrink-0">
                  <button
                    onClick={handleStartEdit}
                    className="w-16 px-3 py-1.5 text-sm text-center bg-gray-100 hover:bg-gray-200 rounded-lg transition"
                  >
                    {t('common.edit')}
                  </button>
                  <button
                    onClick={() => handleDelete(selectedSession.id)}
                    className="w-16 px-3 py-1.5 text-sm text-center bg-red-100 hover:bg-red-200 text-red-700 rounded-lg transition"
                  >
                    {t('common.delete')}
                  </button>
                </div>
              </div>
              {parseTags(selectedSession.tags).length > 0 && (
                <div className="flex gap-1 mt-2 flex-wrap">
                  {parseTags(selectedSession.tags).map((tag, i) => (
                    <span key={i} className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 rounded-full">
                      {tag}
                    </span>
                  ))}
                </div>
              )}
            </div>
            <div className="flex-1 overflow-y-auto p-4">
              {isEditing ? (
                <div className="space-y-4">
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">{t('resources.title_label')}</label>
                    <input
                      type="text"
                      value={editForm.title}
                      onChange={(e) => setEditForm(prev => ({ ...prev, title: e.target.value }))}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">{t('resources.content')}</label>
                    <textarea
                      value={editForm.content}
                      onChange={(e) => setEditForm(prev => ({ ...prev, content: e.target.value }))}
                      rows={20}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">{t('resources.tags')}</label>
                    <input
                      type="text"
                      value={editForm.tags}
                      onChange={(e) => setEditForm(prev => ({ ...prev, tags: e.target.value }))}
                      className="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                    />
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={handleSaveEdit}
                      disabled={saving}
                      className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition disabled:opacity-50"
                    >
                      {saving ? t('common.saving') : t('common.save')}
                    </button>
                    <button
                      onClick={() => setIsEditing(false)}
                      className="px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition"
                    >
                      {t('common.cancel')}
                    </button>
                  </div>
                </div>
              ) : contentLoading ? (
                <div className="flex items-center justify-center py-8 text-gray-500">
                  <span className="animate-spin mr-2">⏳</span> {t('common.loadingContent')}
                </div>
              ) : (
                <pre className="whitespace-pre-wrap text-sm text-gray-700 bg-white p-4 rounded-lg border font-sans">
                  {sessionContent || t('common.noContent')}
                </pre>
              )}
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-gray-400">
            <span className="text-5xl mb-4">💬</span>
            <p className="text-lg">{t('sessions.selectSession')}</p>
          </div>
        )}
      </div>
    </div>
  );
}

 export function Resources() {
   const [searchParams, setSearchParams] = useSearchParams();
  const { t } = useI18n();
  const toast = useToast();
  const tabFromUrl = searchParams.get('tab') as Tab | null;
  const validTabs: Tab[] = ['knowledge', 'memory', 'sessions'];
  const initialTab = tabFromUrl && validTabs.includes(tabFromUrl) ? tabFromUrl : 'knowledge';
  const [activeTab, setActiveTab] = useState<Tab>(initialTab);

  // Key to force child components to remount (triggers re-fetch)
  const [refreshKey, setRefreshKey] = useState(0);

  const handleTabChange = (tab: Tab) => {
    setActiveTab(tab);
    setSearchParams({ tab }, { replace: true });
  };

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-200 overflow-hidden flex flex-col" style={{ height: 'calc(100vh - 7rem)' }}>
      {/* Tab bar */}
      <div className="flex border-b border-gray-200 bg-gray-50 flex-shrink-0">
        {TABS.map(tab => (
          <button
            key={tab.key}
            onClick={() => handleTabChange(tab.key)}
            className={`flex items-center gap-2 px-6 py-3 text-sm font-medium transition-colors border-b-2 ${
              activeTab === tab.key
                ? 'border-blue-600 text-blue-600 bg-white'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:bg-gray-100'
            }`}
          >
            <span>{tab.icon}</span>
            {t(tab.label)}
          </button>
        ))}
      </div>

      {/* Tab content — fills remaining space, no outer scroll */}
      <div className="flex-1 min-h-0 overflow-hidden">
        {activeTab === 'knowledge' && <Knowledge key={`knowledge-${refreshKey}`} />}
        {activeTab === 'memory' && <Memory key={`memory-${refreshKey}`} />}
        {activeTab === 'sessions' && <Sessions key={`sessions-${refreshKey}`} />}
      </div>
    </div>
  );
}

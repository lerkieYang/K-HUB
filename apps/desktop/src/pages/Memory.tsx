import React, { useState, useEffect } from 'react';
import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';
import { open } from '@tauri-apps/api/shell';

// Memory类型定义 - matches actual API response from GET /api/memory
interface Memory {
  id: string;
  workspace_id?: string;
  title: string;
  content?: string;
  content_preview?: string;
  memory_type: string;
  scope: string;
  tags: string[] | string; // API may return JSON string or array
  source_type: string;
  source_file_path?: string;
  source_agent_id?: string;
  owner?: string;
  confidence?: number;
  visibility?: string;
  status: string;
  created_at: string;
  updated_at: string;
  related_to?: string[];
}

// Memory类型配置
const MEMORY_TYPES: Record<string, { label: string; color: string; is_knowledge: boolean }> = {
  project_background: { label: 'knowledge.type.projectBackground', color: 'bg-blue-100 text-blue-800', is_knowledge: true },
  sop: { label: 'knowledge.type.sop', color: 'bg-green-100 text-green-800', is_knowledge: true },
  rules_standards: { label: 'knowledge.type.rulesStandards', color: 'bg-purple-100 text-purple-800', is_knowledge: true },
  template: { label: 'knowledge.type.template', color: 'bg-orange-100 text-orange-800', is_knowledge: true },
  case_library: { label: 'knowledge.type.caseLibrary', color: 'bg-pink-100 text-pink-800', is_knowledge: true },
  decision: { label: 'knowledge.type.decision', color: 'bg-yellow-100 text-yellow-800', is_knowledge: false },
  episodic: { label: 'knowledge.type.episodic', color: 'bg-gray-100 text-gray-800', is_knowledge: false },
  semantic: { label: 'knowledge.type.semantic', color: 'bg-indigo-100 text-indigo-800', is_knowledge: false },
  procedural: { label: 'knowledge.type.procedural', color: 'bg-cyan-100 text-cyan-800', is_knowledge: false },
  preference: { label: 'knowledge.type.preference', color: 'bg-pink-100 text-pink-800', is_knowledge: false },
  warning: { label: 'knowledge.type.warning', color: 'bg-red-100 text-red-800', is_knowledge: false },
};

const SOURCE_TYPES: Record<string, { label: string; icon: string; category: 'agent' | 'other' }> = {
  hermes: { label: 'memory.source.hermes', icon: '⚡', category: 'agent' },
  codex: { label: 'memory.source.codex', icon: '🔧', category: 'agent' },
  gemini: { label: 'memory.source.gemini', icon: '💎', category: 'agent' },
  openclaw: { label: 'memory.source.openclaw', icon: '🐾', category: 'agent' },
  agent: { label: 'memory.source.agent', icon: '🤖', category: 'agent' },
  manual: { label: 'memory.source.manual', icon: '📥', category: 'other' },
  auto: { label: 'memory.source.auto', icon: '🔄', category: 'other' },
  wechat: { label: 'memory.source.wechat', icon: '💬', category: 'other' },
  feishu: { label: 'memory.source.feishu', icon: '📨', category: 'other' },
  browser: { label: 'memory.source.browser', icon: '🌐', category: 'other' },
  git: { label: 'memory.source.git', icon: '📝', category: 'other' },
  // Session source type variants (directly handled so lookup succeeds)
};

// source_type values that belong on the Knowledge page, NOT here
const KNOWLEDGE_SOURCE_TYPES = ['knowledge'];

// source_type values that belong on the Sessions page, NOT here
const SESSION_SOURCE_TYPES = ['hermes_session', 'codex_session', 'gemini_session', 'openclaw_session', 'session'];

// Fallback display labels for i18n keys (in case i18n fails)
const CATEGORY_LABELS: Record<string, string> = {
  'memory.category.agent': 'Agent 记忆',
  'memory.category.other': '其他',
};

const CATEGORY_DESCRIPTIONS: Record<string, string> = {
  'memory.category.agentDesc': '来自 Agent 的记忆数据',
  'memory.category.otherDesc': '手动导入或其他来源的记忆',
};

// Category definitions for the Memory page
const CATEGORIES = {
  agent: { label: 'memory.category.agent', icon: '🤖', description: 'memory.category.agentDesc' },
  other: { label: 'memory.category.other', icon: '📝', description: 'memory.category.otherDesc' },
} as const;

const CATEGORY_LABEL_FALLBACK: Record<string, string> = {
  'memory.category.agent': 'Agent 记忆',
  'memory.category.other': '其他',
  'memory.category.agentDesc': 'Agent的长期记忆与状态',
  'memory.category.otherDesc': '手动添加或自动生成的记忆',
};

export function Memory() {
  const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const API_BASE = getBackendUrl(hubUrl);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterSource, setFilterSource] = useState<string>('all');
  const [filterType, setFilterType] = useState<string>('all');
  const [selectedMemory, setSelectedMemory] = useState<Memory | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState<Partial<Memory>>({});

  // 批量选择状态
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [pullLoading, setPullLoading] = useState(false);

  // Loading & error state
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 分页状态
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [totalCount, setTotalCount] = useState(0);
  const PAGE_SIZE = 50;

  // 导出相关状态
  const [showExportModal, setShowExportModal] = useState(false);
  const [exportFormat, setExportFormat] = useState('json');
  const [exportLoading, setExportLoading] = useState(false);

  // 语义搜索相关状态
  const [isSemanticSearch, setIsSemanticSearch] = useState(false);
  const [semanticResults, setSemanticResults] = useState<any[]>([]);

  // 手动添加相关状态
  const [showManualModal, setShowManualModal] = useState(false);
  const [manualForm, setManualForm] = useState({ title: '', content: '', memory_type: 'semantic', tags: '' });
  const [manualSaving, setManualSaving] = useState(false);
  
  // 候选审核相关状态
  const [candidates, setCandidates] = useState<any[]>([]);
  const [candidatesLoading, setCandidatesLoading] = useState(false);
  const [showCandidates, setShowCandidates] = useState(false);
  const [pendingCount, setPendingCount] = useState(0);

  useEffect(() => {
    fetchMemories();
    fetchCandidates();
  }, [page, filterSource, filterType]);

  const fetchCandidates = async () => {
    setCandidatesLoading(true);
    try {
      const res = await fetch(`${API_BASE}/api/memory/candidates?limit=50`);
      if (res.ok) {
        const data = await res.json();
        setCandidates(data.candidates || []);
        setPendingCount(data.pending_count || 0);
      }
    } catch {
      // silent
    } finally {
      setCandidatesLoading(false);
    }
  };

  const handleApprove = async (id: string) => {
    try {
      const res = await fetch(`${API_BASE}/api/memory/candidates/${id}/approve`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        setCandidates(prev => prev.filter(c => c.id !== id));
        setPendingCount(prev => Math.max(0, prev - 1));
        fetchMemories();
      } else {
        toast.error(t('memory.approveFailed') + ': ' + data.error);
      }
    } catch (err: any) {
      toast.error(t('memory.approveFailed') + ': ' + err.message);
    }
  };

  const handleReject = async (id: string) => {
    if (!await toast.confirm(t('memory.rejectConfirm'))) return;
    try {
      const res = await fetch(`${API_BASE}/api/memory/candidates/${id}/reject`, { method: 'POST' });
      const data = await res.json();
      if (data.success) {
        setCandidates(prev => prev.filter(c => c.id !== id));
        setPendingCount(prev => Math.max(0, prev - 1));
      } else {
        toast.error(t('memory.rejectFailed') + ': ' + data.error);
      }
    } catch (err: any) {
      toast.error(t('memory.rejectFailed') + ': ' + err.message);
    }
  };

  /**
   * Extract memories array from API response.
   * Handles multiple possible response shapes:
   *   { memories: [...] }  /  { items: [...] }  /  { data: [...] }  /  [...]
   */
  const extractMemoriesArray = (data: any): Memory[] => {
    if (Array.isArray(data)) return data;
    if (data && Array.isArray(data.memories)) return data.memories;
    if (data && Array.isArray(data.items)) return data.items;
    if (data && Array.isArray(data.data)) return data.data;
    if (data && Array.isArray(data.results)) return data.results;
    return [];
  };

  const extractTotalCount = (data: any): number => {
    if (data && typeof data.total === 'number') return data.total;
    if (data && typeof data.total_count === 'number') return data.total_count;
    if (data && typeof data.count === 'number') return data.count;
    return 0;
  };

  const fetchMemories = async () => {
    setLoading(true);
    setError(null);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      // 有筛选时拉取更多数据，确保同页显示
      const hasFilter = filterSource !== 'all' || filterType !== 'all';
      const fetchLimit = hasFilter ? 500 : PAGE_SIZE;
      const offset = hasFilter ? 0 : (page - 1) * PAGE_SIZE;
      const params = new URLSearchParams({
        offset: String(offset),
        limit: String(fetchLimit),
      });
      if (filterSource !== 'all') params.append('source_type', filterSource);
      const url = `${baseUrl}/api/memory?${params}`;

      const res = await fetch(url);

      if (!res.ok) {
        const errorText = await res.text().catch(() => '');
        throw new Error(`HTTP ${res.status}: ${errorText || res.statusText}`);
      }

      const data = await res.json();

      const memArray = extractMemoriesArray(data);
      const total = extractTotalCount(data);

      // 后端已排除 knowledge，直接使用
      setMemories(memArray);
      setTotalCount(total);
      setTotalPages(hasFilter ? 1 : Math.ceil(total / PAGE_SIZE));
    } catch (error: any) {
      console.error('Failed to fetch memories:', error);
      setError(error.message || t('common.loadingFailed'));
      setMemories([]);
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      setPage(1);
      fetchMemories();
      return;
    }

    if (isSemanticSearch) {
      await handleSemanticSearch();
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const params = new URLSearchParams({ query: searchQuery });
      if (filterSource !== 'all') params.append('source_type', filterSource);
      if (filterType !== 'all') params.append('memory_type', filterType);

      const res = await fetch(`${baseUrl}/api/memory/search?${params}`);

      if (!res.ok) {
        const errorText = await res.text().catch(() => '');
        throw new Error(`HTTP ${res.status}: ${errorText || res.statusText}`);
      }

      const data = await res.json();
      const results = extractMemoriesArray(data).filter(m => !KNOWLEDGE_SOURCE_TYPES.includes(m.source_type));
      setMemories(results);
      setTotalPages(1);
      setTotalCount(results.length);
    } catch (error: any) {
      console.error('Failed to search memories:', error);
      setError(error.message || t('common.error'));
      setMemories([]);
    } finally {
      setLoading(false);
    }
  };

  const handleSemanticSearch = async () => {
    setLoading(true);
    setError(null);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/search/semantic`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: searchQuery,
          top_k: 20,
          source_type: filterSource !== 'all' ? filterSource : undefined,
          memory_type: filterType !== 'all' ? filterType : undefined,
        }),
      });

      if (!res.ok) {
        const errorText = await res.text().catch(() => '');
        throw new Error(`HTTP ${res.status}: ${errorText || res.statusText}`);
      }

      const data = await res.json();
      const results = (Array.isArray(data.results) ? data.results : extractMemoriesArray(data))
        .filter((m: any) => !KNOWLEDGE_SOURCE_TYPES.includes(m.source_type));
      setSemanticResults(results);
      if (results.length === 0) {
        setError(t('memory.semanticSearchNoResults'));
      }
    } catch (error: any) {
      console.error('Failed to semantic search:', error);
      setError(error.message || t('memory.semanticSearchFailed'));
      setSemanticResults([]);
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!await toast.confirm(t('memory.deleteConfirm'))) return;

    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' });
      fetchMemories();
      if (selectedMemory?.id === id) {
        setSelectedMemory(null);
      }
    } catch (error) {
      console.error('Failed to delete memory:', error);
    }
  };

  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    if (!await toast.confirm(t('memory.batchDeleteConfirm').replace('{count}', String(selectedIds.size)))) return;

    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const ids = Array.from(selectedIds);
      const res = await fetch(`${baseUrl}/api/memory/batch-delete`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ ids }),
      });
      if (!res.ok) {
        await Promise.all(ids.map(id =>
          fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' })
        ));
      }
      setSelectedIds(new Set());
      if (selectedMemory && selectedIds.has(selectedMemory.id)) {
        setSelectedMemory(null);
      }
      fetchMemories();
    } catch (error) {
      console.error('Batch delete failed:', error);
      toast.error(t('memory.batchDeleteFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handlePullFromAgents = async () => {
    setPullLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/memory/pull`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      });
      const data = await res.json();
      if (data.success !== false) {
        alert(data.message || t('memory.pullSuccess'));
        fetchMemories();
      } else {
        toast.error(t('memory.pullFailed') + (data.error ? ': ' + data.error : ''));
      }
    } catch (error) {
      console.error('Failed to pull from agents:', error);
      toast.error(t('memory.pullAgentFailed'));
    } finally {
      setPullLoading(false);
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
        alert(data.message || t('memory.pullSessionSuccess'));
        fetchMemories();
      } else {
        toast.error(t('memory.pullSessionFailed') + (data.error ? ': ' + data.error : ''));
      }
    } catch (error) {
      console.error('Failed to pull sessions:', error);
      toast.error(t('memory.pullSessionFailed'));
    } finally {
      setPullLoading(false);
    }
  };

  const handleUpdate = async () => {
    if (!selectedMemory || !editForm) return;

    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/memory/${selectedMemory.id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(editForm),
      });

      setIsEditing(false);
      fetchMemories();
    } catch (error) {
      console.error('Failed to update memory:', error);
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
        // No selection, export all filtered memories (current tab's all items)
        idsToExport = filteredMemories.map(m => m.id);
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
          // data.data is an array, stringify it
          blobContent = JSON.stringify(data.data, null, 2);
        } else if (exportFormat === 'csv') {
          // data.data is already a CSV string
          blobContent = data.data;
        } else {
          // markdown or other
          blobContent = data.data;
        }
        
        const blob = new Blob([blobContent], {
          type: exportFormat === 'json' ? 'application/json' :
                exportFormat === 'csv' ? 'text/csv' : 'text/markdown'
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `memory-export.${exportFormat}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        setShowExportModal(false);
        toast.success(t('memory.exportSuccess').replace('{count}', String(data.count)));
      } else {
        toast.error(t('memory.exportFailed') + ': ' + data.error);
      }
    } catch (error) {
      console.error('Failed to export:', error);
      toast.error(t('memory.exportFailed'));
    } finally {
      setExportLoading(false);
    }
  };

  const handleGenerateEmbeddings = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/light-embed?type=memory`, {
        method: 'POST',
      });
      const data = await res.json();

      if (data.success) {
        const count = data.total || 0;
        if (count > 0) {
          toast.info(t('memory.vectorizeStarted').replace('{count}', String(count)));
        } else {
          toast.info(t('dashboard.vectorizeNothingToDo', { default: '所有记录已有标签，无需处理' }));
        }
      } else {
        toast.error(t('memory.vectorizeFailed') + (data.error ? ': ' + data.error : ''));
      }
    } catch (error) {
      console.error('Failed to generate embeddings:', error);
      toast.error(t('memory.vectorizeFailed'));
    }
  };

  const handleManualSave = async () => {
    if (!manualForm.title.trim() || !manualForm.content.trim()) {
      toast.info(t('memory.fillTitleAndContent'));
      return;
    }
    setManualSaving(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const tagsArray = manualForm.tags
        ? manualForm.tags.split(',').map(t => t.trim()).filter(t => t.length > 0)
        : [];
      const res = await fetch(`${baseUrl}/api/memory`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          title: manualForm.title,
          content: manualForm.content,
          memory_type: manualForm.memory_type,
          source_type: 'manual',
          tags: tagsArray,
        }),
      });
      if (!res.ok) {
        const errorText = await res.text().catch(() => '');
        throw new Error(`HTTP ${res.status}: ${errorText}`);
      }
      const data = await res.json();
      if (data.error) {
        toast.error(t('memory.createFailed') + ': ' + data.error);
      } else {
        setShowManualModal(false);
        setManualForm({ title: '', content: '', memory_type: 'semantic', tags: '' });
        fetchMemories();
      }
    } catch (error: any) {
      console.error('Failed to create memory:', error);
      toast.error(t('memory.createFailed') + ': ' + (error.message || ''));
    } finally {
      setManualSaving(false);
    }
  };

  // Get the category for a source_type
  const getCategoryForSource = (sourceType: string): 'agent' | 'other' => {
    const config = SOURCE_TYPES[sourceType];
    if (config?.category) return config.category;
    return 'other';
  };

  // 过滤memories (knowledge and sessions already filtered out in fetch)
  const filteredMemories = memories.filter(m => {
    // Exclude session types
    if (SESSION_SOURCE_TYPES.includes(m.source_type)) return false;
    if (filterSource !== 'all' && m.source_type !== filterSource) return false;
    if (filterType !== 'all' && m.memory_type !== filterType) return false;
    return true;
  });

  // Categorize memories
  const agentMemories = filteredMemories.filter(m => getCategoryForSource(m.source_type) === 'agent');
  const otherMemories = filteredMemories.filter(m => getCategoryForSource(m.source_type) === 'other');

  // Group memories by category for rendering
  const groupedMemories: Record<string, Memory[]> = {};
  if (agentMemories.length > 0) groupedMemories['agent'] = agentMemories;
  if (otherMemories.length > 0) groupedMemories['other'] = otherMemories;

  const handleMemorySelect = async (memory: Memory) => {
    setSelectedMemory(memory);
    setIsEditing(false);
    // Fetch full content since list API only returns content_preview (truncated to 200 chars)
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/memory/${memory.id}`);
      if (res.ok) {
        const data = await res.json();
        const fullMemory = data.memory || data;
        setSelectedMemory(prev => {
          if (prev?.id === memory.id) {
            return { ...prev, ...fullMemory };
          }
          return prev;
        });
      }
    } catch (err) {
      console.error('Failed to fetch full memory content:', err);
    }
  };

  const getTypeConfig = (type: string) => {
    const entry = MEMORY_TYPES[type];
    return entry ? { label: t(entry.label), color: entry.color, is_knowledge: entry.is_knowledge } : { label: type, color: 'bg-gray-100 text-gray-800', is_knowledge: false };
  };
  const getSourceConfig = (sourceType: string) => {
    // Strip _session suffix for lookup
    const baseType = sourceType.replace(/_session$/, '');
    return SOURCE_TYPES[baseType] || SOURCE_TYPES[sourceType] || { label: baseType.replace(/^./, (c: string) => c.toUpperCase()), icon: '❓', category: 'other' as const };
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    if (days === 0) return t('common.today');
    if (days === 1) return t('common.yesterday');
    if (days < 7) return t('common.daysAgo').replace('{days}', String(days));
    return date.toLocaleDateString('zh-CN');
  };

  const formatFullDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString('zh-CN');
  };

  // 搜索高亮
  const highlightText = (text: string | undefined, query: string) => {
    if (!text || !query.trim()) return text || '';
    const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    const parts = text.split(regex);
    return parts.map((part, i) =>
      regex.test(part)
        ? `<mark class="bg-yellow-200 rounded px-0.5">${part}</mark>`
        : part
    ).join('');
  };

  // 解析tags字段（API可能返回JSON字符串或数组）
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
    if (selectedIds.size === filteredMemories.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredMemories.map(m => m.id)));
    }
  };

  // Render a single memory item in the list
  const renderMemoryItem = (memory: Memory) => {
    const typeConfig = getTypeConfig(memory.memory_type);
    const sourceConfig = getSourceConfig(memory.source_type);
    return (
      <div
        key={memory.id}
        onClick={() => handleMemorySelect(memory)}
        className={`p-3 rounded-lg cursor-pointer transition-colors ${
          selectedMemory?.id === memory.id
            ? 'bg-blue-50 border-blue-200 border'
            : 'bg-white hover:bg-gray-50 border border-gray-200'
        }`}
      >
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2 mr-2" onClick={e => e.stopPropagation()}>
            <input
              type="checkbox"
              checked={selectedIds.has(memory.id)}
              onChange={() => toggleSelect(memory.id)}
              className="rounded"
            />
          </div>
          <div className="flex-1">
            <h3 className="font-medium text-sm" dangerouslySetInnerHTML={{ __html: highlightText(memory.title, searchQuery) }}></h3>
            <div className="flex gap-1 mt-1 flex-wrap">
              <span className={`px-2 py-0.5 rounded text-xs ${typeConfig.color}`}>
                {t(typeConfig.label)}
              </span>
              <span className="px-2 py-0.5 rounded text-xs bg-gray-100 text-gray-600">
                {sourceConfig.icon} {t(sourceConfig.label)}
              </span>
              {memory.source_file_path && (
                <span className="text-xs text-gray-400 truncate max-w-[200px] flex items-center gap-1">
                  <span className="flex-shrink-0">📎</span>
                  <span className="truncate">{memory.source_file_path.split('/').pop()}</span>
                  <button
                    onClick={(e) => { e.stopPropagation(); open(memory.source_file_path!); }}
                    className="flex-shrink-0 text-indigo-500 hover:text-indigo-700 text-xs"
                    title={memory.source_file_path}
                  >
                    打开
                  </button>
                </span>
              )}
            </div>
            {memory.content_preview && (
              <p className="text-xs text-gray-500 mt-1 line-clamp-1" dangerouslySetInnerHTML={{ __html: highlightText(memory.content_preview, searchQuery) }}></p>
            )}
          </div>
          <span className="text-xs text-gray-400 whitespace-nowrap ml-2">{formatDate(memory.updated_at)}</span>
        </div>
      </div>
    );
  };

  // Render a category section with its list of memories
  const renderCategory = (
    key: 'agent' | 'other',
    mems: Memory[],
    allMemories: Memory[]
  ) => {
    if (mems.length === 0) return null;
    const cat = CATEGORIES[key];
    return (
      <div className="mb-6" key={key}>
        <h2 className="text-lg font-semibold mb-2 flex items-center gap-2">
          {cat.icon} {CATEGORY_LABELS[cat.label] || t(cat.label)}
          <span className="text-sm font-normal text-gray-500">({mems.length})</span>
        </h2>
        <p className="text-xs text-gray-500 mb-2">{CATEGORY_DESCRIPTIONS[cat.description] || t(cat.description)}</p>
        <div className="space-y-2">
          {mems.map(memory => renderMemoryItem(memory))}
        </div>
      </div>
    );
  };

  return (
    <div className="flex" style={{ height: '100%' }}>
      {/* 左侧：Memory列表 */}
      <div className="w-1/2 border-r bg-white flex flex-col" style={{ overflow: 'hidden' }}>
        <div className="p-4 border-b">
          {/* Action buttons with count */}
          <div className="flex items-center gap-2 mb-3">
            <button
              onClick={() => setShowManualModal(true)}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('memory.importTitle')}
            >
              📥 {t('common.import')}
            </button>
            <button
              onClick={() => setShowExportModal(true)}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('memory.exportTitle')}
            >
              📤 {t('common.export')}
            </button>
            <button
              onClick={handleGenerateEmbeddings}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('memory.vectorizeTitle')}
            >
              🔢 {t('memory.vectorize')}
            </button>
            <button
              onClick={handlePullFromAgents}
              disabled={pullLoading}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition disabled:opacity-50 whitespace-nowrap"
              title={t('memory.pullAgentsTitle')}
            >
              {pullLoading ? '⏳' : '🤖'} {t('memory.pull')}
            </button>
            <span className="ml-auto text-sm text-gray-500">
              {t('common.total')} {filteredMemories.length} {t('common.items')}
            </span>
          </div>

          {/* 待审核候选 */}
          {pendingCount > 0 && (
            <div className="mb-4">
              <button
                onClick={() => setShowCandidates(!showCandidates)}
                className="w-full flex items-center justify-between p-3 bg-amber-50 border border-amber-200 rounded-lg hover:bg-amber-100 transition"
              >
                <span className="flex items-center gap-2 text-amber-800 font-medium">
                  <span className="inline-flex items-center justify-center w-6 h-6 bg-amber-500 text-white text-xs font-bold rounded-full">
                    {pendingCount}
                  </span>
                  {t('memory.pendingCandidates')}
                </span>
                <span className="text-amber-600 text-sm">
                  {showCandidates ? t('memory.collapse') : t('memory.expand')}
                </span>
              </button>
              
              {showCandidates && (
                <div className="mt-2 border border-amber-200 rounded-lg overflow-hidden">
                  {candidatesLoading ? (
                    <div className="p-4 text-center text-gray-400">{t('common.loading')}</div>
                  ) : candidates.length === 0 ? (
                    <div className="p-4 text-center text-gray-400">{t('memory.noPendingCandidates')}</div>
                  ) : (
                    <div className="divide-y divide-amber-100">
                      {candidates.map((candidate) => (
                        <div key={candidate.id} className="p-4 bg-white hover:bg-amber-50 transition">
                          <div className="flex items-start justify-between gap-4">
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2 mb-1">
                                <span className="font-medium text-gray-900 truncate">{candidate.title}</span>
                                <span className={`px-2 py-0.5 text-xs rounded-full ${
                                  candidate.ai_review === 'rejected' 
                                    ? 'bg-red-100 text-red-700' 
                                    : 'bg-amber-100 text-amber-700'
                                }`}>
                                  {candidate.ai_review === 'rejected' ? t('memory.aiRejected') : t('memory.aiUncertain')}
                                </span>
                                <span className="text-xs text-gray-400">{candidate.memory_type}</span>
                                <span className="text-xs text-gray-400">{t('memory.from').replace('{agent}', candidate.agent_id)}</span>
                              </div>
                              
                              <p className="text-sm text-gray-600 line-clamp-2 mb-1">
                                {candidate.content_preview}
                              </p>
                              
                              {candidate.ai_reason && (
                                <p className="text-xs text-amber-600 italic">
                                  {t('memory.aiReview')} {candidate.ai_reason}
                                </p>
                              )}
                              
                              {candidate.tags && candidate.tags.length > 0 && (
                                <div className="flex gap-1 mt-1 flex-wrap">
                                  {candidate.tags.map((tag: string, i: number) => (
                                    <span key={i} className="px-1.5 py-0.5 bg-gray-100 text-gray-500 text-xs rounded">
                                      {tag}
                                    </span>
                                  ))}
                                </div>
                              )}
                            </div>
                            
                            <div className="flex items-center gap-2 shrink-0">
                              <button
                                onClick={() => handleApprove(candidate.id)}
                                className="px-3 py-1.5 bg-emerald-600 text-white text-sm rounded-lg hover:bg-emerald-700 transition"
                                title={t('memory.approveTitle')}
                              >
                                {t('memory.approve')}
                              </button>
                              <button
                                onClick={() => handleReject(candidate.id)}
                                className="px-3 py-1.5 bg-red-600 text-white text-sm rounded-lg hover:bg-red-700 transition"
                                title={t('memory.rejectTitle')}
                              >
                                {t('memory.reject')}
                              </button>
                            </div>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* 搜索和过滤 */}
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder={t('memory.searchPlaceholder')}
                className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition"
              />
              <button
                onClick={handleSearch}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm transition"
              >
                {isSemanticSearch ? '🔍 ' + t('memory.semanticSearch') : t('common.search')}
              </button>
            </div>

            <div className="flex gap-2 items-center">
              <select
                value={filterSource}
                onChange={(e) => setFilterSource(e.target.value)}
                className="px-3 py-1.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              >
                <option value="all">{t('memory.allSources')}</option>
                <optgroup label={CATEGORY_LABELS['memory.category.agent'] || t('memory.category.agent')}>
                  {Object.entries(SOURCE_TYPES).filter(([, v]) => v.category === 'agent').map(([key, config]) => (
                    <option key={key} value={key}>{config.icon} {t(config.label)}</option>
                  ))}
                </optgroup>
                <optgroup label={CATEGORY_LABELS['memory.category.other'] || t('memory.category.other')}>
                  {Object.entries(SOURCE_TYPES).filter(([, v]) => v.category === 'other').map(([key, config]) => (
                    <option key={key} value={key}>{config.icon} {t(config.label)}</option>
                  ))}
                </optgroup>
              </select>

              <select
                value={filterType}
                onChange={(e) => setFilterType(e.target.value)}
                className="px-3 py-1.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              >
                <option value="all">{t('memory.allTypes')}</option>
                {Object.entries(MEMORY_TYPES).map(([key, config]) => (
                  <option key={key} value={key}>{t(config.label)}</option>
                ))}
              </select>

              <label className="flex items-center gap-1 text-sm">
                <input
                  type="checkbox"
                  checked={isSemanticSearch}
                  onChange={(e) => setIsSemanticSearch(e.target.checked)}
                  className="rounded"
                />
                {t('memory.semanticSearch')}
              </label>

              <label className="flex items-center gap-1 text-sm ml-auto cursor-pointer">
                <input
                  type="checkbox"
                  checked={filteredMemories.length > 0 && filteredMemories.every(m => selectedIds.has(m.id))}
                  onChange={toggleSelectAll}
                  className="rounded"
                />
                {t('memory.selectAll')}
              </label>
            </div>
          </div>
        </div>

        {/* 批量操作栏 */}
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

        {/* 可滚动的Memory列表 */}
        <div className="flex-1 overflow-y-auto p-4">
          {/* 语义搜索结果 */}
          {isSemanticSearch && semanticResults.length > 0 && (
            <div className="mb-4">
              <h3 className="text-sm font-semibold mb-2">{t('memory.semanticResults')}</h3>
              <div className="space-y-2">
                {semanticResults.map((result, index) => (
                  <div
                    key={index}
                    onClick={() => {
                      const mem = filteredMemories.find(m => m.id === result.id);
                      if (mem) handleMemorySelect(mem);
                    }}
                    className="p-3 bg-yellow-50 border border-yellow-200 rounded-lg cursor-pointer hover:bg-yellow-100"
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm font-medium">{result.title}</span>
                      <span className="text-xs text-gray-400">
                        {t('memory.similarity', { score: (result.score * 100).toFixed(1) })}
                      </span>
                    </div>
                    <p className="text-xs text-gray-500 line-clamp-2">
                      {result.content_preview || result.content?.substring(0, 100)}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Memory列表（按来源分组） */}
          {Object.entries(groupedMemories).map(([category, categoryMemories]) => {
            const categoryConfig = CATEGORIES[category as keyof typeof CATEGORIES];
            return (
              <div key={category} className="mb-4">
                <h3 className="text-sm font-semibold mb-2 flex items-center gap-1 sticky top-0 bg-white py-1 z-10">
                  <span>{categoryConfig?.icon || '📝'}</span>
                  <span>{(categoryConfig?.label && CATEGORY_LABELS[categoryConfig.label]) || categoryConfig?.label || category}</span>
                  <span className="text-gray-400">({categoryMemories.length})</span>
                </h3>
                <div className="space-y-1">
                  {categoryMemories.map(memory => {
                    const sourceConfig = getSourceConfig(memory.source_type);
                    const typeConfig = getTypeConfig(memory.memory_type);
                    const isSelected = selectedMemory?.id === memory.id;
                    const isChecked = selectedIds.has(memory.id);

                    return (
                      <div
                        key={memory.id}
                        onClick={() => handleMemorySelect(memory)}
                        className={`p-2 rounded cursor-pointer transition-colors ${
                          isSelected
                            ? 'bg-blue-50 border-blue-200 border'
                            : 'hover:bg-gray-50 border border-transparent'
                        }`}
                      >
                        <div className="flex items-start gap-2">
                          <input
                            type="checkbox"
                            checked={isChecked}
                            onChange={(e) => {
                              e.stopPropagation();
                              toggleSelect(memory.id);
                            }}
                            className="mt-1 rounded"
                          />
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center justify-between">
                              <h4 className="text-sm font-medium truncate" dangerouslySetInnerHTML={{ __html: highlightText(memory.title, searchQuery) }}></h4>
                              <span className="text-xs text-gray-400 ml-2 flex-shrink-0">
                                {formatDate(memory.created_at)}
                              </span>
                            </div>
                            <div className="flex items-center gap-2 mt-1">
                              <span className={`px-1 py-0.5 rounded text-xs ${sourceConfig.category === 'agent' ? 'bg-blue-100 text-blue-800' : 'bg-gray-100 text-gray-800'}`}>
                                {sourceConfig.icon} {t(sourceConfig.label)}
                              </span>
                              <span className={`px-1 py-0.5 rounded text-xs ${typeConfig.color}`}>
                                {typeConfig.label}
                              </span>
                            </div>
                            <p className="text-xs text-gray-400 mt-1 line-clamp-2">
                              <span dangerouslySetInnerHTML={{ __html: highlightText(memory.content_preview || memory.content?.substring(0, 100), searchQuery) }}></span>
                            </p>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            );
          })}

          {/* 加载状态 */}
          {loading && (
            <div className="text-center text-gray-400 py-4">
              <div className="animate-spin inline-block w-6 h-6 border-2 border-gray-300 border-t-blue-500 rounded-full mb-2"></div>
              <p>{t('common.loading')}</p>
            </div>
          )}

          {/* 错误状态 */}
          {error && (
            <div className="text-center text-red-500 py-4">
              <p>❌ {error}</p>
              <button
                onClick={fetchMemories}
                className="mt-2 px-4 py-2 text-sm border rounded hover:bg-gray-50"
              >
                {t('common.retry')}
              </button>
            </div>
          )}

          {/* 空状态 */}
          {!loading && !error && filteredMemories.length === 0 && (
            <div className="text-center text-gray-400 py-8">
              <p className="text-4xl mb-2">🧠</p>
              <p>{t('memory.noMemoryDataAlt')}</p>
              <p className="text-sm mt-1">{t('memory.pullHint')}</p>
            </div>
          )}

          {/* 分页 */}
          {totalPages > 1 && (
            <div className="flex justify-center gap-2 mt-4">
              <button
                onClick={() => setPage(Math.max(1, page - 1))}
                disabled={page === 1}
                className="px-3 py-1 text-sm border rounded hover:bg-gray-50 disabled:opacity-50"
              >
                {t('common.back')}
              </button>
              <span className="px-3 py-1 text-sm text-gray-500">
                {page} / {totalPages} ({t('common.total')} {filteredMemories.length} {t('common.items')})
              </span>
              <button
                onClick={() => setPage(Math.min(totalPages, page + 1))}
                disabled={page === totalPages}
                className="px-3 py-1 text-sm border rounded hover:bg-gray-50 disabled:opacity-50"
              >
                {t('common.next')}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* 右侧：Memory详情 - 独立滚动 */}
      <div className="w-1/2 overflow-y-auto bg-gray-50">
        {selectedMemory ? (
          <div>
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-xl font-bold truncate min-w-0 mr-4">{selectedMemory.title}</h2>
              <div className="flex gap-2 flex-shrink-0">
                <button
                  onClick={() => {
                    setEditForm(selectedMemory);
                    setIsEditing(true);
                  }}
                  className="w-16 px-3 py-1.5 text-sm text-center bg-gray-100 hover:bg-gray-200 rounded-lg transition"
                >
                  {t('common.edit')}
                </button>
                <button
                  onClick={() => handleDelete(selectedMemory.id)}
                  className="w-16 px-3 py-1.5 text-sm text-center bg-red-100 hover:bg-red-200 text-red-700 rounded-lg transition"
                >
                  {t('common.delete')}
                </button>
              </div>
            </div>

            {/* 元数据 */}
            <div className="bg-gray-50 rounded-lg p-4 mb-4">
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-gray-500">{t('memory.detail.type')}</span>
                  <span className={`ml-2 px-2 py-0.5 rounded ${getTypeConfig(selectedMemory.memory_type).color}`}>
                    {getTypeConfig(selectedMemory.memory_type).label}
                  </span>
                </div>
                <div>
                  <span className="text-gray-500">{t('memory.detail.source')}</span>
                  <span className="ml-2">
                    {getSourceConfig(selectedMemory.source_type).icon}{' '}
                    {t(getSourceConfig(selectedMemory.source_type).label)}
                  </span>
                </div>
                <div>
                  <span className="text-gray-500">{t('memory.detail.category')}</span>
                  <span className="ml-2">
                    {CATEGORIES[getCategoryForSource(selectedMemory.source_type)].icon}{' '}
                    {CATEGORY_LABELS[CATEGORIES[getCategoryForSource(selectedMemory.source_type)].label] || t(CATEGORIES[getCategoryForSource(selectedMemory.source_type)].label)}
                  </span>
                </div>
                <div>
                  <span className="text-gray-500">{t('memory.detail.scope')}</span>
                  <span className="ml-2">{selectedMemory.scope}</span>
                </div>
                {selectedMemory.visibility && (
                  <div>
                    <span className="text-gray-500">{t('memory.detail.visibility')}</span>
                    <span className="ml-2">{selectedMemory.visibility}</span>
                  </div>
                )}
                {selectedMemory.confidence != null && (
                  <div>
                    <span className="text-gray-500">{t('memory.detail.confidence')}</span>
                    <span className="ml-2">{(selectedMemory.confidence * 100).toFixed(0)}%</span>
                  </div>
                )}
                <div>
                  <span className="text-gray-500">{t('memory.detail.status')}</span>
                  <span className="ml-2">{selectedMemory.status}</span>
                </div>
                {selectedMemory.owner && (
                  <div>
                    <span className="text-gray-500">{t('memory.detail.owner')}</span>
                    <span className="ml-2">{selectedMemory.owner}</span>
                  </div>
                )}
                <div>
                  <span className="text-gray-500">{t('memory.detail.createdAt')}</span>
                  <span className="ml-2">{formatFullDate(selectedMemory.created_at)}</span>
                </div>
                <div>
                  <span className="text-gray-500">{t('memory.detail.updatedAt')}</span>
                  <span className="ml-2">{formatFullDate(selectedMemory.updated_at)}</span>
                </div>
              </div>

              {selectedMemory.source_file_path && (
                <div className="mt-3 pt-3 border-t flex items-center gap-2">
                  <span className="text-gray-500">{t('memory.detail.filePath')}</span>
                  <span className="text-sm font-mono break-all">{selectedMemory.source_file_path}</span>
                  <button
                    onClick={() => open(selectedMemory.source_file_path!)}
                    className="flex-shrink-0 px-2 py-1 text-xs text-indigo-600 hover:text-indigo-800 hover:bg-indigo-50 rounded"
                  >
                    打开文件
                  </button>
                </div>
              )}

              {parseTags(selectedMemory.tags).length > 0 && (
                <div className="mt-3 pt-3 border-t">
                  <span className="text-gray-500">{t('memory.detail.tags')}</span>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {parseTags(selectedMemory.tags).map((tag, i) => (
                      <span key={i} className="px-2 py-0.5 bg-gray-200 rounded text-xs">
                        {tag}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* 内容 */}
            <div className="mb-2 text-sm font-medium text-gray-500">{t('memory.detail.content')}</div>
            {isEditing ? (
              <div>
                <textarea
                  value={editForm.content || ''}
                  onChange={(e) => setEditForm({ ...editForm, content: e.target.value })}
                  className="w-full h-96 p-3 border rounded-lg font-mono text-sm resize-y"
                />
                <div className="flex justify-end gap-2 mt-2">
                  <button
                    onClick={() => setIsEditing(false)}
                    className="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    onClick={handleUpdate}
                    className="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition"
                  >
                    {t('common.save')}
                  </button>
                </div>
              </div>
            ) : (
              <div className="bg-white p-4 rounded-xl border border-gray-100 shadow-sm">
                <pre className="whitespace-pre-wrap text-sm font-mono leading-relaxed">
                  {selectedMemory.content || t('memory.detail.noContent')}
                </pre>
              </div>
            )}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-gray-400 gap-3">
            <span className="text-4xl">🧠</span>
            <span className="text-lg">{t('memory.selectMemory')}</span>
            <span className="text-sm">{t('memory.clickToView')}</span>
          </div>
        )}
      </div>

      {/* 手动添加弹窗 */}
      {showManualModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 w-[520px] shadow-xl max-h-[90vh] overflow-y-auto">
            <h3 className="text-lg font-bold mb-4">{t('memory.importModal')}</h3>
            <p className="text-xs text-gray-500 -mt-2 mb-4">{t('memory.importHint')}</p>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2">{t('memory.uploadFile')}</label>
              <input
                type="file"
                accept=".md,.txt,.markdown,.text"
                onChange={(e) => {
                  const file = e.target.files?.[0];
                  if (!file) return;
                  const reader = new FileReader();
                  reader.onload = (ev) => {
                    const text = ev.target?.result as string;
                    const nameWithoutExt = file.name.replace(/\.(md|txt|markdown|text)$/i, '');
                    // Try to extract a better title from the first heading
                    const headingMatch = text.match(/^#\s+(.+)$/m);
                    const extractedTitle = headingMatch ? headingMatch[1].trim() : nameWithoutExt;
                    setManualForm(prev => ({
                      ...prev,
                      title: prev.title || extractedTitle,
                      content: text,
                    }));
                  };
                  reader.readAsText(file);
                }}
                className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm file:mr-4 file:py-1 file:px-3 file:rounded file:border-0 file:text-sm file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
              />
            </div>

            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium mb-2">{t('resources.title_label')}</label>
                <input
                  type="text"
                  value={manualForm.title}
                  onChange={(e) => setManualForm({ ...manualForm, title: e.target.value })}
                  placeholder={t('resources.title_label') + '...'}
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t('resources.content')}</label>
                <textarea
                  value={manualForm.content}
                  onChange={(e) => setManualForm({ ...manualForm, content: e.target.value })}
                  placeholder={t('resources.content') + '...'}
                  className="w-full h-48 px-3 py-2 border border-gray-200 rounded-lg text-sm font-mono resize-y focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                />
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t('memory.memoryType')}</label>
                <select
                  value={manualForm.memory_type}
                  onChange={(e) => setManualForm({ ...manualForm, memory_type: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                >
                  {Object.entries(MEMORY_TYPES).map(([key, config]) => (
                    <option key={key} value={key}>{t(config.label)}</option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t('memory.tagsLabel')}</label>
                <input
                  type="text"
                  value={manualForm.tags}
                  onChange={(e) => setManualForm({ ...manualForm, tags: e.target.value })}
                  placeholder="tag1, tag2, tag3"
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                />
              </div>
            </div>

            <div className="flex justify-end gap-2 mt-6">
              <button
                onClick={() => {
                  setShowManualModal(false);
                  setManualForm({ title: '', content: '', memory_type: 'semantic', tags: '' });
                }}
                className="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={handleManualSave}
                disabled={manualSaving}
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition"
              >
                {manualSaving ? t('common.saving') : t('common.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 导出弹窗 */}
      {showExportModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 w-96 shadow-xl">
            <h3 className="text-lg font-bold mb-4">📤 {t('memory.exportModal')}</h3>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2">{t('memory.exportFormat')}</label>
              <select
                value={exportFormat}
                onChange={(e) => setExportFormat(e.target.value)}
                className="w-full px-3 py-2 border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              >
                <option value="json">{t('memory.jsonFormat')}</option>
                <option value="markdown">{t('memory.markdownFormat')}</option>
                <option value="csv">{t('memory.csvFormat')}</option>
              </select>
            </div>

            <div className="flex justify-end gap-2">
              <button
                onClick={() => setShowExportModal(false)}
                className="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={handleExport}
                disabled={exportLoading}
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 transition"
              >
                {exportLoading ? t('memory.exporting') : t('common.export')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

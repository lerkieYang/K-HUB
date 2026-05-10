 import React, { useState, useEffect } from 'react';
 import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';

interface KnowledgeBaseConfig {
  id: string;
  name: string;
  paths: string[];
  file_patterns: string[];
  auto_index: boolean;
  last_indexed_at?: string;
}

interface KnowledgeFile {
  id: string;
  title: string;
  file_path: string;
  memory_type: string;
  source_type: string;
  tags: string[] | string;
  updated_at: string;
  content?: string;
  content_preview: string;
  source_file_path?: string;
  metadata?: string;
}

const MEMORY_TYPE_LABELS: Record<string, { labelKey: string; icon: string; color: string }> = {
  project_background: { labelKey: 'knowledge.type.projectBackground', icon: '📋', color: 'bg-blue-100 text-blue-800' },
  sop: { labelKey: 'knowledge.type.sop', icon: '📝', color: 'bg-green-100 text-green-800' },
  rules_standards: { labelKey: 'knowledge.type.rulesStandards', icon: '📏', color: 'bg-purple-100 text-purple-800' },
  template: { labelKey: 'knowledge.type.template', icon: '📄', color: 'bg-orange-100 text-orange-800' },
  case_library: { labelKey: 'knowledge.type.caseLibrary', icon: '📚', color: 'bg-pink-100 text-pink-800' },
  semantic: { labelKey: 'knowledge.type.semantic', icon: '💡', color: 'bg-yellow-100 text-yellow-800' },
};

interface EditForm {
  title: string;
  content: string;
  tags: string;
}

 export function Knowledge() {
   const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const [configs, setConfigs] = useState<KnowledgeBaseConfig[]>([]);
  const [files, setFiles] = useState<KnowledgeFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<KnowledgeFile | null>(null);
  const [fileContent, setFileContent] = useState<string>('');
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<string>('all');
  const [filterConfig, setFilterConfig] = useState<string>('all');
  const [loading, setLoading] = useState(false);
  const [isEditing, setIsEditing] = useState(false);
  const [editForm, setEditForm] = useState<EditForm>({ title: '', content: '', tags: '' });
  const [saving, setSaving] = useState(false);

  // Batch selection state (like Memory page)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  // Export modal state
  const [showExportModal, setShowExportModal] = useState(false);
  const [exportFormat, setExportFormat] = useState('json');
  const [exportLoading, setExportLoading] = useState(false);
  const [isSemanticSearch, setIsSemanticSearch] = useState(false);

  // Add knowledge base modal
  const [showAddModal, setShowAddModal] = useState(false);
  const [newName, setNewName] = useState('');
  const [newPaths, setNewPaths] = useState('');

  // 手动添加知识条目状态
  const [showManualModal, setShowManualModal] = useState(false);
  const [manualForm, setManualForm] = useState({ title: '', content: '', memory_type: 'semantic', tags: '' });
  const [manualSaving, setManualSaving] = useState(false);

  useEffect(() => {
    fetchConfigs();
    fetchFiles();
  }, []);

  // Re-fetch when filter changes
  useEffect(() => {
    fetchFiles();
  }, [filterType, filterConfig]);

  const fetchConfigs = async () => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/config`);
      const data = await res.json();
      setConfigs(data.configs || []);
    } catch (error) {
      console.error('Failed to fetch configs:', error);
    }
  };

  const fetchFiles = async () => {
    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const params = new URLSearchParams({ source_type: 'knowledge', limit: '10000' });
      if (filterType !== 'all') params.append('memory_type', filterType);

      const res = await fetch(`${baseUrl}/api/memory/search?${params}`);
      if (!res.ok) {
        console.error('Failed to fetch files:', res.status, res.statusText);
        setFiles([]);
        return;
      }
      const data = await res.json();
      let results = data.results || [];
      
      // Filter by config if selected
      if (filterConfig !== 'all') {
        const selectedConfig = configs.find(c => c.id === filterConfig);
        if (selectedConfig) {
          results = results.filter((file: KnowledgeFile) => {
            // Check if file's source_file_path starts with any of the config's paths
            return selectedConfig.paths.some(path => 
              file.source_file_path?.startsWith(path) || file.file_path?.startsWith(path)
            );
          });
        }
      }
      
      setFiles(results);
    } catch (error) {
      console.error('Failed to fetch files:', error);
    } finally {
      setLoading(false);
    }
  };

  const fetchFileContent = async (fileId: string) => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/memory/${fileId}`);
      const data = await res.json();
      setFileContent(data.content || data.content_preview || '');
    } catch (error) {
      console.error('Failed to fetch file content:', error);
      setFileContent(t('common.loadingFailed'));
    }
  };

  const handleFileSelect = (file: KnowledgeFile) => {
    setSelectedFile(file);
    fetchFileContent(file.id);
  };

  const handleReindex = async () => {
    setLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/reindex`, { method: 'POST' });
      const data = await res.json();
      toast.success(t('knowledge.reindexComplete', { count: data.files_indexed || 0 }));
      fetchFiles();
    } catch (error) {
      console.error('Failed to reindex:', error);
      toast.error(t('knowledge.reindexFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      fetchFiles();
      return;
    }

    if (isSemanticSearch) {
      await handleSemanticSearch();
      return;
    }

    try {
      const baseUrl = getBackendUrl(hubUrl);
      const params = new URLSearchParams({
        query: searchQuery,
        source_type: 'knowledge',
        limit: '50',
      });
      if (filterType !== 'all') params.append('memory_type', filterType);

      const res = await fetch(`${baseUrl}/api/memory/search?${params}`);
      const data = await res.json();
      let results = data.results || [];
      
      // Filter by config if selected
      if (filterConfig !== 'all') {
        const selectedConfig = configs.find(c => c.id === filterConfig);
        if (selectedConfig) {
          results = results.filter((file: KnowledgeFile) => {
            return selectedConfig.paths.some(path => 
              file.source_file_path?.startsWith(path) || file.file_path?.startsWith(path)
            );
          });
        }
      }
      
      setFiles(results);
    } catch (error) {
      console.error('Failed to search:', error);
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
          source_type: 'knowledge',
          memory_type: filterType !== 'all' ? filterType : undefined,
        }),
      });

      if (!res.ok) {
        throw new Error(`HTTP ${res.status}`);
      }

      const data = await res.json();
      let results = (Array.isArray(data.results) ? data.results : []) as KnowledgeFile[];
      
      // Filter by config if selected
      if (filterConfig !== 'all') {
        const selectedConfig = configs.find(c => c.id === filterConfig);
        if (selectedConfig) {
          results = results.filter((file: KnowledgeFile) => {
            return selectedConfig.paths.some(path => 
              file.source_file_path?.startsWith(path) || file.file_path?.startsWith(path)
            );
          });
        }
      }
      
      setFiles(results);
    } catch (error) {
      console.error('Failed to semantic search:', error);
      toast.error(t('knowledge.semanticSearchFailed'));
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!await toast.confirm(t('knowledge.deleteItemConfirm'))) return;

    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setSelectedFile(null);
      setFileContent('');
      fetchFiles();
    } catch (error) {
      console.error('Failed to delete knowledge item:', error);
      toast.error(t('common.deleteFailed'));
    }
  };

  // Batch delete selected items (like Memory page)
  const handleBatchDelete = async () => {
    if (selectedIds.size === 0) return;
    if (!await toast.confirm(t('knowledge.batchDeleteConfirm', { count: selectedIds.size }))) return;

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
        // Fallback: delete one by one
        await Promise.all(ids.map(id =>
          fetch(`${baseUrl}/api/memory/${id}`, { method: 'DELETE' })
        ));
      }
      setSelectedIds(new Set());
      if (selectedFile && selectedIds.has(selectedFile.id)) {
        setSelectedFile(null);
        setFileContent('');
      }
      fetchFiles();
    } catch (error) {
      console.error('Batch delete failed:', error);
      toast.error(t('knowledge.batchDeleteFailed'));
    } finally {
      setLoading(false);
    }
  };

  // Export handler (like Memory page)
  const handleExport = async () => {
    setExportLoading(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      
      // Determine which IDs to export
      let idsToExport: string[] = [];
      if (selectedIds.size > 0) {
        idsToExport = Array.from(selectedIds);
      } else {
        // No selection, export all filtered files (current tab's all items)
        idsToExport = filteredFiles.map(f => f.id);
      }
      
      const res = await fetch(`${baseUrl}/api/export/memory`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          format: exportFormat,
          source_type: 'knowledge',
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
        a.download = `knowledge-export.${exportFormat}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        setShowExportModal(false);
        toast.success(t('knowledge.exportSuccess', { count: data.count }));
      } else {
        toast.error(t('common.exportFailed') + ': ' + data.error);
      }
    } catch (error) {
      console.error('Failed to export:', error);
      toast.error(t('common.exportFailed'));
    } finally {
      setExportLoading(false);
    }
  };

  // Vectorize handler
  const handleVectorize = async () => {
    if (!await toast.confirm('此功能不完善，可能费token，慎用！确定继续？', '⚠️ 向量化警告', { danger: true })) return;
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/light-embed?type=knowledge`, {
        method: 'POST',
      });
      const data = await res.json();

      if (data.success) {
        toast.info(t('knowledge.vectorizeStarted', { count: data.total || 0 }));
      } else {
        toast.error(t('knowledge.vectorizeFailed') + ': ' + (data.error || ''));
      }
    } catch (error) {
      console.error('Failed to vectorize:', error);
      toast.error(t('knowledge.vectorizeFailed'));
    }
  };

  // Add knowledge base handler
  const handleAddKnowledgeBase = async () => {
    if (!newName.trim() || !newPaths.trim()) {
      toast.info(t('knowledge.fillNameAndPath'));
      return;
    }

    try {
      const baseUrl = getBackendUrl(hubUrl);
      const paths = newPaths.split(',').map(p => p.trim()).filter(p => p.length > 0);
      const res = await fetch(`${baseUrl}/api/knowledge/config`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newName,
          paths,
          file_patterns: ['*'],
          auto_index: true,
          auto_embed: false,
        }),
      });
      const data = await res.json();
      if (data.error) {
        toast.error(t('knowledge.addKbFailed') + ': ' + data.error);
      } else {
        toast.success(t('knowledge.addKbSuccess'));
        setShowAddModal(false);
        setNewName('');
        setNewPaths('');
        fetchConfigs();
      }
    } catch (error) {
      console.error('Failed to add knowledge base:', error);
      toast.error(t('knowledge.addKbFailed'));
    }
  };

  // 手动添加知识条目
  const handleManualSave = async () => {
    if (!manualForm.title.trim() || !manualForm.content.trim()) {
      toast.info(t('knowledge.fillTitleAndContent'));
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
          source_type: 'knowledge',
          tags: tagsArray,
        }),
      });
      if (!res.ok) {
        const errorText = await res.text().catch(() => '');
        throw new Error(`HTTP ${res.status}: ${errorText}`);
      }
      const data = await res.json();
      if (data.error) {
        toast.error(t('common.createFailed') + ': ' + data.error);
      } else {
        setShowManualModal(false);
        setManualForm({ title: '', content: '', memory_type: 'semantic', tags: '' });
        fetchFiles();
      }
    } catch (error: any) {
      console.error('Failed to create knowledge:', error);
      toast.error(t('common.createFailed') + ': ' + (error.message || ''));
    } finally {
      setManualSaving(false);
    }
  };

  // Selection helpers (like Memory page)
  const toggleSelect = (id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selectedIds.size === filteredFiles.length) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(filteredFiles.map(f => f.id)));
    }
  };

  // Per-type select all
  const toggleSelectAllForType = (typeFiles: KnowledgeFile[]) => {
    const allIds = typeFiles.map(f => f.id);
    const allSelected = allIds.every(id => selectedIds.has(id));
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (allSelected) {
        allIds.forEach(id => next.delete(id));
      } else {
        allIds.forEach(id => next.add(id));
      }
      return next;
    });
  };

  const handleStartEdit = () => {
    if (!selectedFile) return;
    setEditForm({
      title: selectedFile.title,
      content: fileContent,
      tags: parseTags(selectedFile.tags).join(', '),
    });
    setIsEditing(true);
  };

  const handleSaveEdit = async () => {
    if (!selectedFile) return;
    setSaving(true);
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const tagsArray = editForm.tags
        ? editForm.tags.split(',').map(t => t.trim()).filter(t => t.length > 0)
        : [];
      const res = await fetch(`${baseUrl}/api/memory/${selectedFile.id}`, {
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
      // Refresh the selected file data
      setSelectedFile({ ...selectedFile, title: editForm.title, tags: tagsArray });
      setFileContent(editForm.content);
      fetchFiles();
    } catch (error) {
      console.error('Failed to save:', error);
      toast.error(t('common.saveFailed'));
    } finally {
      setSaving(false);
    }
  };

  const getTypeConfig = (type: string) => {
    const entry = MEMORY_TYPE_LABELS[type];
    return entry ? { label: t(entry.labelKey), icon: entry.icon, color: entry.color } : { label: type, icon: '📄', color: 'bg-gray-100 text-gray-800' };
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

  // Parse tags (API may return JSON string or array)
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

  // Filter files
  const filteredFiles = files.filter(f => {
    if (filterType !== 'all' && f.memory_type !== filterType) return false;
    return true;
  });

  // Group files by memory_type
  const groupedFiles = filteredFiles.reduce((acc, file) => {
    const type = file.memory_type || 'semantic';
    if (!acc[type]) acc[type] = [];
    acc[type].push(file);
    return acc;
  }, {} as Record<string, KnowledgeFile[]>);

  return (
    <div className="flex" style={{ height: '100%' }}>
      {/* Left panel: file list */}
      <div className="w-1/2 border-r bg-white flex flex-col" style={{ overflow: 'hidden' }}>
        <div className="p-4 border-b">
          {/* Action buttons with count */}
          <div className="flex items-center gap-2 mb-3">
            <button
              onClick={() => setShowManualModal(true)}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('knowledge.importTitle')}
            >
              {t('knowledge.import')}
            </button>
            <button
              onClick={() => setShowExportModal(true)}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('knowledge.exportTitle')}
            >
              {t('knowledge.export')}
            </button>
            <button
              onClick={handleVectorize}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition whitespace-nowrap"
              title={t('knowledge.vectorizeTitle')}
            >
              🔢 {t('knowledge.vectorize')}
            </button>
            <button
              onClick={handleReindex}
              disabled={loading}
              className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition disabled:opacity-50 whitespace-nowrap"
              title={t('knowledge.reindexTitle')}
            >
              {loading ? '⏳' : '🔄'} {t('knowledge.reindex')}
            </button>
            <span className="ml-auto text-sm text-gray-500">
              {t('common.total')} {files.length} {t('common.items')}
            </span>
          </div>

          {/* Search and filter */}
          <div className="space-y-2">
            <div className="flex gap-2">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder={t('knowledge.searchPlaceholder')}
                className="flex-1 px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition"
              />
              <button
                onClick={handleSearch}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 text-sm transition"
              >
                {t('common.search')}
              </button>
            </div>

            <div className="flex gap-2 items-center">
              <select
                value={filterType}
                onChange={(e) => setFilterType(e.target.value)}
                className="px-3 py-1.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              >
                <option value="all">{t('knowledge.allTypes')}</option>
                {Object.entries(MEMORY_TYPE_LABELS).map(([key, config]) => (
                  <option key={key} value={key}>{config.icon} {t(config.labelKey)}</option>
                ))}
              </select>

              <select
                value={filterConfig}
                onChange={(e) => setFilterConfig(e.target.value)}
                className="px-3 py-1.5 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition max-w-[12rem] truncate"
              >
                <option value="all">{t('knowledge.allSources')}</option>
                {configs.map((config) => (
                  <option key={config.id} value={config.id}>📁 {config.paths[0] || config.name}</option>
                ))}
              </select>

              <label className="flex items-center gap-1 text-sm">
                <input
                  type="checkbox"
                  checked={isSemanticSearch}
                  onChange={(e) => setIsSemanticSearch(e.target.checked)}
                  className="rounded"
                />
                {t('knowledge.semanticSearch')}
              </label>

              {/* Select all checkbox in filter bar (like Memory page) */}
              <label className="flex items-center gap-1 text-sm ml-auto cursor-pointer">
                <input
                  type="checkbox"
                  checked={filteredFiles.length > 0 && selectedIds.size === filteredFiles.length}
                  onChange={toggleSelectAll}
                  className="rounded"
                />
                {t('knowledge.selectAll')}
              </label>
            </div>
          </div>
        </div>

        {/* Batch operations bar (like Memory page) */}
        {selectedIds.size > 0 && (
          <div className="mx-4 mb-3 p-2 bg-blue-50 border border-blue-200 rounded-lg flex items-center justify-between">
            <span className="text-sm text-blue-700">
              {t('knowledge.batchSelected', { count: selectedIds.size })}
            </span>
            <div className="flex gap-2">
              <button
                onClick={() => setSelectedIds(new Set())}
                className="px-3 py-1.5 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
              >
                {t('knowledge.deselectAll')}
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
                {t('knowledge.batchDelete')}
              </button>
            </div>
          </div>
        )}

        {/* Scrollable file list */}
        <div className="flex-1 overflow-y-auto p-4">
          <div className="space-y-4">
            {Object.entries(groupedFiles).map(([type, typeFiles]) => {
              const typeConfig = getTypeConfig(type);
              return (
                <div key={type}>
                  <h3 className="text-sm font-semibold mb-2 flex items-center gap-1 sticky top-0 bg-white py-1 z-10">
                    <span>{typeConfig.icon}</span>
                    <span>{typeConfig.label}</span>
                    <span className="text-gray-400">({typeFiles.length})</span>
                    {/* Per-type select all (like Memory page categories) */}
                    <label className="ml-auto flex items-center gap-1 text-xs text-gray-400 font-normal cursor-pointer">
                      <input
                        type="checkbox"
                        checked={typeFiles.every(f => selectedIds.has(f.id))}
                        onChange={() => toggleSelectAllForType(typeFiles)}
                        className="rounded"
                      />
                      {t('knowledge.selectAll')}
                    </label>
                  </h3>
                  <div className="space-y-1">
                    {typeFiles.map(file => {
                      const fileTags = parseTags(file.tags);
                      const isSelected = selectedFile?.id === file.id;
                      const isChecked = selectedIds.has(file.id);
                      return (
                        <div
                          key={file.id}
                          onClick={() => handleFileSelect(file)}
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
                                toggleSelect(file.id);
                              }}
                              className="mt-1 rounded"
                            />
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center justify-between">
                                <h4 className="text-sm font-medium truncate">{file.title}</h4>
                                <div className="flex items-center gap-2 ml-2 flex-shrink-0">
                                  <span className={`text-xs px-1.5 py-0.5 rounded ${
                                    file.metadata && file.metadata !== '{}' && file.metadata !== '[]'
                                      ? 'bg-green-100 text-green-700'
                                      : 'bg-gray-100 text-gray-400'
                                  }`}>
                                    {file.metadata && file.metadata !== '{}' && file.metadata !== '[]' ? '✅ 已向量化' : '⏳ 待向量化'}
                                  </span>
                                  <span className="text-xs text-gray-400">{formatDate(file.updated_at)}</span>
                                </div>
                              </div>
                              {(file.content || file.content_preview) && (
                                <p className="text-xs text-gray-500 mt-1 line-clamp-2">
                                  {(file.content || file.content_preview || '').substring(0, 150)}
                                </p>
                              )}
                              {file.source_file_path && (
                                <p className="text-xs text-gray-400 mt-1 truncate font-mono">
                                  📎 {file.source_file_path.split('/').pop() || file.source_file_path.split('\\').pop()}
                                </p>
                              )}
                              {fileTags.length > 0 && (
                                <div className="flex flex-wrap gap-1 mt-1">
                                  {fileTags.slice(0, 3).map((tag, i) => (
                                    <span key={i} className="px-1 py-0.5 bg-gray-100 rounded text-xs">
                                      {tag}
                                    </span>
                                  ))}
                                  {fileTags.length > 3 && (
                                    <span className="text-xs text-gray-400">+{fileTags.length - 3}</span>
                                  )}
                                </div>
                              )}
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })}
          </div>

          {/* Loading state */}
          {loading && (
            <div className="text-center text-gray-400 py-4">
              <div className="animate-spin inline-block w-6 h-6 border-2 border-gray-300 border-t-blue-500 rounded-full mb-2"></div>
              <p>{t('common.loading')}</p>
            </div>
          )}

          {/* Empty state */}
          {!loading && files.length === 0 && (
            <div className="text-center text-gray-400 py-8">
              <p className="text-4xl mb-2">📚</p>
              <p>{configs.length === 0 ? t('knowledge.configInSettings') : t('knowledge.noFiles')}</p>
              <p className="text-sm mt-1">{t('knowledge.importHint')}</p>
            </div>
          )}
        </div>
      </div>

      {/* Right panel: file preview - stays fixed, scrolls independently */}
      <div className="w-1/2 overflow-y-auto bg-gray-50">
        {selectedFile ? (
          <div className="p-4">
            <div className="mb-4">
              <div className="flex items-center justify-between">
                <h2 className="text-xl font-bold">{selectedFile.title}</h2>
                <div className="flex gap-2">
                  <button
                    onClick={handleStartEdit}
                    className="w-16 px-3 py-1.5 text-sm text-center bg-gray-100 hover:bg-gray-200 rounded-lg transition"
                  >
                    {t('common.edit')}
                  </button>
                  <button
                    onClick={() => handleDelete(selectedFile.id)}
                    className="w-16 px-3 py-1.5 text-sm text-center bg-red-100 hover:bg-red-200 text-red-700 rounded-lg transition"
                  >
                    {t('common.delete')}
                  </button>
                </div>
              </div>
              <div className="flex items-center gap-2 mt-2">
                <span className={`px-2 py-0.5 rounded text-xs ${getTypeConfig(selectedFile.memory_type).color}`}>
                  {getTypeConfig(selectedFile.memory_type).icon} {getTypeConfig(selectedFile.memory_type).label}
                </span>
                <span className={`px-2 py-0.5 rounded text-xs ${
                  selectedFile.metadata && selectedFile.metadata !== '{}' && selectedFile.metadata !== '[]'
                    ? 'bg-green-100 text-green-700'
                    : 'bg-gray-100 text-gray-400'
                }`}>
                  {selectedFile.metadata && selectedFile.metadata !== '{}' && selectedFile.metadata !== '[]' ? '✅ 已向量化' : '⏳ 待向量化'}
                </span>
                <span className="text-xs text-gray-400">
                  {t('knowledge.updatedAt', { date: formatDate(selectedFile.updated_at) })}
                </span>
              </div>
              {selectedFile.file_path && (
                <p className="text-xs text-gray-400 mt-1 font-mono">{selectedFile.file_path}</p>
              )}
              {selectedFile.source_file_path && (
                <p className="text-xs text-gray-400 mt-1 font-mono">📎 {selectedFile.source_file_path}</p>
              )}
              {parseTags(selectedFile.tags).length > 0 && !isEditing && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {parseTags(selectedFile.tags).map((tag, i) => (
                    <span key={i} className="px-2 py-0.5 bg-gray-100 rounded text-xs">
                      {tag}
                    </span>
                  ))}
                </div>
              )}
            </div>

            <div className="border-t pt-4">
              {isEditing ? (
                <div className="space-y-3">
                  <div>
                    <label className="block text-sm font-medium mb-1">{t('resources.title_label')}</label>
                    <input
                      type="text"
                      value={editForm.title}
                      onChange={(e) => setEditForm({ ...editForm, title: e.target.value })}
                      className="w-full px-3 py-2 border rounded text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t('knowledge.tagsLabel')}</label>
                    <input
                      type="text"
                      value={editForm.tags}
                      onChange={(e) => setEditForm({ ...editForm, tags: e.target.value })}
                      placeholder="tag1, tag2, tag3"
                      className="w-full px-3 py-2 border rounded text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium mb-1">{t('resources.content')}</label>
                    <textarea
                      value={editForm.content}
                      onChange={(e) => setEditForm({ ...editForm, content: e.target.value })}
                      className="w-full h-64 px-3 py-2 border rounded text-sm font-mono resize-y"
                    />
                  </div>
                  <div className="flex justify-end gap-2">
                    <button
                      onClick={() => setIsEditing(false)}
                      className="px-4 py-2 text-sm bg-gray-100 hover:bg-gray-200 rounded-lg transition"
                    >
                      {t('common.cancel')}
                    </button>
                    <button
                      onClick={handleSaveEdit}
                      disabled={saving}
                      className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition disabled:opacity-50"
                    >
                      {saving ? t('common.saving') : t('common.save')}
                    </button>
                  </div>
                </div>
              ) : (
                <pre className="whitespace-pre-wrap bg-white border border-gray-100 p-4 rounded-xl text-sm leading-relaxed shadow-sm">
                  {fileContent || t('common.loading')}
                </pre>
              )}
            </div>
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400">
            <div className="text-center">
              <p className="text-4xl mb-2">📚</p>
              <p>{t('knowledge.selectFile')}</p>
            </div>
          </div>
        )}
      </div>

      {/* 手动添加知识条目弹窗 */}
      {showManualModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 w-[520px] shadow-xl max-h-[90vh] overflow-y-auto">
            <h3 className="text-lg font-bold mb-4">{t('knowledge.importModal')}</h3>
            <p className="text-xs text-gray-500 -mt-2 mb-4">{t('knowledge.importHint2')}</p>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2">{t('knowledge.uploadFile')}</label>
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
                <label className="block text-sm font-medium mb-2">{t('knowledge.knowledgeType')}</label>
                <select
                  value={manualForm.memory_type}
                  onChange={(e) => setManualForm({ ...manualForm, memory_type: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                >
                  {Object.entries(MEMORY_TYPE_LABELS).map(([key, config]) => (
                    <option key={key} value={key}>{config.icon} {t(config.labelKey)}</option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium mb-2">{t('knowledge.tagsLabel')}</label>
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
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition disabled:opacity-50"
              >
                {manualSaving ? t('common.saving') : t('common.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Export modal (like Memory page) */}
      {showExportModal && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-xl p-6 w-96 shadow-xl">
            <h3 className="text-lg font-bold mb-4">📤 {t('knowledge.exportTitle')}</h3>

            <div className="mb-4">
              <label className="block text-sm font-medium mb-2">{t('knowledge.exportFormat')}</label>
              <select
                value={exportFormat}
                onChange={(e) => setExportFormat(e.target.value)}
                className="w-full px-3 py-2 border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              >
                <option value="json">{t('knowledge.jsonFormat')}</option>
                <option value="markdown">{t('knowledge.markdownFormat')}</option>
                <option value="csv">{t('knowledge.csvFormat')}</option>
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
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50"
              >
                {exportLoading ? t('knowledge.exporting') : t('common.export')}
              </button>
            </div>
          </div>
        </div>
      )}


    </div>
  );
}

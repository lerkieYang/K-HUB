 import React, { useState, useEffect } from 'react';
 import { useAppStore, getBackendUrl } from '../stores/appStore';
import { useI18n } from '../i18n';
import { useToast } from '../components/Toast';

interface Task {
  id: string;
  type: string;
  description: string;
  started_at: string;
  completed_at?: string;
  cancelled: boolean;
  queued?: boolean;
  stage: string;
  scan_count: number;
  total_files: number;
  indexed_count: number;
  vectorized_count: number;
}

 export function Tasks() {
   const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const toast = useToast();
  const [tasks, setTasks] = useState<Task[]>([]);
  const [completedTasks, setCompletedTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [stoppingIds, setStoppingIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    fetchTasks();
    const interval = setInterval(fetchTasks, 2000);
    return () => clearInterval(interval);
  }, [hubUrl]);

  const fetchTasks = async () => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const res = await fetch(`${baseUrl}/api/knowledge/tasks`);
      if (res.ok) {
        const data = await res.json();
        setTasks(data.tasks || []);
        setCompletedTasks(data.completed || []);
      }
    } catch (error) {
      console.error('Fetch tasks failed:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleStopTask = async (taskId: string) => {
    setStoppingIds(prev => new Set(prev).add(taskId));
    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/knowledge/tasks/${taskId}/stop`, { method: 'POST' });
      setTimeout(fetchTasks, 500);
    } catch (error) {
      console.error('Stop task failed:', error);
    } finally {
      setStoppingIds(prev => {
        const next = new Set(prev);
        next.delete(taskId);
        return next;
      });
    }
  };

  const handleForceStopTask = async (taskId: string) => {
    setStoppingIds(prev => new Set(prev).add(taskId));
    try {
      const baseUrl = getBackendUrl(hubUrl);
      await fetch(`${baseUrl}/api/knowledge/tasks/${taskId}/force-stop`, { method: 'POST' });
      setTimeout(fetchTasks, 500);
    } catch (error) {
      console.error('Force stop task failed:', error);
    } finally {
      setStoppingIds(prev => {
        const next = new Set(prev);
        next.delete(taskId);
        return next;
      });
    }
  };

  const handleStopAll = async (type?: string) => {
    try {
      const baseUrl = getBackendUrl(hubUrl);
      const url = type
        ? `${baseUrl}/api/knowledge/tasks/stop-all?type=${type}`
        : `${baseUrl}/api/knowledge/tasks/stop-all`;
      await fetch(url, { method: 'POST' });
      setTimeout(fetchTasks, 500);
    } catch (error) {
      console.error('Stop all tasks failed:', error);
    }
  };

  const getTaskIcon = (type: string) => {
    switch (type) {
      case 'indexing': return '📇';
      case 'vectorizing': return '🔢';
      case 'vectorizing_knowledge': return '🔢';
      case 'vectorizing_memory': return '🧠';
      case 'vectorizing_session': return '💬';
      case 'scanning': return '🔍';
      case 'pulling_sessions': return '💬';
      case 'pulling_memory': return '🧠';
      default: return '⚙️';
    }
  };

  const getTaskLabel = (type: string) => {
    switch (type) {
      case 'indexing': return '索引知识库';
      case 'vectorizing': return '向量化全部';
      case 'vectorizing_knowledge': return '向量化知识库';
      case 'vectorizing_memory': return '向量化记忆';
      case 'vectorizing_session': return '向量化会话';
      case 'scanning': return '扫描文件';
      case 'pulling_sessions': return '拉取会话';
      case 'pulling_memory': return '拉取记忆';
      default: return type;
    }
  };

  // 计算任务进度
  const getProgress = (task: Task) => {
    const { stage, scan_count, total_files, indexed_count, vectorized_count } = task;
    if (stage === 'scanning') {
      return { current: scan_count, total: 0, percentage: 0, label: `${t('tasks.scanningStatus')}... ${scan_count} ${t('tasks.agents')}` };
    }
    if (stage === 'indexing') {
      const total = total_files || 0;
      const current = indexed_count || 0;
      const pct = total > 0 ? Math.min(100, Math.round(current / total * 100)) : 0;
      return { current, total, percentage: pct, label: `${current} / ${total} (${pct}%)` };
    }
    if (stage === 'vectorizing') {
      const total = total_files || 0;
      const current = vectorized_count || 0;
      const pct = total > 0 ? Math.min(100, Math.round(current / total * 100)) : 0;
      return { current, total, percentage: pct, label: `${current} / ${total} (${pct}%)` };
    }
    if (stage === 'pulling') {
      const total = total_files || 0;
      const current = indexed_count || 0;
      const pct = total > 0 ? Math.min(100, Math.round(current / total * 100)) : 0;
      return { current, total, percentage: pct, label: `${t('tasks.agentProgress')} ${current} / ${total} ${t('tasks.agents')} (${pct}%)` };
    }
    if (stage === 'done') {
      return { current: indexed_count || 0, total: total_files || 0, percentage: 100, label: t('tasks.completed') };
    }
    return { current: 0, total: 0, percentage: 0, label: stage };
  };

  // 按类型分组（向量子类型合并为 vectorizing）
  const tasksByType: Record<string, Task[]> = {};
  tasks.forEach(task => {
    const groupType = task.type.startsWith('vectorizing') ? 'vectorizing' : task.type;
    if (!tasksByType[groupType]) tasksByType[groupType] = [];
    tasksByType[groupType].push(task);
  });

  if (loading) {
    return (
      <div className="py-6 flex items-center justify-center h-64">
        <div className="text-center">
          <div className="animate-spin inline-block w-8 h-8 border-2 border-gray-300 border-t-blue-500 rounded-full mb-3"></div>
          <p className="text-gray-500">{t('tasks.loading')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="py-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900">⚙️ {t('tasks.title')}</h1>
        <div className="flex gap-2">
          {tasks.length > 0 && (
            <>
              {Object.keys(tasksByType).map(type => (
                <button
                  key={type}
                  onClick={() => handleStopAll(type)}
                  className="px-4 py-2 text-sm bg-orange-100 text-orange-700 rounded-lg hover:bg-orange-200 transition whitespace-nowrap"
                >
                  🛑 {t('tasks.stopAllType')}{getTaskLabel(type)}
                </button>
              ))}
              <button
                onClick={() => handleStopAll()}
                className="px-4 py-2 text-sm bg-red-600 text-white rounded-lg hover:bg-red-700 transition whitespace-nowrap"
              >
                🛑 {t('tasks.stopAll')}
              </button>
            </>
          )}
        </div>
      </div>

      {tasks.length === 0 && completedTasks.length === 0 ? (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-12 text-center flex-1 flex items-center justify-center">
          <div>
            <p className="text-5xl mb-4">✅</p>
            <p className="text-lg text-gray-600 font-medium">{t('tasks.noTasks')}</p>
            <p className="text-sm text-gray-400 mt-2">{t('tasks.noTasksDesc')}</p>
          </div>
        </div>
      ) : (
        <div className="flex-1 overflow-y-auto space-y-4">
          {/* 任务统计卡片 */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">{t('tasks.running')}</p>
                  <p className="text-2xl font-bold text-blue-600">{tasks.length}</p>
                </div>
                <div className="w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center">
                  <span className="text-xl">⚡</span>
                </div>
              </div>
            </div>
            <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">{t('tasks.indexing')}</p>
                  <p className="text-2xl font-bold text-purple-600">{(tasksByType['indexing'] || []).length}</p>
                </div>
                <div className="w-10 h-10 bg-purple-100 rounded-full flex items-center justify-center">
                  <span className="text-xl">📇</span>
                </div>
              </div>
            </div>
            <div className="bg-white rounded-xl shadow-sm border border-gray-100 p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500">{t('tasks.vectorizing')}</p>
                  <p className="text-2xl font-bold text-green-600">{(tasksByType['vectorizing'] || []).length}</p>
                </div>
                <div className="w-10 h-10 bg-green-100 rounded-full flex items-center justify-center">
                  <span className="text-xl">🔢</span>
                </div>
              </div>
            </div>
          </div>

          {/* 运行中的任务 */}
          {tasks.length > 0 && (
            <div className="space-y-3">
              {Object.entries(tasksByType).map(([type, typeTasks]) => (
                <div key={type} className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                  <div className="px-4 py-2 bg-gray-50 border-b border-gray-100 flex items-center justify-between">
                    <h3 className="text-sm font-semibold text-gray-700 flex items-center gap-2">
                      {getTaskIcon(type)} {getTaskLabel(type)}
                      <span className="text-gray-400 font-normal">({typeTasks.length})</span>
                    </h3>
                    <button
                      onClick={() => handleStopAll(type)}
                      className="px-3 py-1 text-xs bg-red-100 text-red-700 rounded hover:bg-red-200 transition"
                    >
                      {t('tasks.stopAll')}
                    </button>
                  </div>
                  <div className="divide-y divide-gray-50">
                    {typeTasks.map(task => {
                      const progress = getProgress(task);
                      return (
                        <div key={task.id} className="px-4 py-3 flex items-center justify-between hover:bg-gray-50 transition">
                          <div className="flex items-center gap-3 flex-1 min-w-0">
                            <span className={`inline-block w-2.5 h-2.5 rounded-full flex-shrink-0 ${task.cancelled ? 'bg-red-500' : 'bg-green-500 animate-pulse'}`} />
                            <div className="flex-1 min-w-0">
                              <p className="text-sm font-medium text-gray-900 truncate">{task.description}</p>
                              <div className="flex items-center gap-2 mt-0.5">
                                <span className={`text-xs px-1.5 py-0.5 rounded ${
                                  task.stage === 'scanning' ? 'bg-yellow-100 text-yellow-700' :
                                  task.stage === 'indexing' ? 'bg-blue-100 text-blue-700' :
                                  task.stage === 'vectorizing' ? 'bg-green-100 text-green-700' :
                                  task.stage === 'pulling' ? 'bg-cyan-100 text-cyan-700' :
                                  task.stage === 'queued' ? 'bg-gray-100 text-gray-500' :
                                  'bg-gray-100 text-gray-500'
                                }`}>
                                  {task.stage === 'scanning' ? t('tasks.scanningStatus') :
                                   task.stage === 'indexing' ? t('tasks.indexingStatus') :
                                   task.stage === 'vectorizing' ? t('tasks.vectorizingStatus') :
                                   task.stage === 'pulling' ? t('tasks.pullingStatus') :
                                   task.stage === 'queued' ? t('tasks.queued') :
                                   task.stage}
                                </span>
                                {progress.total > 0 && (
                                  <span className="text-xs text-gray-400">
                                    {progress.current}/{progress.total} ({progress.percentage}%)
                                  </span>
                                )}
                              </div>
                              {progress.total > 0 && (
                                <div className="mt-1.5">
                                  <div className="flex-1 bg-gray-200 rounded-full h-1.5 overflow-hidden">
                                    <div
                                      className={`h-1.5 rounded-full transition-all duration-500 ${
                                        task.cancelled ? 'bg-red-400' :
                                        task.type === 'indexing' ? 'bg-blue-500' :
                                        task.type.startsWith('vectorizing') ? 'bg-green-500' :
                                        task.type === 'pulling_sessions' || task.type === 'pulling_memory' ? 'bg-cyan-500' : 'bg-yellow-500'
                                      }`}
                                      style={{ width: `${progress.percentage}%` }}
                                    />
                                  </div>
                                </div>
                              )}
                            </div>
                          </div>
                          <div className="flex items-center gap-2 ml-3">
                            {task.cancelled ? (
                              <>
                                <span className="px-2 py-1 text-xs bg-red-100 text-red-600 rounded">
                                  {t('tasks.cancelling')}
                                </span>
                                <button
                                  onClick={() => handleForceStopTask(task.id)}
                                  className="px-2 py-1 text-xs bg-red-600 text-white rounded hover:bg-red-700 transition"
                                  title={t('tasks.forceStopTitle')}
                                >
                                  {t('tasks.forceStop')}
                                </button>
                              </>
                            ) : stoppingIds.has(task.id) ? (
                              <span className="px-2 py-1 text-xs bg-yellow-100 text-yellow-600 rounded">
                                {t('tasks.stopping')}
                              </span>
                            ) : (
                              <button
                                onClick={() => handleStopTask(task.id)}
                                className="px-3 py-1 text-xs bg-red-100 text-red-700 rounded hover:bg-red-200 transition"
                              >
                                🛑 {t('tasks.stop')}
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* 已完成任务历史 */}
          {completedTasks.length > 0 && (
            <div>
              <h2 className="text-lg font-semibold text-gray-700 mb-3 flex items-center gap-2 flex-shrink-0">
                ✅ {t('tasks.completedTasks')}
                <span className="text-sm font-normal text-gray-400">({completedTasks.length})</span>
              </h2>
              <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
                <div className="divide-y divide-gray-50">
                  {completedTasks.map(task => {
                    const progress = getProgress(task);
                    return (
                      <div key={task.id} className="px-4 py-2.5 flex items-center justify-between hover:bg-gray-50 transition opacity-70">
                        <div className="flex items-center gap-3 flex-1 min-w-0">
                          <span className={`inline-block w-2.5 h-2.5 rounded-full flex-shrink-0 ${task.cancelled ? 'bg-red-400' : 'bg-gray-300'}`} />
                          <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium text-gray-700 truncate">{task.description}</p>
                            <div className="flex items-center gap-2 mt-0.5">
                              <span className="text-xs text-gray-400">
                                {task.completed_at && new Date(task.completed_at).toLocaleString('zh-CN')}
                              </span>
                              <span className={`text-xs px-1.5 py-0.5 rounded ${
                                task.cancelled ? 'bg-red-100 text-red-600' : 'bg-green-100 text-green-600'
                              }`}>
                                {task.cancelled ? t('tasks.cancelled') : t('tasks.completed')}
                              </span>
                              {progress.total > 0 && (
                                <span className="text-xs text-gray-400">
                                  {progress.current}/{progress.total}
                                </span>
                              )}
                            </div>
                          </div>
                        </div>
                        <div className="ml-3">
                          <span className="text-xs text-gray-400">{getTaskIcon(task.type)} {getTaskLabel(task.type)}</span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

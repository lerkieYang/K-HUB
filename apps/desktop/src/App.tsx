import { Routes, Route, Navigate, useLocation, useNavigate } from 'react-router-dom';
import { useState, useEffect, useRef } from 'react';
import { useAppStore, getBackendUrl } from './stores/appStore';
import { useI18n } from './i18n';
import { Dashboard } from './pages/Dashboard';
import { SetupWizard } from './pages/SetupWizard';
import { Resources } from './pages/Resources';
import { Agents } from './pages/Agents';
import { Tasks } from './pages/Tasks';
import { Settings } from './pages/Settings';
import { ToastProvider } from './components/Toast';

function App() {
  const { isConfigured, mode } = useAppStore();
  const { t, locale, setLocale } = useI18n();

  if (!isConfigured) {
    return <SetupWizard />;
  }

  return (
    <ToastProvider>
    <div className="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100">
      <nav className="bg-slate-900 border-b border-slate-700 sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6">
          <div className="flex justify-between items-center h-16">
            <div className="flex items-center flex-shrink-0">
              <a href="/" className="flex-shrink-0 flex items-center gap-2 hover:opacity-80 transition-opacity">
                <span className="text-2xl">🧠</span>
                <span className="text-lg font-bold text-white">
                  K-HUB
                </span>
              </a>
              <div className="hidden sm:ml-8 sm:flex sm:space-x-1">
                <NavLink to="/resources" label={t('nav.resources')} icon="📚" />
                <NavLink to="/agents" label={t('nav.agents')} icon="🤖" />
                <NavLink to="/tasks" label={t('nav.tasks')} icon="⚙️" />
                <NavLink to="/settings" label={t('nav.settings')} icon="🔧" />
              </div>
            </div>
            <div className="flex items-center gap-2 flex-shrink-0">
              <div className="w-48 flex justify-end">
                <TaskIndicator />
              </div>
              {/* Language switcher */}
              <button
                onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')}
                className="inline-flex items-center gap-1 px-2.5 py-1.5 text-xs font-bold bg-slate-800 text-slate-300 rounded-full hover:bg-slate-700 transition-colors"
                title={locale === 'zh' ? 'Switch to English' : '切换到中文'}
              >
                {locale === 'zh' ? '中' : 'EN'}
              </button>
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium bg-slate-800 text-slate-300 rounded-full flex-shrink-0 whitespace-nowrap">
                <span className={`w-1.5 h-1.5 rounded-full ${mode === 'standalone' ? 'bg-emerald-500' : mode === 'hub' ? 'bg-blue-500' : 'bg-amber-500'}`} />
                {mode === 'standalone' ? t('nav.mode.standalone') : mode === 'hub' ? t('nav.mode.hub') : t('nav.mode.client')}
              </span>
              <HubStatusBadge />
            </div>
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 sm:px-6 py-6">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/resources" element={<Resources />} />
          <Route path="/agents" element={<Agents />} />
          <Route path="/tasks" element={<Tasks />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/" />} />
        </Routes>
      </main>
    </div>
    </ToastProvider>
  );
}

function TaskIndicator() {
  const { hubUrl } = useAppStore();
  const navigate = useNavigate();
  const [tasks, setTasks] = useState<any[]>([]);
  const [showPopup, setShowPopup] = useState(false);
  const popupRef = useRef<HTMLDivElement>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    const fetchTasks = async () => {
      try {
        const baseUrl = getBackendUrl(hubUrl);
        const res = await fetch(`${baseUrl}/api/knowledge/tasks`);
        if (res.ok) {
          const data = await res.json();
          setTasks(data.tasks || []);
        }
      } catch {}
    };
    fetchTasks();
    const interval = setInterval(fetchTasks, 3000);
    return () => clearInterval(interval);
  }, [hubUrl]);

  // Close popup on outside click
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (popupRef.current && !popupRef.current.contains(e.target as Node)) {
        setShowPopup(false);
      }
    };
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  const activeTasks = tasks.filter(t => !t.cancelled);
  if (activeTasks.length === 0) return null;

  const running = activeTasks.filter(t => !t.queued);
  const queued = activeTasks.filter(t => t.queued);

  const typeLabels: Record<string, string> = {
    indexing: '📇索引',
    vectorizing: '🔢向量化',
    vectorizing_knowledge: '🔢知识库向量化',
    vectorizing_memory: '🧠记忆向量化',
    vectorizing_session: '💬会话向量化',
    pulling_sessions: '💬拉取会话',
    pulling_memory: '🧠拉取记忆',
  };

  const stageLabels: Record<string, string> = {
    scanning: '扫描中',
    indexing: '索引中',
    vectorizing: '向量化中',
    pulling: '拉取中',
    queued: '排队中',
    done: '完成',
  };

  // Unified task label: "动作+名词" format
  const taskLabel = (task: any): string => {
    const labels: Record<string, string> = {
      indexing: '索引知识库',
      vectorizing: '向量化全部',
      vectorizing_knowledge: '向量化知识库',
      vectorizing_memory: '向量化记忆',
      vectorizing_session: '向量化会话',
      pulling_sessions: '拉取会话',
      pulling_memory: '拉取记忆',
    };
    return labels[task.type] || task.description || task.type;
  };

  const handleMouseEnter = () => {
    hoverTimerRef.current = setTimeout(() => setShowPopup(true), 300);
  };

  const handleMouseLeave = () => {
    clearTimeout(hoverTimerRef.current);
    setShowPopup(false);
  };

  // Badge display logic
  const showTasks = activeTasks.length > 3 ? activeTasks.slice(0, 2) : activeTasks.slice(0, 3);
  const badgeRemaining = activeTasks.length > 3 ? activeTasks.length - 2 : 0;

  return (
    <div className="relative" ref={popupRef} onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave}>
      <button
        onClick={() => navigate('/tasks')}
        className="hover:bg-slate-700 rounded transition-colors cursor-pointer px-2 py-1"
      >
        <div className="flex flex-col gap-1">
          {/* Task rows */}
          {showTasks.map(task => {
            const pct = task.total_files > 0 ? Math.round((task.vectorized_count || task.indexed_count || 0) / task.total_files * 100) : 0;
            return (
              <div key={task.id} className="flex items-center gap-2">
                <span className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${task.queued ? 'bg-slate-500' : 'bg-green-500 animate-pulse'}`} />
                <span className="text-[11px] text-amber-300 whitespace-nowrap">{taskLabel(task)}</span>
                {task.total_files > 0 && (
                  <span className="text-[10px] text-slate-400 ml-auto">{pct}%</span>
                )}
              </div>
            );
          })}
          {/* Remaining count */}
          {badgeRemaining > 0 && (
            <div className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-500 flex-shrink-0" />
              <span className="text-[11px] text-slate-400">+{badgeRemaining} 更多</span>
            </div>
          )}
        </div>
      </button>

      {/* Hover popup — 3 rows max, one task per row */}
      {showPopup && (
        <div className="absolute right-0 top-full mt-2 w-72 bg-white rounded-xl shadow-2xl border border-gray-200 overflow-hidden z-50">
          <div className="px-4 py-3 bg-slate-50 border-b border-gray-200 flex items-center justify-between">
            <div>
              <p className="text-sm font-semibold text-gray-900">任务中心</p>
              <p className="text-xs text-gray-500">{running.length} 运行中 · {queued.length} 排队中</p>
            </div>
            <button onClick={() => { navigate('/tasks'); setShowPopup(false); }} className="text-xs text-indigo-600 hover:text-indigo-700 font-medium">
              查看全部 →
            </button>
          </div>
          
          {/* Task list — max 5 rows */}
          <div className="divide-y divide-gray-100">
            {/* Running tasks first */}
            {running.slice(0, 5).map(task => {
              const pct = task.total_files > 0 ? Math.round((task.vectorized_count || task.indexed_count || 0) / task.total_files * 100) : 0;
              return (
                <div key={task.id} className="px-4 py-2.5 hover:bg-gray-50 transition flex items-center gap-3">
                  <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse flex-shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-xs font-medium text-gray-900 truncate">{taskLabel(task)}</span>
                      <div className="flex items-center gap-1.5 flex-shrink-0 ml-2">
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-green-100 text-green-700">
                          {stageLabels[task.stage] || task.stage}
                        </span>
                        <button
                          onClick={async (e) => {
                            e.stopPropagation();
                            try {
                              const baseUrl = getBackendUrl(hubUrl);
                              await fetch(`${baseUrl}/api/knowledge/tasks/${task.id}/stop`, { method: 'POST' });
                            } catch {}
                          }}
                          className="w-5 h-5 rounded-full bg-red-100 text-red-600 hover:bg-red-200 flex items-center justify-center text-[10px] font-bold transition"
                          title="停止任务"
                        >✕</button>
                      </div>
                    </div>
                    {task.total_files > 0 && (
                      <div className="flex items-center gap-2">
                        <div className="flex-1 bg-gray-200 rounded-full h-1 overflow-hidden">
                          <div className="bg-green-500 h-1 rounded-full transition-all" style={{ width: `${Math.min(100, pct)}%` }} />
                        </div>
                        <span className="text-[10px] text-gray-400 w-16 text-right">{task.indexed_count || 0}/{task.total_files}</span>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
            
            {/* Queued tasks — fill remaining rows up to 5 total */}
            {queued.slice(0, Math.max(0, 5 - running.length)).map(task => (
              <div key={task.id} className="px-4 py-2.5 hover:bg-gray-50 transition flex items-center gap-3">
                <span className="w-2 h-2 rounded-full bg-gray-300 flex-shrink-0" />
                <div className="flex-1 min-w-0 flex items-center justify-between">
                  <div>
                    <span className="text-xs font-medium text-gray-600 truncate block">{taskLabel(task)}</span>
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500 inline-block mt-0.5">排队中</span>
                  </div>
                  <button
                    onClick={async (e) => {
                      e.stopPropagation();
                      try {
                        const baseUrl = getBackendUrl(hubUrl);
                        await fetch(`${baseUrl}/api/knowledge/tasks/${task.id}/stop`, { method: 'POST' });
                      } catch {}
                    }}
                    className="w-5 h-5 rounded-full bg-red-100 text-red-600 hover:bg-red-200 flex items-center justify-center text-[10px] font-bold transition flex-shrink-0 ml-2"
                    title="停止任务"
                  >✕</button>
                </div>
              </div>
            ))}
            
            {/* Remaining tasks summary */}
            {activeTasks.length > 5 && (
              <div className="px-4 py-2 bg-gray-50 text-center">
                <span className="text-[11px] text-gray-400">还有 {activeTasks.length - 5} 个任务...</span>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

function NavLink({ to, label, icon }: { to: string; label: string; icon: string }) {
  const location = useLocation();
  const isActive = location.pathname === to;

  return (
    <a
      href={to}
      className={`inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
        isActive
          ? 'bg-slate-700 text-white'
          : 'text-slate-300 hover:text-white hover:bg-slate-800'
      }`}
    >
      <span>{icon}</span>
      {label}
    </a>
  );
}

function HubStatusBadge() {
  const { hubUrl } = useAppStore();
  const { t } = useI18n();
  const [online, setOnline] = useState<boolean | null>(null);

  useEffect(() => {
    const checkHealth = async () => {
      try {
        const baseUrl = getBackendUrl(hubUrl);
        const res = await fetch(`${baseUrl}/health`, { signal: AbortSignal.timeout(3000) });
        setOnline(res.ok);
      } catch {
        setOnline(false);
      }
    };
    checkHealth();
    const interval = setInterval(checkHealth, 5000);
    return () => clearInterval(interval);
  }, [hubUrl]);

  const label = online === null ? '...' : online ? 'HUB' : 'HUB';
  const dotColor = online === null ? 'bg-gray-400 animate-pulse' : online ? 'bg-emerald-500' : 'bg-red-500';
  const textColor = online === null ? 'text-gray-400' : online ? 'text-emerald-300' : 'text-red-300';

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-1.5 text-xs font-medium bg-slate-800 ${textColor} rounded-full flex-shrink-0 whitespace-nowrap`}
      title={online ? t('nav.hubOnline') : online === false ? t('nav.hubOffline') : t('nav.hubChecking')}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${dotColor}`} />
      {label}
    </span>
  );
}

export default App;

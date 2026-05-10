use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use once_cell::sync::Lazy;
use tokio::sync::{Mutex, Notify};
use serde_json::{json, Value};

/// 共享的取消标志
pub type CancelFlag = Arc<AtomicBool>;

/// 任务信息
pub struct TaskInfo {
    pub id: String,
    pub task_type: String,   // "indexing" | "vectorizing" | "pulling_sessions" | "pulling_memory"
    pub description: String,
    pub started_at: String,
    pub cancel_flag: CancelFlag,
    pub queued: bool,        // 是否在排队等待
    // 进度追踪
    pub stage: Arc<Mutex<String>>,           // "scanning" | "indexing" | "vectorizing" | "done" | "queued"
    pub scan_count: Arc<AtomicU64>,          // 已扫描文件数
    pub total_files: Arc<AtomicU64>,         // 扫描完成后才知道的总数
    pub indexed_count: Arc<AtomicU64>,       // 已索引数（实时更新）
    pub vectorized_count: Arc<AtomicU64>,    // 已向量化数（实时更新）
}

/// 已完成任务记录
#[derive(Clone)]
pub struct CompletedTask {
    pub id: String,
    pub task_type: String,
    pub description: String,
    pub started_at: String,
    pub completed_at: String,
    pub cancelled: bool,
    pub stage: String,
    pub indexed_count: u64,
    pub vectorized_count: u64,
    pub total_files: u64,
}

/// 全局任务注册表
static TASK_REGISTRY: Lazy<Mutex<HashMap<String, TaskInfo>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// 已完成任务历史（最多保留15个）
static COMPLETED_TASKS: Lazy<Mutex<VecDeque<CompletedTask>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

const MAX_COMPLETED_TASKS: usize = 15;

/// 任务队列：同类型任务排队等待
static TASK_QUEUE: Lazy<Mutex<VecDeque<String>>> = Lazy::new(|| Mutex::new(VecDeque::new()));

/// 队列通知：有任务完成时通知排队的任务
static QUEUE_NOTIFY: Lazy<Notify> = Lazy::new(|| Notify::new());

/// 注册一个新任务，返回 (task_id, cancel_flag, progress handle)
pub async fn register_task(task_type: &str, description: &str) -> (String, CancelFlag, ProgressHandle) {
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let now = chrono::Utc::now().to_rfc3339();
    
    let stage = Arc::new(Mutex::new("scanning".to_string()));
    let scan_count = Arc::new(AtomicU64::new(0));
    let total_files = Arc::new(AtomicU64::new(0));
    let indexed_count = Arc::new(AtomicU64::new(0));
    let vectorized_count = Arc::new(AtomicU64::new(0));
    
    let handle = ProgressHandle {
        stage: stage.clone(),
        scan_count: scan_count.clone(),
        total_files: total_files.clone(),
        indexed_count: indexed_count.clone(),
        vectorized_count: vectorized_count.clone(),
    };
    
    let info = TaskInfo {
        id: id.clone(),
        task_type: task_type.to_string(),
        description: description.to_string(),
        started_at: now,
        cancel_flag: cancel_flag.clone(),
        queued: false,
        stage,
        scan_count,
        total_files,
        indexed_count,
        vectorized_count,
    };
    
    let mut registry = TASK_REGISTRY.lock().await;
    registry.insert(id.clone(), info);
    
    tracing::info!("Task registered: {} [{}] {}", id, task_type, description);
    (id, cancel_flag, handle)
}

/// 注册一个排队任务（加入队列，等待轮到自己）
/// 返回 (task_id, cancel_flag, progress handle)
pub async fn register_queued_task(task_type: &str, description: &str) -> (String, CancelFlag, ProgressHandle) {
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let now = chrono::Utc::now().to_rfc3339();
    
    let stage = Arc::new(Mutex::new("queued".to_string()));
    let scan_count = Arc::new(AtomicU64::new(0));
    let total_files = Arc::new(AtomicU64::new(0));
    let indexed_count = Arc::new(AtomicU64::new(0));
    let vectorized_count = Arc::new(AtomicU64::new(0));
    
    let handle = ProgressHandle {
        stage: stage.clone(),
        scan_count: scan_count.clone(),
        total_files: total_files.clone(),
        indexed_count: indexed_count.clone(),
        vectorized_count: vectorized_count.clone(),
    };
    
    let info = TaskInfo {
        id: id.clone(),
        task_type: task_type.to_string(),
        description: description.to_string(),
        started_at: now,
        cancel_flag: cancel_flag.clone(),
        queued: true,
        stage,
        scan_count,
        total_files,
        indexed_count,
        vectorized_count,
    };
    
    // Fix lock ordering: always acquire TASK_REGISTRY first, then TASK_QUEUE.
    // Drop registry before acquiring queue to minimize contention.
    {
        let mut registry = TASK_REGISTRY.lock().await;
        registry.insert(id.clone(), info);
    } // registry dropped here
    
    // 加入队列
    let mut queue = TASK_QUEUE.lock().await;
    queue.push_back(id.clone());
    
    tracing::info!("Task queued: {} [{}] {} (position: {})", id, task_type, description, queue.len());
    (id, cancel_flag, handle)
}

/// 等待轮到自己执行（阻塞直到没有同类型任务在运行，或被取消）
/// 返回 true = 可以执行，false = 被取消
pub async fn wait_for_turn(task_id: &str) -> bool {
    loop {
        // 检查是否被取消
        {
            let registry = TASK_REGISTRY.lock().await;
            if let Some(task) = registry.get(task_id) {
                if task.cancel_flag.load(Ordering::Relaxed) {
                    drop(registry);
                    // 从队列中移除
                    let mut queue = TASK_QUEUE.lock().await;
                    queue.retain(|id| id != task_id);
                    drop(queue);
                    // 注销任务
                    let mut registry = TASK_REGISTRY.lock().await;
                    registry.remove(task_id);
                    return false;
                }
            }
        }
        
        // Check if we're at the front of the queue.
        // Pop first, then check registry WITHOUT holding queue lock — prevents AB-BA deadlock.
        let should_check = {
            let mut queue = TASK_QUEUE.lock().await;
            match queue.front() {
                Some(front) if front == task_id => {
                    queue.pop_front();
                    true
                }
                _ => false,
            }
        };
        
        if should_check {
            // Look up task type from registry (no queue lock held)
            let my_type = {
                let registry = TASK_REGISTRY.lock().await;
                registry.get(task_id).map(|t| t.task_type.clone())
            };
            // Check if there's a running task of the same type (not queued, not cancelled)
            let has_running = {
                let registry = TASK_REGISTRY.lock().await;
                let my_type_str = my_type.as_deref().unwrap_or("");
                registry.values().any(|t| {
                    let type_match = if my_type_str == "vectorizing" || my_type_str.starts_with("vectorizing") {
                        t.task_type.starts_with("vectorizing")
                    } else {
                        t.task_type == my_type_str
                    };
                    type_match
                        && t.id != task_id   // Exclude self
                        && !t.queued
                        && !t.cancel_flag.load(Ordering::Relaxed)
                })
            };
            
            if !has_running {
                // 标记为非排队状态
                let mut registry = TASK_REGISTRY.lock().await;
                if let Some(task) = registry.get_mut(task_id) {
                    task.queued = false;
                }
                
                tracing::info!("Task {} now executing", task_id);
                return true;
            } else {
                // Another task of the same type is running — put ourselves back at the front
                let mut queue = TASK_QUEUE.lock().await;
                queue.push_front(task_id.to_string());
            }
        }
        
        // 等待通知（有任务完成）
        QUEUE_NOTIFY.notified().await;
    }
}

/// 注销任务，并通知队列中的下一个任务
pub async fn unregister_task(task_id: &str) {
    let mut registry = TASK_REGISTRY.lock().await;
    if let Some(task) = registry.remove(task_id) {
        // 保存到已完成任务历史
        let stage = task.stage.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "done".to_string());
        let completed = CompletedTask {
            id: task.id.clone(),
            task_type: task.task_type.clone(),
            description: task.description.clone(),
            started_at: task.started_at.clone(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            cancelled: task.cancel_flag.load(Ordering::Relaxed),
            stage,
            indexed_count: task.indexed_count.load(Ordering::Relaxed),
            vectorized_count: task.vectorized_count.load(Ordering::Relaxed),
            total_files: task.total_files.load(Ordering::Relaxed),
        };
        drop(registry);
        
        let mut history = COMPLETED_TASKS.lock().await;
        if history.len() >= MAX_COMPLETED_TASKS {
            history.pop_front();
        }
        history.push_back(completed);
        drop(history);
        
        tracing::info!("Task unregistered: {}", task_id);
        // 通知队列中的任务
        QUEUE_NOTIFY.notify_waiters();
    }
}

/// 进度句柄 — 任务内部用来更新进度
#[allow(dead_code)]
pub struct ProgressHandle {
    stage: Arc<Mutex<String>>,
    scan_count: Arc<AtomicU64>,
    total_files: Arc<AtomicU64>,
    indexed_count: Arc<AtomicU64>,
    vectorized_count: Arc<AtomicU64>,
}

#[allow(dead_code)]
impl ProgressHandle {
    pub async fn set_stage(&self, stage: &str) {
        let mut s = self.stage.lock().await;
        *s = stage.to_string();
    }
    
    pub fn inc_scan_count(&self) {
        self.scan_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn set_total_files(&self, count: u64) {
        self.total_files.store(count, Ordering::Relaxed);
    }
    
    pub fn set_indexed_count(&self, count: u64) {
        self.indexed_count.store(count, Ordering::Relaxed);
    }
    
    pub fn inc_indexed_count(&self) {
        self.indexed_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_vectorized_count(&self) {
        self.vectorized_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn get_vectorized_count(&self) -> u64 {
        self.vectorized_count.load(Ordering::Relaxed)
    }
}

/// 获取所有运行中的任务（包括排队的）+ 已完成的历史任务
pub async fn list_tasks() -> Value {
    let registry = TASK_REGISTRY.lock().await;
    let running_tasks: Vec<Value> = registry.values().map(|t| {
        let stage = t.stage.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "unknown".to_string());
        json!({
            "id": t.id,
            "type": t.task_type,
            "description": t.description,
            "started_at": t.started_at,
            "cancelled": t.cancel_flag.load(Ordering::Relaxed),
            "queued": t.queued,
            "stage": stage,
            "scan_count": t.scan_count.load(Ordering::Relaxed),
            "total_files": t.total_files.load(Ordering::Relaxed),
            "indexed_count": t.indexed_count.load(Ordering::Relaxed),
            "vectorized_count": t.vectorized_count.load(Ordering::Relaxed),
        })
    }).collect();
    drop(registry);
    
    let history = COMPLETED_TASKS.lock().await;
    let mut completed_tasks: Vec<Value> = history.iter().map(|t| {
        json!({
            "id": t.id,
            "type": t.task_type,
            "description": t.description,
            "started_at": t.started_at,
            "completed_at": t.completed_at,
            "cancelled": t.cancelled,
            "queued": false,
            "stage": if t.cancelled { "cancelled" } else { "done" },
            "scan_count": 0,
            "total_files": t.total_files,
            "indexed_count": t.indexed_count,
            "vectorized_count": t.vectorized_count,
        })
    }).collect();
    // 按 completed_at 倒序排列（最近完成的在前）
    completed_tasks.sort_by(|a, b| {
        let a_time = a.get("completed_at").and_then(|v| v.as_str()).unwrap_or("");
        let b_time = b.get("completed_at").and_then(|v| v.as_str()).unwrap_or("");
        b_time.cmp(a_time)
    });
    drop(history);
    
    let registry = TASK_REGISTRY.lock().await;
    let indexing = registry.values().filter(|t| t.task_type == "indexing" && !t.queued).count();
    let queued = registry.values().filter(|t| t.queued).count();
    let vectorizing = registry.values().filter(|t| t.task_type.starts_with("vectorizing")).count();
    
    json!({
        "total": registry.len(),
        "indexing": indexing,
        "queued": queued,
        "vectorizing": vectorizing,
        "tasks": running_tasks,
        "completed": completed_tasks,
    })
}

/// 停止指定任务
pub async fn stop_task(task_id: &str) -> Result<String, String> {
    let registry = TASK_REGISTRY.lock().await;
    match registry.get(task_id) {
        Some(task) => {
            task.cancel_flag.store(true, Ordering::Relaxed);
            // 如果是排队中的任务，直接移除
            if task.queued {
                drop(registry);
                let mut queue = TASK_QUEUE.lock().await;
                queue.retain(|id| id != task_id);
                let mut registry = TASK_REGISTRY.lock().await;
                registry.remove(task_id);
                QUEUE_NOTIFY.notify_waiters();
                return Ok(format!("Task {} removed from queue", task_id));
            }
            Ok(format!("Task {} [{}] marked for cancellation", task_id, task.description))
        }
        None => Err(format!("Task {} not found", task_id))
    }
}

/// 强制停止并移除指定任务（用于卡死的任务）
pub async fn force_stop_task(task_id: &str) -> Result<String, String> {
    let mut registry = TASK_REGISTRY.lock().await;
    match registry.remove(task_id) {
        Some(task) => {
            task.cancel_flag.store(true, Ordering::Relaxed);
            // 保存到已完成任务历史
            let stage = task.stage.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "force_stopped".to_string());
            let completed = CompletedTask {
                id: task.id.clone(),
                task_type: task.task_type.clone(),
                description: task.description.clone(),
                started_at: task.started_at.clone(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                cancelled: true,
                stage,
                indexed_count: task.indexed_count.load(Ordering::Relaxed),
                vectorized_count: task.vectorized_count.load(Ordering::Relaxed),
                total_files: task.total_files.load(Ordering::Relaxed),
            };
            drop(registry);
            
            let mut history = COMPLETED_TASKS.lock().await;
            if history.len() >= MAX_COMPLETED_TASKS {
                history.pop_front();
            }
            history.push_back(completed);
            drop(history);
            
            // 从队列中移除
            let mut queue = TASK_QUEUE.lock().await;
            queue.retain(|id| id != task_id);
            QUEUE_NOTIFY.notify_waiters();
            tracing::warn!("Task {} [{}] force removed from registry", task_id, task.description);
            Ok(format!("Task {} [{}] force stopped", task_id, task.description))
        }
        None => Err(format!("Task {} not found", task_id))
    }
}

/// 停止所有指定类型的任务
pub async fn stop_all_tasks(task_type: Option<&str>) -> usize {
    let registry = TASK_REGISTRY.lock().await;
    let mut count = 0;
    for task in registry.values() {
        if task_type.map_or(true, |t| {
            if t == "vectorizing" { task.task_type.starts_with("vectorizing") }
            else { task.task_type == t }
        }) {
            task.cancel_flag.store(true, Ordering::Relaxed);
            count += 1;
        }
    }
    count
}

/// 检查指定类型的任务是否已在运行（不包括排队的）
#[allow(dead_code)]
pub async fn has_running_task(task_type: &str) -> bool {
    let registry = TASK_REGISTRY.lock().await;
    registry.values().any(|t| {
        let type_match = if task_type == "vectorizing" { t.task_type.starts_with("vectorizing") }
        else { t.task_type == task_type };
        type_match && !t.queued && !t.cancel_flag.load(Ordering::Relaxed)
    })
}

/// 检查指定类型的任务是否已在运行或排队中
pub async fn has_running_or_queued_task(task_type: &str) -> bool {
    let registry = TASK_REGISTRY.lock().await;
    registry.values().any(|t| {
        let type_match = if task_type == "vectorizing" { t.task_type.starts_with("vectorizing") }
        else { t.task_type == task_type };
        type_match && !t.cancel_flag.load(Ordering::Relaxed)
    })
}

/// 检查任务是否被取消
pub fn is_cancelled(cancel_flag: &CancelFlag) -> bool {
    cancel_flag.load(Ordering::Relaxed)
}

/// 获取当前活跃任务的进度摘要（给 progress API 用）
pub async fn get_active_progress() -> Option<(String, u64, u64, u64, u64)> {
    let registry = TASK_REGISTRY.lock().await;
    // 找第一个非排队的 indexing 任务
    for task in registry.values() {
        if task.task_type == "indexing" && !task.queued && !task.cancel_flag.load(Ordering::Relaxed) {
            let stage = task.stage.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "unknown".to_string());
            let scan = task.scan_count.load(Ordering::Relaxed);
            let total = task.total_files.load(Ordering::Relaxed);
            let indexed = task.indexed_count.load(Ordering::Relaxed);
            let vectorized = task.vectorized_count.load(Ordering::Relaxed);
            return Some((stage, scan, total, indexed, vectorized));
        }
    }
    // 没有 indexing 任务，找 knowledge 的 vectorizing 任务（排除 memory/session）
    for task in registry.values() {
        if (task.task_type == "vectorizing" || task.task_type == "vectorizing_knowledge") && !task.queued && !task.cancel_flag.load(Ordering::Relaxed) {
            let desc = &task.description;
            // 只返回 knowledge 向量化，memory/session 向量化不返回
            if task.task_type == "vectorizing_knowledge" || desc.contains("knowledge") || (!desc.contains("memory") && !desc.contains("session")) {
                let stage = task.stage.try_lock().map(|s| s.clone()).unwrap_or_else(|_| "unknown".to_string());
                let scan = task.scan_count.load(Ordering::Relaxed);
                let total = task.total_files.load(Ordering::Relaxed);
                let indexed = task.indexed_count.load(Ordering::Relaxed);
                let vectorized = task.vectorized_count.load(Ordering::Relaxed);
                return Some((stage, scan, total, indexed, vectorized));
            }
        }
    }
    None
}

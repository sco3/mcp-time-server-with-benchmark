use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub _id: u64,
    pub _payload: Value,
    pub sent_at: Instant,
    pub step_name: String,
}

pub struct TaskTracker {
    tasks: Mutex<HashMap<u64, TaskInfo>>,
    timeout: Duration,
}

impl TaskTracker {
    pub fn new(timeout_seconds: u64) -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            timeout: Duration::from_secs(timeout_seconds),
        }
    }
    
    pub async fn add_task(&self, id: u64, payload: Value, step_name: String) {
        let sent_at = Instant::now();
        let task_info = TaskInfo {
            _id: id,
            _payload: payload.clone(),
            sent_at,
            step_name: step_name.clone(),
        };
        
        let mut tasks = self.tasks.lock().await;
        tasks.insert(id, task_info);
        
        tracing::info!("TRACKED: Task {} added for step '{}' with payload: {}", id, step_name, payload);
    }
    
    pub async fn complete_task(&self, id: u64) -> Option<(Duration, String)> {
        let completed_at = Instant::now();
        let mut tasks = self.tasks.lock().await;
        
        if let Some(task_info) = tasks.remove(&id) {
            let duration = completed_at.duration_since(task_info.sent_at);
            
            tracing::info!("COMPLETED: Task {} for step '{}' completed in {:?}", id, task_info.step_name, duration);
            
            Some((duration, task_info.step_name))
        } else {
            tracing::warn!("COMPLETED: Task {} not found in tracker (possibly already completed or timed out)", id);
            None
        }
    }
    
    pub async fn cleanup_expired_tasks(&self) -> Vec<(u64, String)> {
        let mut tasks = self.tasks.lock().await;
        let now = Instant::now();
        
        let expired: Vec<_> = tasks
            .iter()
            .filter(|(_, task)| now.duration_since(task.sent_at) > self.timeout)
            .map(|(id, task)| (*id, task.step_name.clone()))
            .collect();
        
        for (id, _) in &expired {
            tasks.remove(id);
        }
        
        expired
    }
    
    pub async fn pending_count(&self) -> usize {
        let tasks = self.tasks.lock().await;
        tasks.len()
    }
}

// Made with Bob

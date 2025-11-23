use crate::routes::SensorData;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_HISTORY: usize = 1000;

/// 初始化app的全局状态变量
#[derive(Clone)]
pub struct AppState {
    history: Arc<RwLock<VecDeque<SensorData>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY))),
        }
    }

    pub async fn add_data(&self, data: SensorData) {
        let mut history = self.history.write().await;
        if history.len() >= MAX_HISTORY {
            history.pop_front();
        }
        history.push_back(data);
    }

    pub async fn get_history(&self) -> Vec<SensorData> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }

    pub async fn get_recent(&self, count: usize) -> Vec<SensorData> {
        let history = self.history.read().await;
        let start_idx = history.len().saturating_sub(count);
        history.range(start_idx..).cloned().collect()
    }
}

use crate::routes::SensorData;
use crate::service::{TaskManager, get_global_task_manager};
use infra::{OAuthConfig, Setting};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

const MAX_HISTORY: usize = 1000;

/// 初始化app的全局状态变量
#[derive(Clone)]
pub struct AppState {
    history: Arc<RwLock<VecDeque<SensorData>>>,
    // 定义 OAuth2 授权的相关配置
    pub oauth_config: OAuthConfig,
    pub oauth_client: BasicClient,
    // 数据库连接
    pub db_pool: PgPool,
    // 任务管理器
    pub task_manager: Arc<TaskManager>,
    // 全局配置文件配置
    pub configuration: Setting,
}

impl AppState {
    /// 初始化 AppState
    pub async fn create_app_state(
        db_pool: PgPool,
        configuration: Setting,
        oauth_config: OAuthConfig,
        oauth_client: BasicClient,
    ) -> anyhow::Result<Self> {
        // 从全局单例获取 TaskManager
        let task_manager =
            get_global_task_manager().ok_or_else(|| anyhow::anyhow!("TaskManager 尚未初始化"))?;

        Ok(Self {
            history: Arc::new(Default::default()),
            oauth_config,
            oauth_client,
            db_pool,
            task_manager,
            configuration,
        })
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

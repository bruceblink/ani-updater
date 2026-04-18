use crate::task_manage::{TaskManager, get_global_task_manager};
use infra::{OAuthConfig, Setting};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use std::sync::Arc;

/// 初始化app的全局状态变量
#[derive(Clone)]
pub struct AppState {
    /// GitHub OAuth 配置（可选，仅在配置了 GITHUB_CLIENT_ID 等环境变量时有值）
    pub oauth_config: Option<OAuthConfig>,
    pub oauth_client: Option<BasicClient>,
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
        oauth_config: Option<OAuthConfig>,
        oauth_client: Option<BasicClient>,
    ) -> anyhow::Result<Self> {
        // 从全局单例获取 TaskManager
        let task_manager =
            get_global_task_manager().ok_or_else(|| anyhow::anyhow!("TaskManager 尚未初始化"))?;

        Ok(Self {
            oauth_config,
            oauth_client,
            db_pool,
            task_manager,
            configuration,
        })
    }
}

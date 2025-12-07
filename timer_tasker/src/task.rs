use async_trait::async_trait;
use common::api::ApiResponse;
use common::po::ItemResult;
use cron::Schedule;
use serde::Deserialize;
use service::timer_task_command::{CmdFn, CommandInput};
use sqlx::PgPool;
use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;

/// -----------------
/// 配置层 TaskMeta
/// -----------------
#[derive(Clone, Debug, Deserialize)]
pub struct TaskMeta {
    pub name: String,
    pub cmd: String,
    pub arg: String,
    pub cron_expr: String,
    pub retry_times: u8,
}

/// -----------------
/// 异步任务 trait
/// -----------------
#[async_trait]
pub trait TaskAction: Send + Sync {
    async fn run(&self) -> Result<ApiResponse<ItemResult>, String>;
}

#[async_trait]
impl<F, Fut> TaskAction for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ApiResponse<ItemResult>, String>> + Send,
{
    async fn run(&self) -> Result<ApiResponse<ItemResult>, String> {
        self().await
    }
}

/// -----------------
/// 运行时 Task
/// -----------------
#[derive(Clone)]
pub struct Task {
    pub name: String,
    pub cron_expr: String,
    pub action: Arc<dyn TaskAction>,
    pub retry_times: u8,
}

impl Task {
    pub fn new<F, Fut>(meta: &TaskMeta, action: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ApiResponse<ItemResult>, String>> + Send + 'static,
    {
        Self {
            name: meta.name.clone(),
            cron_expr: meta.cron_expr.clone(),
            action: Arc::new(action),
            retry_times: meta.retry_times,
        }
    }

    pub fn schedule(&self) -> Schedule {
        Schedule::from_str(&self.cron_expr).expect("Invalid cron expression")
    }
}

/// 将 TaskMeta 列表和命令表合并，生成运行时 Task 列表
pub fn build_tasks_from_meta(
    metas: &Vec<TaskMeta>,
    cmd_map: &HashMap<String, CmdFn>,
    db_pool: Arc<PgPool>,
) -> Vec<Task> {
    let mut tasks = Vec::new();

    for meta in metas {
        // 提前克隆需要的字段，避免闭包借用局部变量
        let name = meta.name.clone();
        let cmd = meta.cmd.clone();
        let arg = meta.arg.clone();
        let cron_expr = meta.cron_expr.clone();
        let retry_times = meta.retry_times;

        if let Some(cmd_fn) = cmd_map.get(&cmd) {
            // 找到命令：把 cmd_fn 和 arg 克隆到闭包里
            let cmd_fn = cmd_fn.clone();
            let arg_for_closure = arg.clone();
            let db_pool = db_pool.clone();
            // 构造 Task
            let task = Task::new(
                &TaskMeta {
                    name: name.clone(),
                    cmd: cmd.clone(),
                    arg: arg.clone(),
                    cron_expr: cron_expr.clone(),
                    retry_times,
                },
                move || {
                    let cmd_fn = cmd_fn.clone();
                    let args = arg_for_closure.clone();
                    let name_for_log = name.clone(); // 如果闭包里要用 name 做日志
                    let db_pool = db_pool.clone();
                    async move {
                        // 调用命令函数
                        let input = CommandInput {
                            args,
                            db: Option::from(db_pool), // 后续可传 Arc<PgPool>
                        };
                        cmd_fn(input)
                            .await
                            .map_err(|e| format!("Task '{}' failed: {}", name_for_log, e))
                    }
                },
            );

            tasks.push(task);
        } else {
            // cmd 未找到，返回 Err
            let missing_cmd = cmd.clone();
            let name_for_log = name.clone();
            let task = Task::new(
                &TaskMeta {
                    name: name.clone(),
                    cmd: cmd.clone(),
                    arg: arg.clone(),
                    cron_expr: cron_expr.clone(),
                    retry_times,
                },
                move || {
                    let missing_cmd = missing_cmd.clone();
                    let name_for_log = name_for_log.clone();
                    async move {
                        Err(format!(
                            "cmd '{}' not found for task '{}'",
                            missing_cmd, name_for_log
                        ))
                    }
                },
            );

            tasks.push(task);
        }
    }

    tasks
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    pub name: String,
    pub result: Option<ItemResult>,
}

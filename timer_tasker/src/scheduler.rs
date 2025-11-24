use crate::task::{Task, TaskResult};
use chrono::Local;
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

#[derive(Clone)]
pub struct Scheduler {
    pub tasks: Vec<Arc<Task>>,
    shutdown: Arc<Notify>,
    task_handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    semaphore: Arc<Semaphore>, // 控制并发的信号量
}

impl Scheduler {
    /// 构建定时任务执行器 <br>
    /// tasks 定时任务的任务Vec <br>
    /// max_concurrent_tasks 默认并发为系统的CPU核心数
    pub fn new(tasks: Vec<Task>, max_concurrent_tasks: Option<usize>) -> Self {
        // 获取系统的 CPU 核心数
        let default_max_concurrent_tasks = num_cpus::get();

        // 使用传入的值，或者默认值
        let max_concurrent_tasks = max_concurrent_tasks.unwrap_or(default_max_concurrent_tasks);

        Self {
            tasks: tasks.into_iter().map(Arc::new).collect(),
            shutdown: Arc::new(Notify::new()),
            task_handles: Arc::new(Mutex::new(vec![])),
            semaphore: Arc::new(Semaphore::new(max_concurrent_tasks)), // 限制并发任务数
        }
    }

    /// 运行调度器，所有任务各自独立运行，互不干扰
    pub async fn run(&self, sender: mpsc::Sender<TaskResult>) {
        for task in &self.tasks {
            let t = task.clone();
            let s = sender.clone();
            let shutdown = self.shutdown.clone();
            let semaphore = self.semaphore.clone();

            let handle = tokio::spawn(async move {
                Self::run_single_task(t, s, semaphore, shutdown).await;
            });
            self.task_handles.lock().unwrap().push(handle);
        }
    }

    /// 运行单个任务的独立调度循环
    async fn run_single_task(
        task: Arc<Task>,
        sender: mpsc::Sender<TaskResult>,
        semaphore: Arc<Semaphore>,
        shutdown: Arc<Notify>,
    ) {
        let schedule = task.schedule();

        loop {
            let mut upcoming = schedule.upcoming(Local);
            let Some(next_time) = upcoming.next() else {
                warn!("任务 [{}] 无下次运行时间，结束调度", task.name);
                break;
            };

            let duration = (next_time - Local::now())
                .to_std()
                .unwrap_or(Duration::from_secs(0)); // 计算任务等待时间

            tokio::select! {
                _ = sleep(duration) => {
                    // 获取信号量许可
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    // 执行任务
                    let t = task.clone();
                    let s = sender.clone();
                    tokio::spawn(async move {
                        let _permit = permit; // 确保在任务执行期间保持许可有效
                        Self::execute_task(t, s).await;
                    });
                }
                _ = shutdown.notified() => {
                    warn!("任务 [{}] 收到停止信号，退出调度", task.name);
                    break;
                }
            }
        }
    }

    /// 实际执行任务逻辑 + 重试
    async fn execute_task(task: Arc<Task>, sender: mpsc::Sender<TaskResult>) {
        for attempt in 0..=task.retry_times {
            match task.action.run().await {
                Ok(resp) => {
                    info!("任务 [{}] 执行成功", task.name);
                    let result = TaskResult {
                        name: task.name.clone(),
                        result: Some(resp.data.unwrap_or_default()),
                    };
                    let _ = sender.send(result).await;
                    break;
                }
                Err(e) => {
                    warn!(
                        "任务 [{}] 执行失败: {}, 重试 {}/{}",
                        task.name,
                        e,
                        attempt + 1,
                        task.retry_times
                    );
                    if attempt < task.retry_times {
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }

    /// 外部调用，用于停止所有任务
    pub fn stop(&self) {
        self.shutdown.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskAction, TaskResult};
    use common::api::NewsItem;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;

    /// 模拟的任务动作
    struct MockAction {
        id: String,
        counter: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl TaskAction for MockAction {
        async fn run(
            &self,
        ) -> Result<
            common::api::ApiResponse<std::collections::HashMap<String, Vec<common::api::TaskItem>>>,
            String,
        > {
            use chrono::Local;
            use common::api::{ApiResponse, TaskItem};
            use std::collections::HashMap;

            let count = self.counter.fetch_add(1, Ordering::SeqCst) + 1;

            println!(
                "[{}] {}: 执行第 {} 次",
                Local::now().format("%H:%M:%S"),
                self.id,
                count
            );

            // 构造一个假的结果数据
            let mut map = HashMap::new();
            map.insert(
                "mock".into(),
                vec![TaskItem::News(NewsItem {
                    id: "baidu".to_string(),
                    name: "百度".to_string(),
                    items: vec![],
                })],
            );

            Ok(ApiResponse {
                status: "".to_string(),
                data: Some(map),
                message: None,
            })
        }
    }

    #[tokio::test]
    async fn test_multiple_cron_expressions_run_independently() {
        // 任务 A: 在 6-23 小时每 20 分钟执行一次（模拟 cron_expr2）
        let counter_a = Arc::new(AtomicUsize::new(0));
        let task_a = Task {
            name: "任务A".into(),
            cron_expr: "*/5 * * * * * *".into(), // 为测试改成每5秒执行一次
            retry_times: 0,
            action: Arc::new(MockAction {
                id: "A".into(),
                counter: counter_a.clone(),
            }),
        };

        // 任务 B: 指定某些小时点触发（模拟 cron_expr1）
        let counter_b = Arc::new(AtomicUsize::new(0));
        let task_b = Task {
            name: "任务B".into(),
            cron_expr: "*/7 * * * * * *".into(), // 每7秒执行一次
            retry_times: 0,
            action: Arc::new(MockAction {
                id: "B".into(),
                counter: counter_b.clone(),
            }),
        };

        let scheduler = Scheduler::new(vec![task_a, task_b], Some(2));
        let (tx, mut rx) = mpsc::channel::<TaskResult>(100);

        tokio::spawn(async move {
            scheduler.run(tx).await;
        });

        // 收集部分结果
        let mut received = vec![];
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                println!("收到结果: {:?}", res.name);
                received.push(res);
            }
        });

        // 观察 15 秒
        sleep(Duration::from_secs(15)).await;

        println!(
            "任务A执行次数: {}, 任务B执行次数: {}",
            counter_a.load(Ordering::SeqCst),
            counter_b.load(Ordering::SeqCst)
        );

        assert!(counter_a.load(Ordering::SeqCst) >= 2);
        assert!(counter_b.load(Ordering::SeqCst) >= 2);
    }
}

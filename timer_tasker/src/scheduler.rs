use crate::task::{Task, TaskResult};
use chrono::{Local, Utc};
use std::sync::Arc;
use tokio::sync::{Notify, Semaphore, mpsc};
use tokio::time::{Duration, sleep};
use tracing::{info, warn};

#[derive(Clone)]
pub struct Scheduler {
    pub tasks: Vec<Arc<Task>>,
    shutdown: Arc<Notify>,
    semaphore: Arc<Semaphore>,
}

impl Scheduler {
    pub fn new(tasks: Vec<Task>, max_concurrent_tasks: Option<usize>) -> Self {
        let default_max_concurrent_tasks = num_cpus::get();
        let max_concurrent_tasks = max_concurrent_tasks.unwrap_or(default_max_concurrent_tasks);

        Self {
            tasks: tasks.into_iter().map(Arc::new).collect(),
            shutdown: Arc::new(Notify::new()),
            semaphore: Arc::new(Semaphore::new(max_concurrent_tasks)),
        }
    }

    pub async fn run(&self, sender: mpsc::Sender<TaskResult>) {
        for task in &self.tasks {
            let t = task.clone();
            let s = sender.clone();
            let shutdown = self.shutdown.clone();
            let semaphore = self.semaphore.clone();

            tokio::spawn(async move {
                Self::run_single_task(t, s, semaphore, shutdown).await;
            });
        }
    }

    async fn run_single_task(
        task: Arc<Task>,
        sender: mpsc::Sender<TaskResult>,
        semaphore: Arc<Semaphore>,
        shutdown: Arc<Notify>,
    ) {
        let schedule = match task.schedule() {
            Ok(s) => s,
            Err(e) => {
                warn!("{e}，跳过调度");
                return;
            }
        };

        loop {
            let mut upcoming = schedule.upcoming(Local);
            let Some(next_time) = upcoming.next() else {
                warn!("任务 [{}] 无下次运行时间，结束调度", task.name);
                break;
            };

            let duration = (next_time - Local::now())
                .to_std()
                .unwrap_or(Duration::from_secs(0));

            tokio::select! {
                _ = sleep(duration) => {
                    match semaphore.clone().acquire_owned().await {
                        Ok(permit) => {
                            let t = task.clone();
                            let s = sender.clone();
                            tokio::spawn(async move {
                                let _permit = permit;
                                Self::execute_task(t, s).await;
                            });
                        }
                        Err(e) => {
                            warn!("任务 [{}] 信号量已关闭: {e}，退出调度", task.name);
                            break;
                        }
                    }
                }
                _ = shutdown.notified() => {
                    warn!("任务 [{}] 收到停止信号，退出调度", task.name);
                    break;
                }
            }
        }
    }

    async fn execute_task(task: Arc<Task>, sender: mpsc::Sender<TaskResult>) {
        let schedule = match task.schedule() {
            Ok(s) => s,
            Err(e) => {
                warn!("{}", e);
                return;
            }
        };

        let last_run = Utc::now();
        let next_run = schedule.upcoming(Utc).next();

        for attempt in 0..=task.retry_times {
            match task.action.run().await {
                Ok(resp) => {
                    info!("任务 [{}] 执行成功", task.name);
                    let result = TaskResult {
                        name: task.name.clone(),
                        result: Some(resp.data.unwrap_or_default()),
                        last_run,
                        next_run,
                        last_status: "success".to_string(),
                    };
                    let _ = sender.send(result).await;
                    return;
                }
                Err(e) => {
                    let current_try = attempt + 1;
                    if attempt < task.retry_times {
                        warn!(
                            "任务 [{}] 执行失败: {}, 将重试 {}/{}",
                            task.name,
                            e,
                            current_try,
                            task.retry_times + 1
                        );
                        sleep(Duration::from_secs(5)).await;
                    } else {
                        warn!(
                            "任务 [{}] 执行失败: {}, 已达到最大重试次数 {}/{}",
                            task.name,
                            e,
                            current_try,
                            task.retry_times + 1
                        );
                    }
                }
            }
        }

        let result = TaskResult {
            name: task.name.clone(),
            result: None,
            last_run,
            next_run,
            last_status: "failed".to_string(),
        };
        let _ = sender.send(result).await;
    }

    pub fn stop(&self) {
        self.shutdown.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskAction, TaskResult};
    use common::po::{NewsInfo, TaskItem};
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::mpsc;

    struct MockAction {
        id: String,
        counter: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl TaskAction for MockAction {
        async fn run(
            &self,
        ) -> Result<
            common::api::ApiResponse<std::collections::HashMap<String, HashSet<TaskItem>>>,
            String,
        > {
            use chrono::Local;
            use common::api::ApiResponse;
            use std::collections::HashMap;

            let count = self.counter.fetch_add(1, Ordering::SeqCst) + 1;

            println!(
                "[{}] {}: 执行第 {} 次",
                Local::now().format("%H:%M:%S"),
                self.id,
                count
            );

            let mut map = HashMap::new();
            let mut set = HashSet::new();
            set.insert(TaskItem::News(NewsInfo {
                id: "baidu".to_string(),
                name: "百度".to_string(),
                items: vec![],
            }));
            map.insert("mock".into(), set);

            Ok(ApiResponse {
                status: "".to_string(),
                data: Some(map),
                message: None,
            })
        }
    }

    #[tokio::test]
    async fn test_multiple_cron_expressions_run_independently() {
        let counter_a = Arc::new(AtomicUsize::new(0));
        let task_a = Task {
            name: "任务A".into(),
            cron_expr: "*/5 * * * * * *".into(),
            retry_times: 0,
            action: Arc::new(MockAction {
                id: "A".into(),
                counter: counter_a.clone(),
            }),
        };

        let counter_b = Arc::new(AtomicUsize::new(0));
        let task_b = Task {
            name: "任务B".into(),
            cron_expr: "*/7 * * * * * *".into(),
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

        let mut received = vec![];
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                println!("收到结果: {:?}", res.name);
                received.push(res);
            }
        });

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

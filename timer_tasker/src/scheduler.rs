use crate::task::{Task, TaskResult};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc, watch};
use tokio::time::{Duration, Instant, sleep};
use tracing::{info, warn};

#[derive(Clone)]
pub struct Scheduler {
    pub tasks: Vec<Arc<Task>>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    semaphore: Arc<Semaphore>,
}

impl Scheduler {
    pub fn new(tasks: Vec<Task>, max_concurrent_tasks: Option<usize>) -> Self {
        let default_max_concurrent_tasks = num_cpus::get();
        let max_concurrent_tasks = max_concurrent_tasks.unwrap_or(default_max_concurrent_tasks);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            tasks: tasks.into_iter().map(Arc::new).collect(),
            shutdown_tx,
            shutdown_rx,
            semaphore: Arc::new(Semaphore::new(max_concurrent_tasks)),
        }
    }

    pub async fn run(&self, sender: mpsc::Sender<TaskResult>) {
        let mut task_handles = Vec::with_capacity(self.tasks.len());

        for task in &self.tasks {
            let t = task.clone();
            let s = sender.clone();
            let shutdown = self.shutdown_rx.clone();
            let semaphore = self.semaphore.clone();

            task_handles.push(tokio::spawn(async move {
                Self::run_single_task(t, s, semaphore, shutdown).await;
            }));
        }

        for handle in task_handles {
            if let Err(e) = handle.await {
                warn!(error = %e, "任务调度线程退出异常");
            }
        }
    }

    fn next_run_delay(schedule: &cron::Schedule, now: chrono::DateTime<Utc>) -> Option<Duration> {
        let next_time = schedule.after(&now).next()?;
        Some((next_time - now).to_std().unwrap_or(Duration::from_secs(0)))
    }

    fn backoff_with_jitter(attempt: u8) -> Duration {
        let base_secs = 1_u64;
        let max_secs = 30_u64;
        let exp = 2_u64.saturating_pow(attempt as u32);
        let capped = (base_secs.saturating_mul(exp)).min(max_secs);
        let jitter = rand::random_range(80_u64..=120_u64);
        Duration::from_millis(capped.saturating_mul(1000).saturating_mul(jitter) / 100)
    }

    async fn run_single_task(
        task: Arc<Task>,
        sender: mpsc::Sender<TaskResult>,
        semaphore: Arc<Semaphore>,
        mut shutdown: watch::Receiver<bool>,
    ) {
        let schedule = match task.schedule() {
            Ok(s) => s,
            Err(e) => {
                warn!(task_name = %task.name, error = %e, "cron 表达式无效，跳过调度");
                return;
            }
        };

        loop {
            if *shutdown.borrow() {
                warn!(task_name = %task.name, "收到停止信号，退出调度");
                break;
            }

            let Some(duration) = Self::next_run_delay(&schedule, Utc::now()) else {
                warn!(task_name = %task.name, "无下次运行时间，结束调度");
                break;
            };

            tokio::select! {
                _ = sleep(duration) => {
                    match semaphore.clone().acquire_owned().await {
                        Ok(permit) => {
                            let t = task.clone();
                            let s = sender.clone();
                            let schedule_for_exec = schedule.clone();
                            tokio::spawn(async move {
                                let _permit = permit;
                                Self::execute_task(t, s, schedule_for_exec).await;
                            });
                        }
                        Err(e) => {
                            warn!(task_name = %task.name, error = %e, "信号量已关闭，退出调度");
                            break;
                        }
                    }
                }
                change = shutdown.changed() => {
                    match change {
                        Ok(()) if *shutdown.borrow() => {
                            warn!(task_name = %task.name, "收到停止信号，退出调度");
                            break;
                        }
                        Ok(()) => {}
                        Err(e) => {
                            warn!(task_name = %task.name, error = %e, "停止信号通道关闭，退出调度");
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn execute_task(
        task: Arc<Task>,
        sender: mpsc::Sender<TaskResult>,
        schedule: cron::Schedule,
    ) {
        let started_at = Instant::now();
        let last_run = Utc::now();
        let next_run = schedule.upcoming(Utc).next();
        let max_attempts = task.retry_times + 1;

        for attempt in 0..=task.retry_times {
            let current_try = attempt + 1;
            match task.action.run().await {
                Ok(resp) => {
                    info!(
                        task_name = %task.name,
                        attempt = current_try,
                        max_attempts,
                        elapsed_ms = started_at.elapsed().as_millis() as u64,
                        "任务执行成功"
                    );
                    let result = TaskResult {
                        name: task.name.clone(),
                        result: Some(resp.data.unwrap_or_default()),
                        last_run,
                        next_run,
                        last_status: "success".to_string(),
                    };
                    if let Err(e) = sender.send(result).await {
                        warn!(task_name = %task.name, error = %e, "任务结果发送失败");
                    }
                    return;
                }
                Err(e) => {
                    if attempt < task.retry_times {
                        let backoff = Self::backoff_with_jitter(attempt);
                        warn!(
                            task_name = %task.name,
                            attempt = current_try,
                            max_attempts,
                            error = %e,
                            backoff_ms = backoff.as_millis() as u64,
                            elapsed_ms = started_at.elapsed().as_millis() as u64,
                            "任务执行失败，准备重试"
                        );
                        sleep(backoff).await;
                    } else {
                        warn!(
                            task_name = %task.name,
                            attempt = current_try,
                            max_attempts,
                            error = %e,
                            elapsed_ms = started_at.elapsed().as_millis() as u64,
                            "任务执行失败，达到最大重试次数"
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
        if let Err(e) = sender.send(result).await {
            warn!(task_name = %task.name, error = %e, "任务失败结果发送失败");
        }
    }

    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskAction, TaskResult};
    use chrono::{Duration as ChronoDuration, TimeZone, Utc};
    use common::api::ApiResponse;
    use common::po::{ItemResult, NewsInfo, TaskItem};
    use std::collections::HashSet;
    use std::str::FromStr;
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

    struct FlakyAction {
        counter: Arc<AtomicUsize>,
        succeed_on: usize,
    }

    #[async_trait::async_trait]
    impl TaskAction for FlakyAction {
        async fn run(&self) -> Result<ApiResponse<ItemResult>, String> {
            let current = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
            if current >= self.succeed_on {
                Ok(ApiResponse {
                    status: "".to_string(),
                    data: Some(Default::default()),
                    message: None,
                })
            } else {
                Err(format!("attempt {current} failed"))
            }
        }
    }

    struct AlwaysFailAction {
        counter: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl TaskAction for AlwaysFailAction {
        async fn run(&self) -> Result<ApiResponse<ItemResult>, String> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Err("always fail".to_string())
        }
    }

    #[test]
    fn test_next_run_delay_uses_utc_baseline() {
        let schedule = cron::Schedule::from_str("0 */5 * * * * *").unwrap();
        let now = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 2).unwrap();

        let delay = Scheduler::next_run_delay(&schedule, now).unwrap();

        assert_eq!(delay, ChronoDuration::seconds(298).to_std().unwrap());
    }

    #[test]
    fn test_backoff_with_jitter_has_bounds() {
        for attempt in 0..=6 {
            let delay = Scheduler::backoff_with_jitter(attempt);
            assert!(delay >= Duration::from_millis(800));
            assert!(delay <= Duration::from_millis(36_000));
        }
    }

    #[tokio::test]
    async fn test_execute_task_retries_until_success() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let task = Arc::new(Task {
            name: "flaky-success".into(),
            cron_expr: "*/5 * * * * * *".into(),
            retry_times: 2,
            action: Arc::new(FlakyAction {
                counter: attempts.clone(),
                succeed_on: 3,
            }),
        });

        let (tx, mut rx) = mpsc::channel::<TaskResult>(8);
        let schedule = task.schedule().unwrap();
        Scheduler::execute_task(task, tx, schedule).await;

        let result = rx.recv().await.unwrap();
        assert_eq!(result.last_status, "success");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_task_reports_failed_after_max_retries() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let task = Arc::new(Task {
            name: "always-fail".into(),
            cron_expr: "*/5 * * * * * *".into(),
            retry_times: 2,
            action: Arc::new(AlwaysFailAction {
                counter: attempts.clone(),
            }),
        });

        let (tx, mut rx) = mpsc::channel::<TaskResult>(8);
        let schedule = task.schedule().unwrap();
        Scheduler::execute_task(task, tx, schedule).await;

        let result = rx.recv().await.unwrap();
        assert_eq!(result.last_status, "failed");
        assert!(result.result.is_none());
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
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

        let scheduler_clone = scheduler.clone();
        let run_handle = tokio::spawn(async move {
            scheduler_clone.run(tx).await;
        });

        let mut received = vec![];
        tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                println!("收到结果: {:?}", res.name);
                received.push(res);
            }
        });

        sleep(Duration::from_secs(15)).await;
        scheduler.stop();
        let _ = run_handle.await;

        println!(
            "任务A执行次数: {}, 任务B执行次数: {}",
            counter_a.load(Ordering::SeqCst),
            counter_b.load(Ordering::SeqCst)
        );

        assert!(counter_a.load(Ordering::SeqCst) >= 2);
        assert!(counter_b.load(Ordering::SeqCst) >= 2);
    }
}

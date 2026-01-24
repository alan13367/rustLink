use crate::db::Repository;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// Background job types
#[derive(Debug)]
pub enum Job {
    /// Increment click count for a URL
    IncrementClickCount { short_code: String },
    /// Delete cache entry for a URL
    #[allow(dead_code)]
    InvalidateCache { short_code: String },
}

/// Background worker configuration
#[derive(Clone)]
pub struct WorkerConfig {
    /// Maximum retries for failed jobs
    pub max_retries: u32,
    /// Backoff duration between retries
    pub retry_delay_ms: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Background job worker
pub struct Worker {
    repository: Repository,
    receiver: mpsc::UnboundedReceiver<Job>,
    config: WorkerConfig,
}

impl Worker {
    /// Create a new worker
    pub fn new(repository: Repository, receiver: mpsc::UnboundedReceiver<Job>) -> Self {
        Self {
            repository,
            receiver,
            config: WorkerConfig::default(),
        }
    }

    /// Set worker configuration
    #[allow(dead_code)]
    pub fn with_config(mut self, config: WorkerConfig) -> Self {
        self.config = config;
        self
    }

    /// Run the worker - processes jobs until channel closes
    pub async fn run(mut self) {
        info!("Background worker started");

        while let Some(job) = self.receiver.recv().await {
            self.process_job(job).await;
        }

        info!("Background worker stopped");
    }

    /// Process a single job with retries
    async fn process_job(&self, job: Job) {
        let mut retries = 0;

        loop {
            match self.execute_job(&job).await {
                Ok(_) => {
                    // Job succeeded, move to next
                    break;
                }
                Err(e) if retries < self.config.max_retries => {
                    retries += 1;
                    let delay = std::time::Duration::from_millis(self.config.retry_delay_ms);
                    warn!(
                        "Job failed (attempt {}/{}), retrying in {:?}: {:?}",
                        retries,
                        self.config.max_retries,
                        delay,
                        job
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(_e) => {
                    // Job failed after all retries
                    error!("Job failed after {} retries: {:?}", self.config.max_retries, job);
                    break;
                }
            }
        }
    }

    /// Execute a job without retries
    async fn execute_job(&self, job: &Job) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match job {
            Job::IncrementClickCount { short_code } => {
                self.repository
                    .increment_click_count(short_code)
                    .await?;
                Ok(())
            }
            Job::InvalidateCache { short_code: _ } => {
                // Cache invalidation is handled separately by the cache layer
                // This job type is for future use or coordination
                Ok(())
            }
        }
    }
}

/// Job sender - used to submit jobs to the worker
#[derive(Clone)]
pub struct JobSender {
    sender: mpsc::UnboundedSender<Job>,
}

impl JobSender {
    /// Create a new job sender
    pub fn new(sender: mpsc::UnboundedSender<Job>) -> Self {
        Self { sender }
    }

    /// Submit a job to be processed asynchronously
    pub fn send(&self, job: Job) {
        if let Err(_) = self.sender.send(job) {
            error!("Failed to send job to worker - channel may be closed");
        }
    }

    /// Submit an increment click count job
    pub fn increment_click_count(&self, short_code: String) {
        self.send(Job::IncrementClickCount { short_code });
    }

    /// Submit an invalidate cache job
    #[allow(dead_code)]
    pub fn invalidate_cache(&self, short_code: String) {
        self.send(Job::InvalidateCache { short_code });
    }
}

/// Create a new job sender and receiver pair
pub fn create_job_channel() -> (JobSender, mpsc::UnboundedReceiver<Job>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    (JobSender::new(sender), receiver)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_sender() {
        let (sender, mut receiver) = create_job_channel();

        sender.send(Job::IncrementClickCount {
            short_code: "test".to_string(),
        });

        assert!(receiver.try_recv().is_ok());
    }
}

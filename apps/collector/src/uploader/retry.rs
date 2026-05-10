use std::time::Duration;

pub struct RetryPolicy {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
        }
    }
    
    pub fn default() -> Self {
        Self::new(3, Duration::from_secs(1), Duration::from_secs(60))
    }
    
    pub fn get_delay(&self, attempt: u32) -> Duration {
        if attempt >= self.max_retries {
            return self.max_delay;
        }
        
        let delay = self.base_delay * 2u32.pow(attempt);
        delay.min(self.max_delay)
    }
    
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

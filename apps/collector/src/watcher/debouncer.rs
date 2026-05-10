use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct Debouncer {
    delays: HashMap<String, Duration>,
    pending: HashMap<String, Instant>,
}

impl Debouncer {
    pub fn new() -> Self {
        Self {
            delays: HashMap::new(),
            pending: HashMap::new(),
        }
    }
    
    pub fn set_delay(&mut self, source_id: &str, delay: Duration) {
        self.delays.insert(source_id.to_string(), delay);
    }
    
    pub fn debounce(&mut self, key: &str, source_id: &str) -> bool {
        let delay = self.delays.get(source_id)
            .copied()
            .unwrap_or(Duration::from_secs(5));
        
        let now = Instant::now();
        
        if let Some(last) = self.pending.get(key) {
            if now.duration_since(*last) < delay {
                return false;
            }
        }
        
        self.pending.insert(key.to_string(), now);
        true
    }
    
    pub fn cleanup(&mut self, max_age: Duration) {
        let now = Instant::now();
        self.pending.retain(|_, time| now.duration_since(*time) < max_age);
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub max_message_size: usize,
    pub drain_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_message_size: 4096,
            drain_timeout_secs: 5,
            idle_timeout_secs: 60,
        }
    }
}

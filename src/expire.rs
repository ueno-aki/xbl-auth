use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpiringValue<V> {
    expired_at: u64,
    value: V,
}
impl<V> ExpiringValue<V> {
    pub fn with_duration_secs(value: V, expired_in: u64) -> Self {
        Self {
            expired_at: expired_in
                + SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            value,
        }
    }
    pub fn with_timestamp(value: V, expired_at: u64) -> Self {
        Self { expired_at, value }
    }
    pub fn is_expired(&self) -> bool {
        self.expired_at
            <= SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
    }
    pub fn get(&self) -> Result<&V> {
        if self.is_expired() {
            Err(anyhow!("This value expired yet."))
        } else {
            Ok(&self.value)
        }
    }
    pub fn get_mut(&mut self) -> Result<&mut V> {
        if self.is_expired() {
            Err(anyhow!("This value expired yet."))
        } else {
            Ok(&mut self.value)
        }
    }
}

use std::sync::atomic::AtomicUsize;

use serde::Serialize;

pub struct Metrics {
    active_connections: AtomicUsize,
    total_connections: AtomicUsize,
    bytes_received: AtomicUsize,
    bytes_sent: AtomicUsize,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            active_connections: AtomicUsize::new(0),
            total_connections: AtomicUsize::new(0),
            bytes_received: AtomicUsize::new(0),
            bytes_sent: AtomicUsize::new(0),
        }
    }

    pub fn connection_started(&self) -> (usize, usize) {
        let active = self
            .active_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let total = self
            .total_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (active + 1, total + 1)
    }

    pub fn connection_ended(&self) -> (usize, usize) {
        let active = self
            .active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        let total = self
            .total_connections
            .load(std::sync::atomic::Ordering::Relaxed);
        (active - 1, total)
    }

    pub fn record_bytes_received(&self, bytes: usize) -> usize {
        self.bytes_received
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed)
            + bytes
    }

    pub fn record_bytes_sent(&self, bytes: usize) -> usize {
        self.bytes_sent
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed)
            + bytes
    }
    pub fn active_connections(&self) -> usize {
        self.active_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    }
    pub fn total_connections(&self) -> usize {
        self.total_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn bytes_received(&self) -> usize {
        self.bytes_received
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn bytes_sent(&self) -> usize {
        self.bytes_sent.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_connections: self.active_connections(),
            total_connections: self.total_connections(),
            bytes_received: self.bytes_received(),
            bytes_sent: self.bytes_sent(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MetricsSnapshot {
    pub active_connections: usize,
    pub total_connections: usize,
    pub bytes_received: usize,
    pub bytes_sent: usize,
}

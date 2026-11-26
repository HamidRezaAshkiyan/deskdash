use crate::models::{DiskStat, SystemStats};
use chrono::Utc;
use parking_lot::Mutex;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

struct MetricsInner {
    sys: System,
    disks: Disks,
    networks: Networks,
    last_sample: Instant,
    last_rx: u64,
    last_tx: u64,
    cached: Option<SystemStats>,
}

pub struct SystemMetricsProvider {
    inner: Mutex<MetricsInner>,
}

impl SystemMetricsProvider {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu_usage();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let (rx, tx) = sum_network(&networks);
        Self {
            inner: Mutex::new(MetricsInner {
                sys,
                disks,
                networks,
                last_sample: Instant::now(),
                last_rx: rx,
                last_tx: tx,
                cached: None,
            }),
        }
    }

    pub fn sample(&self) -> SystemStats {
        let mut guard = self.inner.lock();
        let elapsed = guard.last_sample.elapsed().as_secs_f64();
        // Throttle heavy sysinfo refreshes to at most once every 400ms across multiple widgets
        if elapsed < 0.4 {
            if let Some(ref cached) = guard.cached {
                return cached.clone();
            }
        }

        guard.sys.refresh_cpu_usage();
        guard.sys.refresh_memory();
        guard.disks.refresh(true);
        guard.networks.refresh(true);

        let (rx_now, tx_now) = sum_network(&guard.networks);
        let dt = elapsed.max(0.1);
        let rx_rate = (rx_now.saturating_sub(guard.last_rx) as f64 / dt) as u64;
        let tx_rate = (tx_now.saturating_sub(guard.last_tx) as f64 / dt) as u64;
        guard.last_rx = rx_now;
        guard.last_tx = tx_now;
        guard.last_sample = Instant::now();

        let cpu_usage = guard.sys.global_cpu_usage();
        let cpu_cores: Vec<f32> = guard.sys.cpus().iter().map(|c| c.cpu_usage()).collect();
        let mem_total = guard.sys.total_memory();
        let mem_used = guard.sys.used_memory();
        let mem_pct = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64 * 100.0) as f32
        } else {
            0.0
        };

        let disks = guard
            .disks
            .iter()
            .take(4)
            .map(|d| DiskStat {
                name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                total_bytes: d.total_space(),
                used_bytes: d.total_space().saturating_sub(d.available_space()),
            })
            .collect();

        let stats = SystemStats {
            cpu_usage,
            cpu_cores,
            memory_used_bytes: mem_used,
            memory_total_bytes: mem_total,
            memory_percent: mem_pct,
            gpu_usage: None,
            net_rx_bytes_per_sec: rx_rate,
            net_tx_bytes_per_sec: tx_rate,
            disks,
            uptime_secs: System::uptime(),
            timestamp_ms: Utc::now().timestamp_millis(),
        };
        guard.cached = Some(stats.clone());
        stats
    }
}

fn sum_network(networks: &Networks) -> (u64, u64) {
    let mut rx = 0u64;
    let mut tx = 0u64;
    for (_, data) in networks.iter() {
        rx = rx.saturating_add(data.total_received());
        tx = tx.saturating_add(data.total_transmitted());
    }
    (rx, tx)
}

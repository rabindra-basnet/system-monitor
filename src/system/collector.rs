use std::collections::VecDeque;
use std::time::Instant;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, Networks, RefreshKind, System};

pub const HISTORY_LEN: usize = 60;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CoreUsage {
    pub name: String,
    pub usage: f32,
    pub frequency_mhz: u64,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NetworkInterfaceItem {
    pub name: String,
    pub mac_address: String,
    pub total_rx: u64,
    pub total_tx: u64,
    pub rx_speed: u64,
    pub tx_speed: u64,
}

pub struct SystemCollector {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub cpu_history: VecDeque<u64>,
    pub mem_history: VecDeque<u64>,
    pub swap_history: VecDeque<u64>,
    pub net_rx_history: VecDeque<u64>,
    pub net_tx_history: VecDeque<u64>,
    pub last_net_rx: u64,
    pub last_net_tx: u64,
    pub current_rx_speed: u64,
    pub current_tx_speed: u64,
    pub last_refresh: Instant,
    pub os_name: String,
    pub kernel_version: String,
    pub host_name: String,
    pub cpu_model: String,
    pub cpu_count: usize,
    pub core_usages: Vec<CoreUsage>,
    pub interface_stats: Vec<NetworkInterfaceItem>,
    pub load_avg_one: f64,
    pub load_avg_five: f64,
    pub load_avg_fifteen: f64,
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut sys = System::new_with_specifics(
            RefreshKind::everything()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let os_name = System::name().unwrap_or_else(|| "Linux".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
        let host_name = System::host_name().unwrap_or_else(|| "localhost".to_string());
        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_string())
            .unwrap_or_else(|| "Generic CPU".to_string());
        let cpu_count = sys.cpus().len();

        let mut cpu_history = VecDeque::with_capacity(HISTORY_LEN);
        let mut mem_history = VecDeque::with_capacity(HISTORY_LEN);
        let mut swap_history = VecDeque::with_capacity(HISTORY_LEN);
        let mut net_rx_history = VecDeque::with_capacity(HISTORY_LEN);
        let mut net_tx_history = VecDeque::with_capacity(HISTORY_LEN);

        for _ in 0..HISTORY_LEN {
            cpu_history.push_back(0);
            mem_history.push_back(0);
            swap_history.push_back(0);
            net_rx_history.push_back(0);
            net_tx_history.push_back(0);
        }

        let (rx, tx) = Self::calculate_total_net(&networks);

        let mut collector = Self {
            sys,
            disks,
            networks,
            cpu_history,
            mem_history,
            swap_history,
            net_rx_history,
            net_tx_history,
            last_net_rx: rx,
            last_net_tx: tx,
            current_rx_speed: 0,
            current_tx_speed: 0,
            last_refresh: Instant::now(),
            os_name,
            kernel_version,
            host_name,
            cpu_model,
            cpu_count,
            core_usages: Vec::new(),
            interface_stats: Vec::new(),
            load_avg_one: 0.0,
            load_avg_five: 0.0,
            load_avg_fifteen: 0.0,
        };

        collector.refresh();
        collector
    }

    fn calculate_total_net(networks: &Networks) -> (u64, u64) {
        let mut total_rx = 0;
        let mut total_tx = 0;
        for data in networks.values() {
            total_rx += data.total_received();
            total_tx += data.total_transmitted();
        }
        (total_rx, total_tx)
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);

        let elapsed = self.last_refresh.elapsed().as_secs_f64().max(0.1);
        self.last_refresh = Instant::now();

        // CPU Global Usage
        let cpu_pct = self.sys.global_cpu_usage().round().clamp(0.0, 100.0) as u64;
        self.cpu_history.pop_front();
        self.cpu_history.push_back(cpu_pct);

        // Per-core usages
        self.core_usages = self
            .sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| CoreUsage {
                name: format!("Core {}", i),
                usage: cpu.cpu_usage().clamp(0.0, 100.0),
                frequency_mhz: cpu.frequency(),
            })
            .collect();

        // Memory Usage %
        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        let mem_pct = if total_mem > 0 {
            ((used_mem as f64 / total_mem as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u64
        } else {
            0
        };
        self.mem_history.pop_front();
        self.mem_history.push_back(mem_pct);

        // Swap Usage %
        let total_swap = self.sys.total_swap();
        let used_swap = self.sys.used_swap();
        let swap_pct = if total_swap > 0 {
            ((used_swap as f64 / total_swap as f64) * 100.0)
                .round()
                .clamp(0.0, 100.0) as u64
        } else {
            0
        };
        self.swap_history.pop_front();
        self.swap_history.push_back(swap_pct);

        // Load Averages
        let load_avg = System::load_average();
        self.load_avg_one = load_avg.one;
        self.load_avg_five = load_avg.five;
        self.load_avg_fifteen = load_avg.fifteen;

        // Network RX/TX speeds
        let (current_rx, current_tx) = Self::calculate_total_net(&self.networks);
        let rx_diff = current_rx.saturating_sub(self.last_net_rx);
        let tx_diff = current_tx.saturating_sub(self.last_net_tx);

        self.current_rx_speed = (rx_diff as f64 / elapsed) as u64;
        self.current_tx_speed = (tx_diff as f64 / elapsed) as u64;

        self.last_net_rx = current_rx;
        self.last_net_tx = current_tx;

        self.net_rx_history.pop_front();
        self.net_rx_history.push_back(self.current_rx_speed);

        self.net_tx_history.pop_front();
        self.net_tx_history.push_back(self.current_tx_speed);

        // Per-Interface stats
        self.interface_stats = self
            .networks
            .iter()
            .map(|(name, data)| NetworkInterfaceItem {
                name: name.to_string(),
                mac_address: data.mac_address().to_string(),
                total_rx: data.total_received(),
                total_tx: data.total_transmitted(),
                rx_speed: (data.received() as f64 / elapsed) as u64,
                tx_speed: (data.transmitted() as f64 / elapsed) as u64,
            })
            .collect();
        self.interface_stats.sort_by_key(|a| a.name.to_lowercase());
    }

    pub fn uptime_formatted(&self) -> String {
        let uptime = System::uptime();
        let days = uptime / 86400;
        let hours = (uptime % 86400) / 3600;
        let minutes = (uptime % 3600) / 60;
        let seconds = uptime % 60;

        if days > 0 {
            format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{:02}h {:02}m {:02}s", hours, minutes, seconds)
        } else {
            format!("{:02}m {:02}s", minutes, seconds)
        }
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

pub fn format_speed(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

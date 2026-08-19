use std::collections::VecDeque;
use std::process::Command;

pub const GPU_HISTORY_LEN: usize = 60;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GpuProcessItem {
    pub pid: u32,
    pub name: String,
    pub memory_mb: u64,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub driver: String,
    pub utilization: u16,
    pub vram_used_mb: u64,
    pub vram_total_mb: u64,
    pub temperature: u16,
    pub power_w: f32,
    pub history: VecDeque<u64>,
    pub processes: Vec<GpuProcessItem>,
}

pub struct GpuCollector {
    pub gpu: Option<GpuInfo>,
    pub is_available: bool,
}

impl GpuCollector {
    pub fn new() -> Self {
        let mut collector = Self {
            gpu: None,
            is_available: false,
        };
        collector.refresh();
        collector
    }

    pub fn refresh(&mut self) {
        if let Some(mut info) = self.query_nvidia() {
            if let Some(existing) = &self.gpu {
                info.history = existing.history.clone();
                info.history.pop_front();
                info.history.push_back(info.utilization as u64);
            }
            self.gpu = Some(info);
            self.is_available = true;
        } else if let Some(info) = self.query_drm_sysfs() {
            self.gpu = Some(info);
            self.is_available = true;
        } else {
            self.is_available = false;
        }
    }

    fn query_nvidia(&self) -> Option<GpuInfo> {
        let output = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,driver_version,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let line = text.lines().next()?;
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() < 6 {
            return None;
        }

        let name = parts[0].to_string();
        let driver = parts[1].to_string();
        let utilization = parts[2].parse::<u16>().unwrap_or(0);
        let vram_used_mb = parts[3].parse::<u64>().unwrap_or(0);
        let vram_total_mb = parts[4].parse::<u64>().unwrap_or(0);
        let temperature = parts[5].parse::<u16>().unwrap_or(0);
        let power_w = if parts.len() > 6 { parts[6].parse::<f32>().unwrap_or(0.0) } else { 0.0 };

        let mut history = VecDeque::with_capacity(GPU_HISTORY_LEN);
        for _ in 0..GPU_HISTORY_LEN {
            history.push_back(utilization as u64);
        }

        // Query active GPU compute / graphics apps
        let mut processes = Vec::new();
        if let Ok(proc_out) = Command::new("nvidia-smi")
            .args(["--query-compute-apps=pid,process_name,used_memory", "--format=csv,noheader,nounits"])
            .output()
        {
            let proc_text = String::from_utf8_lossy(&proc_out.stdout);
            for p_line in proc_text.lines() {
                let p_parts: Vec<&str> = p_line.split(',').map(|s| s.trim()).collect();
                if p_parts.len() >= 3 {
                    if let Ok(pid) = p_parts[0].parse::<u32>() {
                        let proc_name = p_parts[1].to_string();
                        let mem_mb = p_parts[2].parse::<u64>().unwrap_or(0);
                        processes.push(GpuProcessItem {
                            pid,
                            name: proc_name,
                            memory_mb: mem_mb,
                        });
                    }
                }
            }
        }

        Some(GpuInfo {
            name,
            driver,
            utilization,
            vram_used_mb,
            vram_total_mb,
            temperature,
            power_w,
            history,
            processes,
        })
    }

    fn query_drm_sysfs(&self) -> Option<GpuInfo> {
        // Fallback for AMD / Intel sysfs DRM nodes
        if std::path::Path::new("/sys/class/drm/card0/device/gpu_busy_percent").exists() {
            if let Ok(util_str) = std::fs::read_to_string("/sys/class/drm/card0/device/gpu_busy_percent") {
                let util = util_str.trim().parse::<u16>().unwrap_or(0);
                let mut history = VecDeque::with_capacity(GPU_HISTORY_LEN);
                for _ in 0..GPU_HISTORY_LEN {
                    history.push_back(util as u64);
                }

                return Some(GpuInfo {
                    name: "Integrated/Discrete GPU (DRM)".to_string(),
                    driver: "Mesa/DRM".to_string(),
                    utilization: util,
                    vram_used_mb: 0,
                    vram_total_mb: 0,
                    temperature: 0,
                    power_w: 0.0,
                    history,
                    processes: Vec::new(),
                });
            }
        }
        None
    }
}

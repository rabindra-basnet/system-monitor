use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkFilterMode {
    All,
    Listening,
    Established,
    Tcp,
    Udp,
}

impl NetworkFilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            NetworkFilterMode::All => "All Sockets",
            NetworkFilterMode::Listening => "Listening Ports",
            NetworkFilterMode::Established => "Established Connections",
            NetworkFilterMode::Tcp => "TCP Sockets",
            NetworkFilterMode::Udp => "UDP Sockets",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            NetworkFilterMode::All => NetworkFilterMode::Listening,
            NetworkFilterMode::Listening => NetworkFilterMode::Established,
            NetworkFilterMode::Established => NetworkFilterMode::Tcp,
            NetworkFilterMode::Tcp => NetworkFilterMode::Udp,
            NetworkFilterMode::Udp => NetworkFilterMode::All,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SocketEntry {
    pub proto: String,
    pub state: String,
    pub recv_q: u64,
    pub send_q: u64,
    pub local_addr: String,
    pub local_port: u16,
    pub peer_addr: String,
    pub peer_port: String,
    pub proc_name: String,
    pub pid: Option<u32>,
    pub is_system: bool,
}

#[derive(Debug, Clone)]
pub struct AppNetworkUsage {
    pub name: String,
    pub pid: Option<u32>,
    pub socket_count: usize,
    pub listening_count: usize,
    pub established_count: usize,
    pub tcp_count: usize,
    pub udp_count: usize,
    pub recv_q_bytes: u64,
    pub send_q_bytes: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NetworkSummary {
    pub listening_ports: Vec<SocketEntry>,
    pub active_connections: Vec<SocketEntry>,
    pub total_sockets: usize,
    pub top_network_processes: Vec<(String, usize)>,
    pub app_network_usage: Vec<AppNetworkUsage>,
}

pub struct NetworkManager {
    pub sockets: Vec<SocketEntry>,
    pub summary: NetworkSummary,
    pub filter: String,
    pub filter_mode: NetworkFilterMode,
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            sockets: Vec::new(),
            summary: NetworkSummary {
                listening_ports: Vec::new(),
                active_connections: Vec::new(),
                total_sockets: 0,
                top_network_processes: Vec::new(),
                app_network_usage: Vec::new(),
            },
            filter: String::new(),
            filter_mode: NetworkFilterMode::All,
        };
        mgr.refresh();
        mgr
    }

    pub fn refresh(&mut self) {
        let mut entries = Vec::new();

        // Scan via ss
        if let Ok(output) = Command::new("ss").args(["-H", "-tunap"]).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let proto = parts[0].to_uppercase();
                    let state = parts[1].to_uppercase();
                    let recv_q = parts[2].parse::<u64>().unwrap_or(0);
                    let send_q = parts[3].parse::<u64>().unwrap_or(0);
                    let local = parts[4];
                    let peer = if parts.len() > 5 { parts[5] } else { "*:*" };

                    let (local_ip, local_port) = match local.rsplit_once(':') {
                        Some((ip, port_str)) => {
                            (ip.to_string(), port_str.parse::<u16>().unwrap_or(0))
                        }
                        None => (local.to_string(), 0),
                    };

                    let (peer_ip, peer_port) = match peer.rsplit_once(':') {
                        Some((ip, port_str)) => (ip.to_string(), port_str.to_string()),
                        None => (peer.to_string(), "*".to_string()),
                    };

                    let mut proc_name = "-".to_string();
                    let mut pid = None;

                    if parts.len() > 6 {
                        let user_str = parts[6..].join(" ");
                        if let Some(start) = user_str.find("((\"") {
                            if let Some(end) = user_str[start + 3..].find('"') {
                                proc_name = user_str[start + 3..start + 3 + end].to_string();
                            }
                        }
                        if let Some(pid_start) = user_str.find("pid=") {
                            let after_pid = &user_str[pid_start + 4..];
                            let pid_digits: String = after_pid
                                .chars()
                                .take_while(|c| c.is_ascii_digit())
                                .collect();
                            if let Ok(p) = pid_digits.parse::<u32>() {
                                pid = Some(p);
                            }
                        }
                    }

                    let is_system = match pid {
                        Some(p) => {
                            p <= 1000 || proc_name == "systemd-resolved" || proc_name == "cupsd"
                        }
                        None => true,
                    };

                    entries.push(SocketEntry {
                        proto,
                        state,
                        recv_q,
                        send_q,
                        local_addr: local_ip,
                        local_port,
                        peer_addr: peer_ip,
                        peer_port,
                        proc_name,
                        pid,
                        is_system,
                    });
                }
            }
        }

        let mut listening = Vec::new();
        let mut active = Vec::new();
        let mut app_stats: HashMap<String, AppNetworkUsage> = HashMap::new();

        for entry in &entries {
            if entry.proc_name != "-" {
                let stat =
                    app_stats
                        .entry(entry.proc_name.clone())
                        .or_insert_with(|| AppNetworkUsage {
                            name: entry.proc_name.clone(),
                            pid: entry.pid,
                            socket_count: 0,
                            listening_count: 0,
                            established_count: 0,
                            tcp_count: 0,
                            udp_count: 0,
                            recv_q_bytes: 0,
                            send_q_bytes: 0,
                        });

                stat.socket_count += 1;
                stat.recv_q_bytes += entry.recv_q;
                stat.send_q_bytes += entry.send_q;

                if entry.proto.contains("TCP") {
                    stat.tcp_count += 1;
                } else if entry.proto.contains("UDP") {
                    stat.udp_count += 1;
                }

                if entry.state == "LISTEN" {
                    stat.listening_count += 1;
                } else if entry.state == "ESTAB" || entry.state == "ESTABLISHED" {
                    stat.established_count += 1;
                }
            }

            if entry.state == "LISTEN" {
                listening.push(entry.clone());
            } else if entry.state == "ESTAB" || entry.state == "ESTABLISHED" {
                active.push(entry.clone());
            }
        }

        listening.sort_by_key(|e| e.local_port);

        let mut app_usages: Vec<AppNetworkUsage> = app_stats.into_values().collect();
        app_usages.sort_by_key(|b| std::cmp::Reverse(b.socket_count));

        let mut top_procs: Vec<(String, usize)> = app_usages
            .iter()
            .map(|u| (u.name.clone(), u.socket_count))
            .collect();
        top_procs.truncate(6);

        self.summary = NetworkSummary {
            total_sockets: entries.len(),
            listening_ports: listening,
            active_connections: active,
            top_network_processes: top_procs,
            app_network_usage: app_usages,
        };

        self.sockets = entries;
    }

    pub fn filtered_sockets(&self) -> Vec<&SocketEntry> {
        let q = self.filter.to_lowercase();
        self.sockets
            .iter()
            .filter(|s| match self.filter_mode {
                NetworkFilterMode::All => true,
                NetworkFilterMode::Listening => s.state == "LISTEN",
                NetworkFilterMode::Established => s.state == "ESTAB" || s.state == "ESTABLISHED",
                NetworkFilterMode::Tcp => s.proto.contains("TCP"),
                NetworkFilterMode::Udp => s.proto.contains("UDP"),
            })
            .filter(|s| {
                if q.is_empty() {
                    return true;
                }
                s.proc_name.to_lowercase().contains(&q)
                    || s.local_port.to_string().contains(&q)
                    || s.local_addr.to_lowercase().contains(&q)
                    || s.peer_addr.to_lowercase().contains(&q)
                    || s.peer_port.to_lowercase().contains(&q)
                    || s.proto.to_lowercase().contains(&q)
                    || s.state.to_lowercase().contains(&q)
                    || s.pid.map(|p| p.to_string().contains(&q)).unwrap_or(false)
            })
            .collect()
    }

    pub fn kill_port(
        &self,
        port: u16,
        proto: &str,
        pid: Option<u32>,
        sudo_password: Option<&str>,
    ) -> Result<(), String> {
        let port_str = port.to_string();
        let proto_lower = proto.to_lowercase();

        if let Some(p) = pid {
            let pid_str = p.to_string();
            let res = Command::new("kill").args(["-9", &pid_str]).output();
            if let Ok(out) = res {
                if out.status.success() {
                    return Ok(());
                }
            }
            return crate::system::sudo::run_elevated_command(
                "kill",
                &["-9", &pid_str],
                sudo_password,
            )
            .map(|_| ());
        }

        crate::system::sudo::run_elevated_command(
            "fuser",
            &["-k", "-n", &proto_lower, &port_str],
            sudo_password,
        )
        .map(|_| ())
    }
}

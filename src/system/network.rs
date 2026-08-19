use std::collections::HashMap;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SocketEntry {
    pub proto: String,
    pub state: String,
    pub local_addr: String,
    pub local_port: u16,
    pub peer_addr: String,
    pub peer_port: String,
    pub proc_name: String,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NetworkSummary {
    pub listening_ports: Vec<SocketEntry>,
    pub active_connections: Vec<SocketEntry>,
    pub total_sockets: usize,
    pub top_network_processes: Vec<(String, usize)>,
}

pub struct NetworkManager {
    pub sockets: Vec<SocketEntry>,
    pub summary: NetworkSummary,
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
            },
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
                    let local = parts[4];
                    let peer = if parts.len() > 5 { parts[5] } else { "*:*" };

                    let (local_ip, local_port) = match local.rsplit_once(':') {
                        Some((ip, port_str)) => (ip.to_string(), port_str.parse::<u16>().unwrap_or(0)),
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
                            let pid_digits: String = after_pid.chars().take_while(|c| c.is_ascii_digit()).collect();
                            if let Ok(p) = pid_digits.parse::<u32>() {
                                pid = Some(p);
                            }
                        }
                    }

                    entries.push(SocketEntry {
                        proto,
                        state,
                        local_addr: local_ip,
                        local_port,
                        peer_addr: peer_ip,
                        peer_port,
                        proc_name,
                        pid,
                    });
                }
            }
        }

        let mut listening = Vec::new();
        let mut active = Vec::new();
        let mut proc_counts: HashMap<String, usize> = HashMap::new();

        for entry in &entries {
            if entry.proc_name != "-" {
                *proc_counts.entry(entry.proc_name.clone()).or_insert(0) += 1;
            }

            if entry.state == "LISTEN" {
                listening.push(entry.clone());
            } else if entry.state == "ESTAB" || entry.state == "ESTABLISHED" {
                active.push(entry.clone());
            }
        }

        // Sort listening by port number
        listening.sort_by_key(|e| e.local_port);

        // Sort proc counts
        let mut top_procs: Vec<(String, usize)> = proc_counts.into_iter().collect();
        top_procs.sort_by(|a, b| b.1.cmp(&a.1));
        top_procs.truncate(5);

        self.summary = NetworkSummary {
            total_sockets: entries.len(),
            listening_ports: listening,
            active_connections: active,
            top_network_processes: top_procs,
        };

        self.sockets = entries;
    }
}

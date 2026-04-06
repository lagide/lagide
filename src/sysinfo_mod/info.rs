use sysinfo::System;

#[derive(Debug, Clone)]
pub struct SystemInfoData {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub cpu_count: usize,
    pub cpu_usage: f32,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub total_swap_mb: u64,
    pub used_swap_mb: u64,
    pub uptime_secs: u64,
}

pub struct SystemInfo {
    system: System,
}

impl SystemInfo {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
    }

    pub fn refresh(&mut self) {
        self.system.refresh_all();
    }

    pub fn gather(&mut self) -> SystemInfoData {
        self.system.refresh_all();

        let cpu_usage = self.system.global_cpu_usage();

        SystemInfoData {
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            os_name: System::name().unwrap_or_else(|| "unknown".to_string()),
            os_version: System::os_version().unwrap_or_else(|| "unknown".to_string()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
            cpu_count: self.system.cpus().len(),
            cpu_usage,
            total_memory_mb: self.system.total_memory() / (1024 * 1024),
            used_memory_mb: self.system.used_memory() / (1024 * 1024),
            total_swap_mb: self.system.total_swap() / (1024 * 1024),
            used_swap_mb: self.system.used_swap() / (1024 * 1024),
            uptime_secs: System::uptime(),
        }
    }

    pub fn get_processes(&mut self) -> Vec<ProcessInfo> {
        self.system.refresh_all();
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                cpu_usage: process.cpu_usage(),
                memory_mb: process.memory() / (1024 * 1024),
            })
            .collect()
    }

    pub fn get_networks(&self) -> Vec<NetworkInfo> {
        use sysinfo::Networks;
        let networks = Networks::new_with_refreshed_list();
        networks
            .iter()
            .map(|(name, data)| NetworkInfo {
                interface: name.clone(),
                received_bytes: data.total_received(),
                transmitted_bytes: data.total_transmitted(),
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: u64,
}

#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub interface: String,
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

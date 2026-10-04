pub mod affinity;
pub mod memory;
pub mod process;
pub mod settings;
pub mod storage;
pub mod topology;

pub type Result<T> = std::result::Result<T, String>;
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub mod network;
pub mod powershell;

pub mod nic;

pub mod affinity_worker;

pub mod command;
pub mod graphics;

pub mod dxvk;
pub mod http;

pub mod muo;

pub mod app_services;

pub mod blackbox;
pub mod boost;
pub mod channel_ping;
pub mod dxvk_manager;
pub mod inputs;
pub mod network_manager;
pub mod rpc;
pub mod service_windows;
pub mod workers;

pub mod diagnostics;
pub mod service;
pub mod service_fixture;

mod helper_log;
pub mod updater;
pub mod update_install;

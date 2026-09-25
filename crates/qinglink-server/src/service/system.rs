use std::alloc::{Layout, alloc, dealloc};
use std::net::Ipv4Addr;
use std::sync::Mutex;

use sysinfo::System;
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows::Win32::Networking::WinSock::AF_INET;
use windows::Win32::Networking::WinSock::AF_UNSPEC;

use crate::platform::SystemController;
use crate::protocol::responses::SystemStatus;
use qinglink_protocol::WakeNetworkAdapter;

fn wide_description(pointer: windows::core::PWSTR) -> String {
    if pointer.is_null() {
        return String::new();
    }
    unsafe {
        let mut length = 0;
        while *pointer.0.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(pointer.0, length))
    }
}

pub struct SystemService {
    system: Mutex<System>,
}

impl Default for SystemService {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemService {
    pub fn new() -> Self {
        Self {
            system: Mutex::new(System::new_all()),
        }
    }

    /// Return physical IPv4 adapters suitable for Wake-on-LAN.
    pub fn get_wake_networks(&self) -> Vec<WakeNetworkAdapter> {
        const VIRTUAL: &[&str] = &[
            "virtual",
            "vmware",
            "hyper-v",
            "hyperv",
            "wsl",
            "vpn",
            "tap",
            "tun",
            "loopback",
            "bluetooth",
            "miniport",
            "ndis",
            "remote ndis",
            "virtualbox",
            "vbox",
            "ppp",
            "ras",
        ];
        fn virtual_adapter(description: &str) -> bool {
            let lower = description.to_lowercase();
            VIRTUAL.iter().any(|keyword| lower.contains(keyword))
        }
        unsafe {
            let mut size = 0u32;
            let result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                None,
                &mut size,
            );
            if result != 0x000000EA || size == 0 {
                return Vec::new();
            }
            let layout = Layout::from_size_align_unchecked(size as usize, 8);
            let buffer = alloc(layout);
            if buffer.is_null() {
                return Vec::new();
            }
            let result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                Some(buffer as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );
            if result != 0 {
                dealloc(buffer, layout);
                return Vec::new();
            }
            let mut networks = Vec::new();
            let mut adapter = buffer as *mut IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                let physical_length = (*adapter).PhysicalAddressLength as usize;
                if (*adapter).IfType == 6
                    && (*adapter).OperStatus.0 == 1
                    && physical_length >= 6
                    && !virtual_adapter(&wide_description((*adapter).Description))
                {
                    let mut unicast = (*adapter).FirstUnicastAddress;
                    while !unicast.is_null() {
                        let sockaddr = (*unicast).Address.lpSockaddr;
                        if !sockaddr.is_null() && (*sockaddr).sa_family == AF_INET {
                            let raw = std::slice::from_raw_parts(sockaddr as *const u8, 8);
                            let ip = Ipv4Addr::new(raw[4], raw[5], raw[6], raw[7]);
                            let prefix = (*unicast).OnLinkPrefixLength;
                            if prefix > 0 && prefix <= 32 {
                                let ip_num = u32::from(ip);
                                let mask = if prefix == 32 {
                                    u32::MAX
                                } else {
                                    u32::MAX << (32 - prefix)
                                };
                                let broadcast = Ipv4Addr::from(ip_num | !mask);
                                let mac = format!(
                                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                                    (*adapter).PhysicalAddress[0],
                                    (*adapter).PhysicalAddress[1],
                                    (*adapter).PhysicalAddress[2],
                                    (*adapter).PhysicalAddress[3],
                                    (*adapter).PhysicalAddress[4],
                                    (*adapter).PhysicalAddress[5]
                                );
                                networks.push(WakeNetworkAdapter {
                                    mac_address: mac,
                                    ipv4_address: ip.to_string(),
                                    prefix_length: prefix,
                                    broadcast_address: broadcast.to_string(),
                                });
                            }
                        }
                        unicast = (*unicast).Next;
                    }
                }
                adapter = (*adapter).Next;
            }
            dealloc(buffer, layout);
            networks
        }
    }

    pub fn get_status(&self) -> SystemStatus {
        let mut system = crate::lock_unpoisoned(&self.system);
        system.refresh_all();

        let cpu_usage = system.global_cpu_usage();
        let memory_total = system.total_memory();
        let memory_used = system.used_memory();
        let memory_percent = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };
        let uptime_seconds = System::uptime();

        SystemStatus {
            cpu_usage,
            memory_total,
            memory_used,
            memory_percent,
            uptime_seconds,
        }
    }

    /// Get the physical MAC address (for Wake-on-LAN capable adapters).
    pub fn get_mac_address(&self) -> Option<String> {
        if let Some(network) = self.get_wake_networks().into_iter().next() {
            return Some(network.mac_address);
        }
        // Use windows crate to get MAC address via GetAdaptersAddresses

        // Virtual adapter keywords (case-insensitive matching).
        const VIRTUAL_ADAPTER_KEYWORDS: &[&str] = &[
            "virtual",
            "vmware",
            "hyper-v",
            "hyperv",
            "wsl",
            "vpn",
            "tap",
            "tun",
            "loopback",
            "bluetooth",
            "miniport",
            "ndis",
            "wan miniport",
            "microsoft kernel debug",
            "remote ndis",
            "virtualbox",
            "vbox",
            "ppp",
            "ras",
        ];

        // 检查是否为虚拟网卡
        fn is_virtual_adapter(description: &str) -> bool {
            let desc_lower = description.to_lowercase();
            VIRTUAL_ADAPTER_KEYWORDS
                .iter()
                .any(|kw| desc_lower.contains(kw))
        }

        unsafe {
            let mut size: u32 = 0;
            let result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                None,
                &mut size,
            );
            if result != 0x000000EA && size == 0 {
                // ERROR_BUFFER_OVERFLOW = 0x6F (111)
                return None;
            }

            let layout = Layout::from_size_align_unchecked(size as usize, 8);
            let buffer = alloc(layout);
            if buffer.is_null() {
                return None;
            }

            let result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                Some(buffer as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );

            if result != 0 {
                dealloc(buffer, layout);
                return None;
            }

            // 第一遍：优先查找物理以太网适配器（IfType = 6，IF_TYPE_ETHERNET_CSMACD）
            let mut adapter = buffer as *mut IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                let physical_address = (*adapter).PhysicalAddress;
                let physical_address_length = (*adapter).PhysicalAddressLength as usize;
                let if_type = (*adapter).IfType;

                // 只考虑有 MAC 地址的适配器
                if physical_address_length >= 6 {
                    // 获取适配器描述
                    let description = (*adapter).Description;
                    let desc_str = {
                        let mut len = 0;
                        while *description.0.add(len) != 0 {
                            len += 1;
                        }
                        let slice: &[u16] =
                            std::slice::from_raw_parts(description.0 as *const u16, len);
                        String::from_utf16_lossy(slice)
                    };

                    // 过滤掉虚拟网卡
                    if !is_virtual_adapter(&desc_str) {
                        // 优先选择以太网适配器（IfType = 6）
                        if if_type == 6 {
                            let mac = format!(
                                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                                physical_address[0],
                                physical_address[1],
                                physical_address[2],
                                physical_address[3],
                                physical_address[4],
                                physical_address[5]
                            );
                            dealloc(buffer, layout);
                            return Some(mac);
                        }
                    }
                }
                adapter = (*adapter).Next;
            }

            // 第二遍：如果没有找到以太网适配器，找任何非虚拟的物理网卡
            adapter = buffer as *mut IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                let physical_address = (*adapter).PhysicalAddress;
                let physical_address_length = (*adapter).PhysicalAddressLength as usize;

                if physical_address_length >= 6 {
                    let description = (*adapter).Description;
                    let desc_str = {
                        let mut len = 0;
                        while *description.0.add(len) != 0 {
                            len += 1;
                        }
                        let slice: &[u16] =
                            std::slice::from_raw_parts(description.0 as *const u16, len);
                        String::from_utf16_lossy(slice)
                    };

                    if !is_virtual_adapter(&desc_str) {
                        let mac = format!(
                            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                            physical_address[0],
                            physical_address[1],
                            physical_address[2],
                            physical_address[3],
                            physical_address[4],
                            physical_address[5]
                        );
                        dealloc(buffer, layout);
                        return Some(mac);
                    }
                }
                adapter = (*adapter).Next;
            }

            dealloc(buffer, layout);
            None
        }
    }

    /// 获取子网前缀长度（如 24 表示 /24）
    pub fn get_subnet_prefix(&self) -> Option<u8> {
        if let Some(network) = self.get_wake_networks().into_iter().next() {
            return Some(network.prefix_length);
        }
        use std::alloc::{Layout, alloc, dealloc};
        use windows::Win32::NetworkManagement::IpHelper::{
            GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
        };
        use windows::Win32::Networking::WinSock::AF_INET;

        unsafe {
            let mut size: u32 = 0;
            let result = GetAdaptersAddresses(
                AF_INET.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                None,
                &mut size,
            );
            if result != 0x000000EA && size == 0 {
                return None;
            }

            let layout = Layout::from_size_align_unchecked(size as usize, 8);
            let buffer = alloc(layout);
            if buffer.is_null() {
                return None;
            }

            let result = GetAdaptersAddresses(
                AF_INET.0 as u32,
                GAA_FLAG_INCLUDE_PREFIX,
                None,
                Some(buffer as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );

            if result != 0 {
                dealloc(buffer, layout);
                return None;
            }

            let mut adapter = buffer as *mut IP_ADAPTER_ADDRESSES_LH;
            while !adapter.is_null() {
                if (*adapter).IfType == 6 {
                    let addr = (*adapter).FirstUnicastAddress;
                    if !addr.is_null() {
                        let prefix = (*addr).OnLinkPrefixLength;
                        dealloc(buffer, layout);
                        return Some(prefix);
                    }
                }
                adapter = (*adapter).Next;
            }

            dealloc(buffer, layout);
            None
        }
    }

    pub fn shutdown(&self, force: bool, delay_secs: u64) -> anyhow::Result<()> {
        SystemController::shutdown(force, delay_secs)
    }

    pub fn sleep(&self, hibernate: bool) -> anyhow::Result<()> {
        SystemController::sleep(hibernate)
    }

    pub fn restart(&self, force: bool, delay_secs: u64) -> anyhow::Result<()> {
        SystemController::restart(force, delay_secs)
    }

    pub fn logoff(&self) -> anyhow::Result<()> {
        SystemController::logoff()
    }

    pub fn lock(&self) -> anyhow::Result<()> {
        SystemController::lock()
    }

    pub fn get_unlock_status(&self) -> anyhow::Result<crate::protocol::responses::UnlockStatus> {
        crate::platform::get_unlock_status()
    }

    pub fn auto_unlock(&self, pin: Option<&str>) -> anyhow::Result<()> {
        crate::platform::auto_unlock(pin)
    }

    pub fn configure_no_password_wake(&self) -> anyhow::Result<()> {
        crate::platform::configure_no_password_wake()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn get_status_returns_valid_data() {
        let service = SystemService::new();
        let status = service.get_status();

        assert!(status.memory_total > 0, "内存总量应大于 0");
        assert!(
            status.memory_used <= status.memory_total,
            "已用内存不应超过总量"
        );
        assert!(
            status.memory_percent >= 0.0 && status.memory_percent <= 100.0,
            "内存占用百分比应在 0-100 之间"
        );
        assert!(status.uptime_seconds > 0, "运行时间应大于 0");
    }

    #[test]
    fn get_mac_address_format() {
        let service = SystemService::new();
        let mac = service.get_mac_address();

        if let Some(mac) = mac {
            // MAC 地址格式: XX:XX:XX:XX:XX:XX
            let parts: Vec<&str> = mac.split(':').collect();
            assert_eq!(parts.len(), 6, "MAC 地址应有 6 段");

            for part in &parts {
                assert_eq!(part.len(), 2, "每段应为 2 个十六进制字符");
                assert!(
                    u8::from_str_radix(part, 16).is_ok(),
                    "每段应为有效的十六进制数"
                );
            }
        }
    }
}

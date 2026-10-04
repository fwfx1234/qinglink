//! 网络信息：唤醒网卡枚举、物理 MAC 地址与子网前缀查询。
//!
//! 三者都基于 `GetAdaptersAddresses` 的两阶段调用（先探测缓冲区大小、再枚举），
//! 这里用 [`AdapterBuffer`] 统一管理该缓冲区的分配与释放。

use std::alloc::{Layout, alloc, dealloc};
use std::net::Ipv4Addr;

use qinglink_protocol::WakeNetworkAdapter;
use windows::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_UNICAST_ADDRESS_LH,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_UNSPEC};

use super::SystemService;

/// 唤醒网卡枚举使用的虚拟网卡关键字（大小写不敏感）。
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

fn virtual_adapter(description: &str) -> bool {
    let lower = description.to_lowercase();
    VIRTUAL.iter().any(|keyword| lower.contains(keyword))
}

/// 检查是否为虚拟网卡
fn is_virtual_adapter(description: &str) -> bool {
    let desc_lower = description.to_lowercase();
    VIRTUAL_ADAPTER_KEYWORDS
        .iter()
        .any(|kw| desc_lower.contains(kw))
}

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

/// 首轮 `GetAdaptersAddresses` 的探测结果：返回码与所需缓冲区大小。
struct AdapterProbe {
    result: u32,
    size: u32,
}

/// 适配器地址缓冲区：持有枚举期间的内存，离开作用域时按分配时的 `Layout` 释放。
struct AdapterBuffer {
    pointer: *mut IP_ADAPTER_ADDRESSES_LH,
    layout: Layout,
}

impl Drop for AdapterBuffer {
    fn drop(&mut self) {
        // SAFETY: pointer 由 fetch_adapters 用同一 layout 分配，且只在此处释放一次。
        unsafe { dealloc(self.pointer as *mut u8, self.layout) }
    }
}

/// 调用 `GetAdaptersAddresses` 探测枚举所需的缓冲区大小。
fn probe_adapters(family: u32) -> AdapterProbe {
    let mut size = 0u32;
    // SAFETY: 首轮调用只查询所需大小，传入空缓冲区指针，不读取地址列表。
    let result =
        unsafe { GetAdaptersAddresses(family, GAA_FLAG_INCLUDE_PREFIX, None, None, &mut size) };
    AdapterProbe { result, size }
}

/// 按探测到的大小分配缓冲区并完成第二轮枚举；分配或调用失败时返回 `None`。
fn fetch_adapters(family: u32, size: u32) -> Option<AdapterBuffer> {
    // SAFETY: layout 由 GetAdaptersAddresses 报告的大小构造，8 字节对齐与 Windows 结构体一致。
    let layout = unsafe { Layout::from_size_align_unchecked(size as usize, 8) };
    // SAFETY: 分配的内存交由 AdapterBuffer 用同一 layout 释放。
    let pointer = unsafe { alloc(layout) } as *mut IP_ADAPTER_ADDRESSES_LH;
    if pointer.is_null() {
        return None;
    }
    let buffer = AdapterBuffer { pointer, layout };

    let mut requested = size;
    // SAFETY: buffer 至少可容纳 requested 字节，调用成功时由系统填充适配器链表。
    let result = unsafe {
        GetAdaptersAddresses(
            family,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            Some(buffer.pointer),
            &mut requested,
        )
    };
    if result != 0 {
        return None;
    }
    Some(buffer)
}

/// 把适配器物理地址的前 6 字节格式化为 `XX:XX:XX:XX:XX:XX`。
fn format_mac(physical_address: &[u8]) -> String {
    format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        physical_address[0],
        physical_address[1],
        physical_address[2],
        physical_address[3],
        physical_address[4],
        physical_address[5]
    )
}

/// 由单个单播地址条目生成唤醒网卡信息；非 IPv4 或前缀长度非法时返回 `None`。
fn wake_network_from_unicast(
    adapter: *mut IP_ADAPTER_ADDRESSES_LH,
    unicast: *mut IP_ADAPTER_UNICAST_ADDRESS_LH,
) -> Option<WakeNetworkAdapter> {
    // SAFETY: 两个指针都来自 GetAdaptersAddresses 填充的链表；调用方保证 unicast 非空，
    // sockaddr 在解引用前已做空指针检查。
    unsafe {
        let sockaddr = (*unicast).Address.lpSockaddr;
        if sockaddr.is_null() || (*sockaddr).sa_family != AF_INET {
            return None;
        }
        let raw = std::slice::from_raw_parts(sockaddr as *const u8, 8);
        let ip = Ipv4Addr::new(raw[4], raw[5], raw[6], raw[7]);
        let prefix = (*unicast).OnLinkPrefixLength;
        if prefix == 0 || prefix > 32 {
            return None;
        }
        let ip_num = u32::from(ip);
        let mask = if prefix == 32 {
            u32::MAX
        } else {
            u32::MAX << (32 - prefix)
        };
        let broadcast = Ipv4Addr::from(ip_num | !mask);
        Some(WakeNetworkAdapter {
            mac_address: format_mac(&(*adapter).PhysicalAddress),
            ipv4_address: ip.to_string(),
            prefix_length: prefix,
            broadcast_address: broadcast.to_string(),
        })
    }
}

/// 筛选已启用、带 MAC 地址的物理以太网适配器，生成唤醒网卡列表。
fn collect_wake_networks(buffer: &AdapterBuffer) -> Vec<WakeNetworkAdapter> {
    let mut networks = Vec::new();
    let mut adapter = buffer.pointer;
    while !adapter.is_null() {
        // SAFETY: adapter 来自 GetAdaptersAddresses 填充的链表，循环以链尾空指针结束。
        unsafe {
            let physical_length = (*adapter).PhysicalAddressLength as usize;
            if (*adapter).IfType == 6
                && (*adapter).OperStatus.0 == 1
                && physical_length >= 6
                && !virtual_adapter(&wide_description((*adapter).Description))
            {
                let mut unicast = (*adapter).FirstUnicastAddress;
                while !unicast.is_null() {
                    if let Some(network) = wake_network_from_unicast(adapter, unicast) {
                        networks.push(network);
                    }
                    unicast = (*unicast).Next;
                }
            }
            adapter = (*adapter).Next;
        }
    }
    networks
}

/// 查找物理网卡的 MAC 地址；`required_if_type` 为 `Some` 时只接受该适配器类型。
fn find_physical_mac(buffer: &AdapterBuffer, required_if_type: Option<u32>) -> Option<String> {
    let mut adapter = buffer.pointer;
    while !adapter.is_null() {
        // SAFETY: adapter 来自 GetAdaptersAddresses 填充的链表，循环以链尾空指针结束。
        unsafe {
            // 只考虑有 MAC 地址的适配器
            if (*adapter).PhysicalAddressLength as usize >= 6 {
                let desc_str = wide_description((*adapter).Description);

                // 过滤掉虚拟网卡
                if !is_virtual_adapter(&desc_str)
                    && required_if_type.is_none_or(|if_type| (*adapter).IfType == if_type)
                {
                    return Some(format_mac(&(*adapter).PhysicalAddress));
                }
            }
            adapter = (*adapter).Next;
        }
    }
    None
}

impl SystemService {
    /// Return physical IPv4 adapters suitable for Wake-on-LAN.
    pub fn get_wake_networks(&self) -> Vec<WakeNetworkAdapter> {
        let probe = probe_adapters(AF_UNSPEC.0 as u32);
        if probe.result != 0x000000EA || probe.size == 0 {
            return Vec::new();
        }
        let Some(buffer) = fetch_adapters(AF_UNSPEC.0 as u32, probe.size) else {
            return Vec::new();
        };
        collect_wake_networks(&buffer)
    }

    /// Get the physical MAC address (for Wake-on-LAN capable adapters).
    pub fn get_mac_address(&self) -> Option<String> {
        if let Some(network) = self.get_wake_networks().into_iter().next() {
            return Some(network.mac_address);
        }
        // Use windows crate to get MAC address via GetAdaptersAddresses

        let probe = probe_adapters(AF_UNSPEC.0 as u32);
        if probe.result != 0x000000EA && probe.size == 0 {
            // ERROR_BUFFER_OVERFLOW = 0x6F (111)
            return None;
        }
        let buffer = fetch_adapters(AF_UNSPEC.0 as u32, probe.size)?;

        // 第一遍：优先查找物理以太网适配器（IfType = 6，IF_TYPE_ETHERNET_CSMACD）
        if let Some(mac) = find_physical_mac(&buffer, Some(6)) {
            return Some(mac);
        }

        // 第二遍：如果没有找到以太网适配器，找任何非虚拟的物理网卡
        find_physical_mac(&buffer, None)
    }

    /// 获取子网前缀长度（如 24 表示 /24）
    pub fn get_subnet_prefix(&self) -> Option<u8> {
        if let Some(network) = self.get_wake_networks().into_iter().next() {
            return Some(network.prefix_length);
        }

        let probe = probe_adapters(AF_INET.0 as u32);
        if probe.result != 0x000000EA && probe.size == 0 {
            return None;
        }
        let buffer = fetch_adapters(AF_INET.0 as u32, probe.size)?;

        let mut adapter = buffer.pointer;
        while !adapter.is_null() {
            // SAFETY: adapter 来自 GetAdaptersAddresses 填充的链表，循环以链尾空指针结束。
            unsafe {
                if (*adapter).IfType == 6 {
                    let addr = (*adapter).FirstUnicastAddress;
                    if !addr.is_null() {
                        let prefix = (*addr).OnLinkPrefixLength;
                        return Some(prefix);
                    }
                }
                adapter = (*adapter).Next;
            }
        }
        None
    }
}

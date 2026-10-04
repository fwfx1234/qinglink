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

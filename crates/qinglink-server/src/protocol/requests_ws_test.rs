//! WebSocket 协议帧的序列化测试（自 `requests_ws.rs` 原样搬运，未新增或删除用例）。
//!
//! 独立成文件是为了让源码文件保持在 AGENTS.md §2.1 的规模上限以内；
//! 由于本文件挂在 `requests_ws` 子模块下，测试改用绝对路径引用被测类型。

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod secure_desktop_protocol_tests {
    use crate::protocol::requests::{WsClientRequest, WsServerMessage, WsServerResponse};

    #[test]
    fn secure_requests_use_stable_snake_case_wire_names() {
        let status: WsClientRequest =
            serde_json::from_str(r#"{"type":"get_secure_desktop_status"}"#).unwrap();
        assert!(matches!(status, WsClientRequest::GetSecureDesktopStatus));
        let sas: WsClientRequest = serde_json::from_str(r#"{"type":"secure_attention"}"#).unwrap();
        assert!(matches!(sas, WsClientRequest::SecureAttention));
        let ascii: WsClientRequest =
            serde_json::from_str(r#"{"type":"send_secure_ascii","text":"Admin123!"}"#).unwrap();
        assert!(matches!(ascii, WsClientRequest::SendSecureAscii { text } if text == "Admin123!"));
    }

    #[test]
    fn secure_status_response_contains_all_state_fields() {
        let message = WsServerMessage::Response(WsServerResponse::SecureDesktopStatus(
            crate::protocol::responses::SecureDesktopStatus {
                desktop: "winlogon".into(),
                locked: true,
                input_ready: true,
                sas_supported: true,
                session_id: 7,
                agent_state: "ready".into(),
                uac_detected: false,
                token_integrity_level: 0,
                last_error: String::new(),
            },
        ));
        let value = serde_json::to_value(message).unwrap();
        assert_eq!(value["type"], "secure_desktop_status");
        assert_eq!(value["data"]["session_id"], 7);
        assert_eq!(value["data"]["input_ready"], true);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::protocol::requests::*;

    #[test]
    fn deserialize_auth_pair_request() {
        let json = r#"{"pin": "123456"}"#;
        let req: AuthPairRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.pin, "123456");
    }

    #[test]
    fn deserialize_shutdown_request_default() {
        let json = r#"{}"#;
        let req: ShutdownRequest = serde_json::from_str(json).unwrap();
        assert!(!req.force);
        assert_eq!(req.delay_secs, 0);
    }

    #[test]
    fn deserialize_shutdown_request_with_fields() {
        let json = r#"{"force": true, "delay_secs": 30}"#;
        let req: ShutdownRequest = serde_json::from_str(json).unwrap();
        assert!(req.force);
        assert_eq!(req.delay_secs, 30);
    }

    #[test]
    fn deserialize_sleep_request() {
        let json = r#"{"hibernate": true}"#;
        let req: SleepRequest = serde_json::from_str(json).unwrap();
        assert!(req.hibernate);
    }

    #[test]
    fn deserialize_restart_request_with_delay() {
        let request: WsClientRequest =
            serde_json::from_str(r#"{"type":"restart","force":true,"delay_secs":15}"#).unwrap();
        assert!(matches!(
            request,
            WsClientRequest::Restart {
                force: true,
                delay_secs: 15
            }
        ));
    }

    #[test]
    fn deserialize_process_list_query_default() {
        let json = r#"{}"#;
        let req: ProcessListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(req.search, None);
        assert_eq!(req.page, 0);
        assert_eq!(req.page_size, 50);
    }

    #[test]
    fn deserialize_process_list_query_with_search() {
        let json = r#"{"search": "notepad", "page": 1, "page_size": 20}"#;
        let req: ProcessListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(req.search, Some("notepad".to_string()));
        assert_eq!(req.page, 1);
        assert_eq!(req.page_size, 20);
    }

    #[test]
    fn deserialize_launch_app_request() {
        let json = r#"{"path": "C:\\Windows\\System32\\notepad.exe", "args": ["file.txt"]}"#;
        let req: LaunchAppRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "C:\\Windows\\System32\\notepad.exe");
        assert_eq!(req.args, vec!["file.txt".to_string()]);
    }

    #[test]
    fn deserialize_launch_app_request_no_args() {
        let json = r#"{"path": "C:\\Windows\\System32\\calc.exe"}"#;
        let req: LaunchAppRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "C:\\Windows\\System32\\calc.exe");
        assert!(req.args.is_empty());
    }

    #[test]
    fn deserialize_add_custom_dir_request() {
        let json = r#"{"path": "D:\\Games", "name": "游戏目录", "enabled": true, "max_depth": 3, "recursive": true}"#;
        let req: AddCustomDirRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.path, "D:\\Games");
        assert_eq!(req.name, Some("游戏目录".to_string()));
        assert!(req.enabled);
        assert_eq!(req.max_depth, 3);
        assert!(req.recursive);
    }

    #[test]
    fn deserialize_update_custom_dir_request() {
        let json = r#"{"name": "新名称", "enabled": false}"#;
        let req: UpdateCustomDirRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, Some("新名称".to_string()));
        assert_eq!(req.enabled, Some(false));
        assert_eq!(req.path, None);
    }

    #[test]
    fn deserialize_update_settings_request() {
        let json = r#"{"port": 8080, "auto_start": true, "minimize_to_tray": false}"#;
        let req: UpdateSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.port, Some(8080));
        assert_eq!(req.auto_start, Some(true));
        assert_eq!(req.minimize_to_tray, Some(false));
    }

    #[test]
    fn serialize_ws_event_foreground() {
        let event = WsEvent::ForegroundChanged {
            data: ForegroundInfo {
                pid: 1234,
                title: "测试窗口".to_string(),
                path: Some("C:\\test.exe".to_string()),
                hwnd: 42,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"foreground_changed\""));
        assert!(json.contains("\"pid\":1234"));
        assert!(json.contains("\"hwnd\":42"));
        assert!(json.contains("测试窗口"));
    }

    #[test]
    fn deserialize_ws_volume_and_window_requests() {
        let get_volume: WsClientRequest = serde_json::from_str(r#"{"type":"get_volume"}"#).unwrap();
        assert!(matches!(get_volume, WsClientRequest::GetVolume));

        let set_muted: WsClientRequest =
            serde_json::from_str(r#"{"type":"set_muted","muted":true}"#).unwrap();
        assert!(matches!(
            set_muted,
            WsClientRequest::SetMuted { muted: true }
        ));

        let focus: WsClientRequest =
            serde_json::from_str(r#"{"type":"focus_window","hwnd":12345}"#).unwrap();
        assert!(matches!(
            focus,
            WsClientRequest::FocusWindow { hwnd: 12345 }
        ));

        let switch: WsClientRequest =
            serde_json::from_str(r#"{"type":"switch_window","direction":"previous"}"#).unwrap();
        assert!(matches!(
            switch,
            WsClientRequest::SwitchWindow {
                direction: WindowSwitchDirection::Previous
            }
        ));
    }

    #[test]
    fn deserialize_display_and_remote_input_requests() {
        let display: WsClientRequest = serde_json::from_str(
            r#"{"type":"set_display_mode","display_id":"\\.\\DISPLAY1","width":1920,"height":1080,"refresh_millihz":60000}"#,
        )
        .unwrap();
        assert!(matches!(
            display,
            WsClientRequest::SetDisplayMode {
                width: 1920,
                height: 1080,
                ..
            }
        ));

        let mouse: WsClientRequest =
            serde_json::from_str(r#"{"type":"mouse_move","dx":4,"dy":-2}"#).unwrap();
        assert!(matches!(
            mouse,
            WsClientRequest::MouseMove { dx: 4, dy: -2 }
        ));

        let mouse_abs: WsClientRequest =
            serde_json::from_str(r#"{"type":"mouse_move_absolute","x":960,"y":540}"#).unwrap();
        assert!(matches!(
            mouse_abs,
            WsClientRequest::MouseMoveAbsolute { x: 960, y: 540 }
        ));

        let get_audio_outputs: WsClientRequest =
            serde_json::from_str(r#"{"type":"get_audio_outputs"}"#).unwrap();
        assert!(matches!(
            get_audio_outputs,
            WsClientRequest::GetAudioOutputs
        ));

        let set_audio_output: WsClientRequest =
            serde_json::from_str(r#"{"type":"set_audio_output","device_id":"audio-device-1"}"#)
                .unwrap();
        assert!(matches!(
            set_audio_output,
            WsClientRequest::SetAudioOutput { device_id } if device_id == "audio-device-1"
        ));

        let text: WsClientRequest =
            serde_json::from_str(r#"{"type":"send_text","text":"你好"}"#).unwrap();
        assert!(matches!(text, WsClientRequest::SendText { text } if text == "你好"));

        let key: WsClientRequest =
            serde_json::from_str(r#"{"type":"key_press","key":"backspace"}"#).unwrap();
        assert!(matches!(key, WsClientRequest::KeyPress { key } if key == "backspace"));

        let key_event: WsClientRequest =
            serde_json::from_str(r#"{"type":"key_event","usage":4,"modifiers":2,"pressed":true}"#)
                .unwrap();
        assert!(matches!(
            key_event,
            WsClientRequest::KeyEvent {
                usage: 4,
                modifiers: 2,
                pressed: true
            }
        ));

        let state_down: WsClientRequest =
            serde_json::from_str(r#"{"type":"mouse_button_state","button":"left","pressed":true}"#)
                .unwrap();
        assert!(matches!(
            state_down,
            WsClientRequest::MouseButtonState { button, pressed } if button == "left" && pressed
        ));

        let state_up: WsClientRequest = serde_json::from_str(
            r#"{"type":"mouse_button_state","button":"left","pressed":false}"#,
        )
        .unwrap();
        assert!(matches!(
            state_up,
            WsClientRequest::MouseButtonState { button, pressed } if button == "left" && !pressed
        ));
    }

    #[test]
    fn deserialize_auto_unlock_request() {
        let request: WsClientRequest = serde_json::from_str(r#"{"type":"auto_unlock"}"#).unwrap();
        assert!(matches!(request, WsClientRequest::AutoUnlock { pin: None }));
        let request: WsClientRequest =
            serde_json::from_str(r#"{"type":"auto_unlock","pin":"1234"}"#).unwrap();
        assert!(matches!(request, WsClientRequest::AutoUnlock { pin: Some(pin) } if pin == "1234"));
    }

    #[test]
    fn deserialize_manual_catalog_and_steam_install_requests() {
        let refresh: WsClientRequest =
            serde_json::from_str(r#"{"type":"refresh_launcher_catalog","kind":"game"}"#).unwrap();
        assert!(matches!(
            refresh,
            WsClientRequest::RefreshLauncherCatalog { kind } if kind == "game"
        ));

        let install: WsClientRequest =
            serde_json::from_str(r#"{"type":"install_steam_game","app_id":570}"#).unwrap();
        assert!(matches!(
            install,
            WsClientRequest::InstallSteamGame { app_id: 570 }
        ));
    }

    #[test]
    fn serialize_volume_and_unlock_status() {
        let volume = WsServerResponse::VolumeStatus(crate::protocol::responses::VolumeStatus {
            percent: 65,
            muted: false,
        });
        let json = serde_json::to_string(&volume).unwrap();
        assert!(json.contains("\"type\":\"volume_status\""));
        assert!(json.contains("\"percent\":65"));

        let outputs =
            WsServerResponse::AudioOutputs(crate::protocol::responses::AudioOutputsResponse {
                devices: vec![crate::protocol::responses::AudioOutputDevice {
                    id: "audio-device-1".into(),
                    name: "扬声器".into(),
                    is_default: true,
                }],
            });
        let json = serde_json::to_string(&outputs).unwrap();
        assert!(json.contains("\"type\":\"audio_outputs\""));
        assert!(json.contains("\"is_default\":true"));

        let unlock = WsServerResponse::UnlockStatus(crate::protocol::responses::UnlockStatus {
            locked: true,
            mode: "manual".to_string(),
            auto_login_enabled: false,
            message: "PC 已上线但仍需本机解锁".to_string(),
            uac_detected: false,
            surface: "login".to_string(),
        });
        let json = serde_json::to_string(&unlock).unwrap();
        assert!(json.contains("\"type\":\"unlock_status\""));
        assert!(json.contains("\"locked\":true"));
    }

    #[test]
    fn serialize_ws_event_process_started() {
        let event = WsEvent::ProcessStarted {
            data: ProcessBasic {
                pid: 5678,
                name: "test.exe".to_string(),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"process_started\""));
        assert!(json.contains("\"pid\":5678"));
        assert!(json.contains("test.exe"));
    }

    #[test]
    fn deserialize_mobile_launch_directory_requests() {
        let list: WsClientRequest = serde_json::from_str(
            r#"{"type":"list_launcher_items","kind":"game","query":"elden","page":1,"page_size":30}"#,
        )
        .unwrap();
        assert!(matches!(
            list,
            WsClientRequest::ListLauncherItems { kind, query: Some(query), page: 1, page_size: 30 }
                if kind == "game" && query == "elden"
        ));

        let process: WsClientRequest = serde_json::from_str(r#"{"type":"get_processes"}"#).unwrap();
        assert!(matches!(
            process,
            WsClientRequest::GetProcesses { filter, sort, page: 0, page_size: 50, .. }
                if filter == "user" && sort == "active"
        ));
    }

    #[test]
    fn serialize_ping_pong_with_request_id_shape() {
        let ping: WsClientRequest =
            serde_json::from_str(r#"{"type":"ping","request_id":"health-1"}"#).unwrap();
        assert!(matches!(ping, WsClientRequest::Ping));

        let pong = serde_json::to_string(&WsServerResponse::Pong).unwrap();
        assert_eq!(pong, r#"{"type":"pong"}"#);
    }
}

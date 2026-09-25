use super::*;

pub(super) fn handle(_state: &AppState, req: &WsClientRequest, _session_token: Option<&str>) -> WsServerMessage {
    match req {
        // === Volume & Media ===
        WsClientRequest::GetVolume => match crate::platform::get_volume() {
            Ok(status) => WsServerMessage::Response(WsServerResponse::VolumeStatus(status)),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::GetAudioOutputs => match crate::platform::get_audio_outputs() {
            Ok(outputs) => WsServerMessage::Response(WsServerResponse::AudioOutputs(outputs)),
            Err(e) => WsServerMessage::Response(WsServerResponse::error(
                "AUDIO_OUTPUTS_FAILED",
                &e.to_string(),
            )),
        },
        WsClientRequest::SetAudioOutput { device_id } => {
            match crate::platform::set_audio_output(device_id) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "AUDIO_OUTPUT_FAILED",
                    &e.to_string(),
                )),
            }
        }
        WsClientRequest::SetVolume { percent } => match crate::platform::set_volume(*percent) {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::SetMuted { muted } => match crate::platform::set_muted(*muted) {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::VolumeUp => match crate::platform::volume_up() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::VolumeDown => match crate::platform::volume_down() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::VolumeMute => match crate::platform::volume_mute() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("VOLUME_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::MediaPlayPause => match crate::platform::media_play_pause() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("MEDIA_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::MediaStop => match crate::platform::media_stop() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("MEDIA_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::MediaNext => match crate::platform::media_next() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("MEDIA_FAILED", &e.to_string()))
            }
        },
        WsClientRequest::MediaPrev => match crate::platform::media_prev() {
            Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
            Err(e) => {
                WsServerMessage::Response(WsServerResponse::error("MEDIA_FAILED", &e.to_string()))
            }
        },

        _ => unreachable!("request routed to the wrong handler"),
    }
}



use super::*;

pub(super) fn handle(
    state: &AppState,
    req: &WsClientRequest,
    _session_token: Option<&str>,
) -> WsServerMessage {
    match req {
        // === Steam ===
        WsClientRequest::GetSteamGames => {
            let games = state.app_scanner.get_steam_games();
            let libraries = state.app_scanner.get_steam_libraries();
            let steam_installed = state.app_scanner.is_steam_installed();
            let steam_path = state.app_scanner.get_steam_path();
            let total = games.len();
            WsServerMessage::Response(WsServerResponse::SteamGames(
                crate::protocol::responses::SteamGamesResponse {
                    steam_installed,
                    steam_path,
                    libraries,
                    games,
                    total,
                },
            ))
        }
        WsClientRequest::RefreshSteamGames => {
            if scanning_is_paused(state) {
                return WsServerMessage::Response(WsServerResponse::error(
                    "GAME_ACTIVE",
                    "检测到游戏运行，Steam 扫描已延后",
                ));
            }
            let _scan_permit = crate::lock_unpoisoned(&state.scan_gate);
            let games = state.app_scanner.refresh_steam();
            let entries = state.app_scanner.steam_entries(&games);
            let db = crate::lock_unpoisoned(&state.app_db);
            let result = db
                .sync_steam_games(&games)
                .and_then(|()| db.sync_scanned_source("Steam", &entries));
            drop(db);
            match result {
                Ok(()) => {
                    state.app_scanner.replace_source("Steam", entries);
                    WsServerMessage::Response(WsServerResponse::SteamRefresh {
                        scanned: games.len(),
                    })
                }
                Err(error) => {
                    WsServerMessage::Response(WsServerResponse::error("STEAM_SAVE_FAILED", &error))
                }
            }
        }
        WsClientRequest::InstallSteamGame { app_id } => {
            match crate::platform::install_steam_game(*app_id) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(error) => WsServerMessage::Response(WsServerResponse::error(
                    "STEAM_INSTALL_FAILED",
                    &error.to_string(),
                )),
            }
        }
        WsClientRequest::LaunchSteamGame { app_id } => {
            match crate::platform::launch_steam_game(*app_id) {
                Ok(()) => WsServerMessage::Response(WsServerResponse::success()),
                Err(e) => WsServerMessage::Response(WsServerResponse::error(
                    "LAUNCH_FAILED",
                    &e.to_string(),
                )),
            }
        }

        _ => super::unrouted_request(),
    }
}

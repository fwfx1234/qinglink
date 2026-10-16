//! 游戏启动显示模式协调、进程跟踪与异常退出恢复。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::protocol::responses::DisplayInfoResponse;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

use crate::service::app_scanner::AppEntry;

const SNAPSHOT_KEY: &str = "launch_display.snapshot";
const PROCESS_POLL_INTERVAL: Duration = Duration::from_secs(2);
const PROCESS_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DisplaySnapshot {
    display_id: String,
    width: u32,
    height: u32,
    refresh_millihz: u32,
    #[serde(default)]
    targets: Vec<PersistedTarget>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PersistedTarget {
    path: String,
    directory: bool,
}

impl LaunchTarget {
    fn persisted(&self) -> PersistedTarget {
        PersistedTarget {
            path: self.path.to_string_lossy().into_owned(),
            directory: self.directory,
        }
    }
}

#[derive(Debug)]
struct LaunchTarget {
    path: PathBuf,
    directory: bool,
    initial_pids: HashSet<u32>,
    tracked_pids: HashSet<u32>,
    started_at: Instant,
    discovered: bool,
}

#[derive(Debug)]
struct ActiveLaunch {
    snapshot: DisplaySnapshot,
    targets: Vec<LaunchTarget>,
}

/// 串行化游戏显示模式切换，并以首次配置游戏的模式管理并行游戏。
#[derive(Debug, Clone, Default)]
pub struct LaunchDisplayCoordinator {
    active: Arc<Mutex<Option<ActiveLaunch>>>,
}

impl LaunchDisplayCoordinator {
    /// 服务启动时恢复上次异常退出遗留的显示器快照。
    pub fn recover(&self, state: &crate::server::AppState) {
        let snapshot = crate::lock_unpoisoned(&state.app_db)
            .get_setting(SNAPSHOT_KEY)
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str::<DisplaySnapshot>(&value).ok());
        if let Some(snapshot) = snapshot {
            if !snapshot.targets.is_empty() {
                let targets = snapshot
                    .targets
                    .iter()
                    .map(|target| LaunchTarget {
                        path: PathBuf::from(&target.path),
                        directory: target.directory,
                        initial_pids: HashSet::new(),
                        tracked_pids: HashSet::new(),
                        started_at: Instant::now(),
                        discovered: false,
                    })
                    .collect();
                *self
                    .active
                    .lock()
                    .unwrap_or_else(|error| error.into_inner()) =
                    Some(ActiveLaunch { snapshot, targets });
                if let Err(error) = self.start_monitor(state.clone()) {
                    tracing::error!(%error, "恢复游戏进程监视线程失败");
                    if let Some(launch) = self
                        .active
                        .lock()
                        .unwrap_or_else(|value| value.into_inner())
                        .take()
                    {
                        if restore(&launch.snapshot).is_ok() {
                            clear_snapshot(state);
                        }
                    }
                }
                return;
            }
            match restore(&snapshot) {
                Ok(()) => {
                    clear_snapshot(state);
                }
                Err(error) => tracing::error!(%error, "恢复游戏启动前的显示模式失败"),
            }
        }
    }

    /// 按配置切换显示模式并启动游戏；切换失败不会阻止游戏启动。
    pub fn launch(&self, state: &crate::server::AppState, app: &AppEntry) -> anyhow::Result<()> {
        if app.launch_mode.is_none() || !app.category.eq_ignore_ascii_case("game") {
            return launch_app(app);
        }
        let mut active = self
            .active
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut system = process_system();
        refresh_processes(&mut system);
        let initial_pids = system.processes().keys().map(|pid| pid.as_u32()).collect();
        let target = launch_target(app, initial_pids);

        if let Some(session) = active.as_mut() {
            launch_app(app)?;
            session.snapshot.targets.push(target.persisted());
            session.targets.push(target);
            persist_snapshot(state, &session.snapshot);
            return Ok(());
        }

        let mode = app.launch_mode.as_ref().expect("launch mode checked above");
        let Some(mut snapshot) = primary_display_snapshot() else {
            tracing::warn!(app_id = %app.id, "无法读取主显示器，按原显示模式启动游戏");
            return launch_app(app);
        };
        snapshot.targets.push(target.persisted());
        let needs_change = !(snapshot.width == mode.width
            && snapshot.height == mode.height
            && snapshot.refresh_millihz == mode.refresh_millihz);

        let encoded = serde_json::to_string(&snapshot)?;
        if let Err(error) =
            crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, &encoded)
        {
            tracing::warn!(app_id = %app.id, %error, "无法保存显示模式恢复快照，按原模式启动");
            return launch_app(app);
        }
        if needs_change {
            if let Err(error) = crate::platform::set_display_mode(
                &snapshot.display_id,
                mode.width,
                mode.height,
                mode.refresh_millihz,
            ) {
                tracing::warn!(app_id = %app.id, %error, "切换游戏显示模式失败，继续启动");
                let _ = crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, "");
                return launch_app(app);
            }
        }
        if let Err(error) = launch_app(app) {
            match restore(&snapshot) {
                Ok(()) => {
                    let _ = crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, "");
                }
                Err(restore_error) => {
                    tracing::error!(%restore_error, "游戏启动失败后恢复显示模式失败");
                }
            }
            return Err(error);
        }

        *active = Some(ActiveLaunch {
            snapshot,
            targets: vec![target],
        });
        drop(active);
        if let Err(error) = self.start_monitor(state.clone()) {
            tracing::error!(%error, "启动游戏进程监视线程失败");
            let mut active = self
                .active
                .lock()
                .unwrap_or_else(|value| value.into_inner());
            if let Some(launch) = active.take() {
                match restore(&launch.snapshot) {
                    Ok(()) => {
                        let _ = crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, "");
                    }
                    Err(restore_error) => {
                        tracing::error!(%restore_error, "启动监视线程失败后恢复显示模式失败");
                    }
                }
            }
        }
        Ok(())
    }

    fn start_monitor(&self, state: crate::server::AppState) -> std::io::Result<()> {
        let active = self.active.clone();
        std::thread::Builder::new()
            .name("qinglink-game-display-monitor".to_string())
            .spawn(move || monitor_processes(active, state))?;
        Ok(())
    }
}

fn primary_display_snapshot() -> Option<DisplaySnapshot> {
    crate::platform::get_displays()
        .ok()?
        .into_iter()
        .find(|display| display.primary)
        .map(|display: DisplayInfoResponse| DisplaySnapshot {
            display_id: display.id,
            width: display.current_mode.width,
            height: display.current_mode.height,
            refresh_millihz: display.current_mode.refresh_millihz,
            targets: Vec::new(),
        })
}

fn persist_snapshot(state: &crate::server::AppState, snapshot: &DisplaySnapshot) {
    match serde_json::to_string(snapshot) {
        Ok(value) => {
            if let Err(error) =
                crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, &value)
            {
                tracing::error!(%error, "更新游戏显示模式恢复快照失败");
            }
        }
        Err(error) => tracing::error!(%error, "序列化游戏显示模式恢复快照失败"),
    }
}

fn clear_snapshot(state: &crate::server::AppState) {
    if let Err(error) = crate::lock_unpoisoned(&state.app_db).set_setting(SNAPSHOT_KEY, "") {
        tracing::error!(%error, "清除游戏显示模式恢复快照失败");
    }
}

fn restore(snapshot: &DisplaySnapshot) -> anyhow::Result<()> {
    crate::platform::set_display_mode(
        &snapshot.display_id,
        snapshot.width,
        snapshot.height,
        snapshot.refresh_millihz,
    )
}

fn launch_app(app: &AppEntry) -> anyhow::Result<()> {
    if app.source.eq_ignore_ascii_case("steam")
        && let Some(app_id) = app.id.strip_prefix("steam:").and_then(|id| id.parse().ok())
    {
        crate::platform::launch_steam_game(app_id)
    } else {
        crate::platform::launch_app(&app.exe_path, &app.args)
    }
}

fn launch_target(app: &AppEntry, initial_pids: HashSet<u32>) -> LaunchTarget {
    let path = PathBuf::from(&app.exe_path);
    let directory = app.source.eq_ignore_ascii_case("steam") || !path.is_file();
    let path = if directory {
        if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(&path).to_path_buf()
        }
    } else {
        path
    };
    LaunchTarget {
        path,
        directory,
        initial_pids,
        tracked_pids: HashSet::new(),
        started_at: Instant::now(),
        discovered: false,
    }
}

fn monitor_processes(active: Arc<Mutex<Option<ActiveLaunch>>>, state: crate::server::AppState) {
    let mut system = process_system();
    loop {
        std::thread::sleep(PROCESS_POLL_INTERVAL);
        refresh_processes(&mut system);
        let processes = system.processes();
        let mut session = active.lock().unwrap_or_else(|error| error.into_inner());
        let Some(launch) = session.as_mut() else {
            return;
        };
        for target in &mut launch.targets {
            track_target(target, processes);
        }
        let previous_count = launch.targets.len();
        launch.targets.retain(|target| {
            if target.discovered {
                !target.tracked_pids.is_empty()
            } else {
                target.started_at.elapsed() < PROCESS_DISCOVERY_TIMEOUT
            }
        });
        if launch.targets.len() != previous_count {
            launch.snapshot.targets = launch.targets.iter().map(LaunchTarget::persisted).collect();
            persist_snapshot(&state, &launch.snapshot);
        }
        if !launch.targets.is_empty() {
            continue;
        }
        let snapshot = launch.snapshot.clone();
        *session = None;
        drop(session);
        match restore(&snapshot) {
            Ok(()) => clear_snapshot(&state),
            Err(error) => tracing::error!(%error, "游戏退出后恢复显示模式失败"),
        }
        return;
    }
}

fn process_system() -> System {
    System::new_with_specifics(RefreshKind::new().with_processes(process_refresh_kind()))
}

fn refresh_processes(system: &mut System) {
    system.refresh_processes_specifics(ProcessesToUpdate::All, process_refresh_kind());
}

fn process_refresh_kind() -> ProcessRefreshKind {
    ProcessRefreshKind::new().with_exe(UpdateKind::Always)
}

fn track_target(target: &mut LaunchTarget, processes: &HashMap<Pid, sysinfo::Process>) {
    let matching: Vec<u32> = processes
        .iter()
        .filter_map(|(pid, process)| {
            let pid_value = pid.as_u32();
            let path = process.exe()?;
            (!target.initial_pids.contains(&pid_value) && path_matches(target, path))
                .then_some(pid_value)
        })
        .collect();
    target.tracked_pids.extend(matching);
    target.discovered |= !target.tracked_pids.is_empty();
    let tracked = target.tracked_pids.clone();
    for (pid, process) in processes {
        if process
            .parent()
            .is_some_and(|parent| tracked.contains(&parent.as_u32()))
        {
            target.tracked_pids.insert(pid.as_u32());
        }
    }
    target.tracked_pids.retain(|pid| {
        processes
            .keys()
            .any(|process_pid| process_pid.as_u32() == *pid)
    });
}

fn path_matches(target: &LaunchTarget, process_path: &Path) -> bool {
    if target.directory {
        let mut directory = target
            .path
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        if !directory.ends_with('\\') {
            directory.push('\\');
        }
        process_path
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase()
            .starts_with(&directory)
    } else {
        process_path
            .to_string_lossy()
            .eq_ignore_ascii_case(&target.path.to_string_lossy())
    }
}

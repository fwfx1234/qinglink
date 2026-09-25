pub mod health;
pub mod pairing;
pub mod platform;
pub mod service_ipc;

pub use health::HealthResponse;
pub use pairing::{
    PairingConfirm, PairingRequest, PairingRequestResponse, PairingTokenResponse,
    WakeNetworkAdapter,
};
pub use platform::{
    AppInfo, AudioOutputDevice, AudioOutputsResponse, DisplayInfoResponse, DisplayModeResponse,
    ForegroundResponse, LockInputResult, LockInputStatus, MAX_MOUSE_MOVE_DELTA,
    SecureDesktopStatus, UnlockStatus, VolumeStatus, WindowSwitchDirection,
};

pub const API_VERSION: u8 = 2;
pub const DEFAULT_PORT: u16 = 3721;
pub const DISCOVERY_PORT: u16 = 3723;
pub const DISCOVERY_MAGIC: &str = "qinglink-discover-v1";

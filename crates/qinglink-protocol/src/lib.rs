pub mod health;
pub mod ir;
pub mod pairing;
pub mod platform;
pub mod service_ipc;

pub use health::HealthResponse;
pub use ir::{
    IR_DEFAULT_STEP_DELAY_MS, IR_MAX_MACRO_STEPS, IR_MAX_PATTERN_DURATION_US, IR_MAX_PATTERN_ITEMS,
    IR_MAX_REPEAT_COUNT, IR_MAX_STEP_DELAY_MS, IrCommand, IrMacro, IrMacroStep, IrRemoteProfile,
    IrValidationError,
};
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

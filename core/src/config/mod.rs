mod flash_region;
mod manager;
mod store;
mod load;
pub mod storage;
pub mod types;
pub mod validate;
pub mod write_session;

pub use manager::ConfigManager;
pub use types::{
    CURRENT_CONFIG_VERSION, CURRENT_STORAGE_VERSION, ConfigError, DeviceConfig, MAX_PAYLOAD_SIZE,
    SLOT_A_INDEX, SLOT_B_INDEX, SLOT_BUF_SIZE, SLOT_COUNT, SLOT_FLAGS_NONE, SLOT_MAGIC,
    SaveOutcome, SlotHeader,
};
pub use validate::validate_config;

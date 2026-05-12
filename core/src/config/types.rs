use crate::motion::config::MotionConfig;
use crate::power::PowerConfig;
use crate::report::config::ReportConfig;
use serde::{Deserialize, Serialize};

pub const CURRENT_STORAGE_VERSION: u16 = 1;
pub const CURRENT_CONFIG_VERSION: u16 = 1;
pub const SLOT_COUNT: usize = 2;
pub const SLOT_A_INDEX: usize = 0;
pub const SLOT_B_INDEX: usize = 1;
pub const SLOT_MAGIC: u32 = 0x4746_4356;
pub const SLOT_FLAGS_NONE: u32 = 0;

/// 编译期最大 slot buffer，平台层可能报告更小的 slot_size
pub const SLOT_BUF_SIZE: usize = 4096;

/// 编译期最大 payload = SLOT_BUF_SIZE 减去固定 header 开销
pub const MAX_PAYLOAD_SIZE: usize = SLOT_BUF_SIZE - SlotHeader::ENCODED_LEN;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub power: PowerConfig,
    pub motion: MotionConfig,
    pub report: ReportConfig,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            power: PowerConfig::default(),
            motion: MotionConfig::default(),
            report: ReportConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotHeader {
    pub magic: u32,
    pub storage_version: u16,
    pub config_version: u16,
    pub payload_len: u32,
    pub sequence: u32,
    pub payload_crc32: u32,
    pub header_crc32: u32,
    pub flags: u32,
}

impl SlotHeader {
    pub const ENCODED_LEN: usize = 28;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    EncodeFailed,
    PayloadTooLarge,
    StorageEmpty,
    DeserializeFailed,
    ValidationFailed,
    StorageUnavailable,
    InvalidFlashRegion,
    HeaderCrcMismatch,
    PayloadCrcMismatch,
    InvalidMagic,
    UnsupportedStorageVersion,
    UnsupportedConfigVersion,
    InvalidPayloadLength,
    FlashEraseFailed,
    FlashWriteFailed,
    ReadbackVerifyFailed,
    MigrationFailed,
    WriteSessionBusy,
    WriteSessionNotActive,
    WriteSequenceMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Noop,
    Saved,
}

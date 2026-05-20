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
pub const SLOT_BUF_SIZE: usize = 4096;
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

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::config::load::parse_bytes;

    #[test]
    fn slot_header_encoded_len() {
        assert_eq!(SlotHeader::ENCODED_LEN, 28);
    }

    #[test]
    fn default_device_config_fields() {
        let c = DeviceConfig::default();
        assert!(c.report.report_hz > 0.0);
        assert!(c.power.suspend_timeout_ms > 0);
    }

    #[test]
    fn serde_roundtrip() {
        let c = DeviceConfig::default();
        let mut buf = [0u8; 256];
        let encoded = postcard::to_slice(&c, &mut buf).unwrap();
        let decoded: DeviceConfig = parse_bytes(encoded).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn serde_non_default() {
        let mut c = DeviceConfig::default();
        c.motion.sensitivity_x = 24000.0;
        c.motion.invert_y = true;
        c.power.suspend_timeout_ms = 30000;

        let mut buf = [0u8; 256];
        let encoded = postcard::to_slice(&c, &mut buf).unwrap();
        let decoded: DeviceConfig = parse_bytes(encoded).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn max_payload_size_positive() {
        assert!(MAX_PAYLOAD_SIZE > 0);
        assert!(MAX_PAYLOAD_SIZE >= SLOT_BUF_SIZE - SlotHeader::ENCODED_LEN);
    }
}

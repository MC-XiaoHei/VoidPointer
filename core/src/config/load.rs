use serde::Deserialize;

use crate::config::flash_region::FlashRegionInfo;
use crate::config::migration::migrate_payload;
use crate::config::storage::{compute_header_crc32, crc32, slot_header_decode};
use crate::config::types::{
    CURRENT_CONFIG_VERSION, CURRENT_STORAGE_VERSION, ConfigError, DeviceConfig, SLOT_A_INDEX,
    SLOT_B_INDEX, SLOT_BUF_SIZE, SLOT_MAGIC, SlotHeader,
};
use crate::config::validate::validate_config;
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_flash_read};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActiveSlot {
    A,
    B,
}

impl ActiveSlot {
    pub(crate) fn from_index(index: usize) -> Option<Self> {
        match index {
            SLOT_A_INDEX => Some(Self::A),
            SLOT_B_INDEX => Some(Self::B),
            _ => None,
        }
    }

    pub(crate) fn inactive_index(self) -> usize {
        match self {
            Self::A => SLOT_B_INDEX,
            Self::B => SLOT_A_INDEX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PersistedConfig {
    pub(crate) active_slot: ActiveSlot,
    pub(crate) next_sequence: u32,
    pub(crate) config: DeviceConfig,
    pub(crate) was_migrated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ValidSlot {
    index: usize,
    header: SlotHeader,
    config: DeviceConfig,
    was_migrated: bool,
}

pub(crate) fn parse_bytes<'a, T>(payload: &'a [u8]) -> Result<T, ConfigError>
where
    T: Deserialize<'a>,
{
    postcard::from_bytes(payload).map_err(|_| ConfigError::DeserializeFailed)
}

#[cfg_attr(coverage, coverage(off))]
pub(crate) fn load_persisted_config(
    flash: FlashRegionInfo,
    slot_size: u32,
    slot_buf: &mut [u8; SLOT_BUF_SIZE],
) -> Result<PersistedConfig, ConfigError> {
    let slot_a = read_and_validate_slot(flash, slot_size, slot_buf, SLOT_A_INDEX).ok();
    let slot_b = read_and_validate_slot(flash, slot_size, slot_buf, SLOT_B_INDEX).ok();
    select_persisted_config(slot_a, slot_b)
}

fn select_persisted_config(
    slot_a: Option<ValidSlot>,
    slot_b: Option<ValidSlot>,
) -> Result<PersistedConfig, ConfigError> {
    let selected = pick_active_slot(slot_a, slot_b).ok_or(ConfigError::StorageEmpty)?;
    Ok(PersistedConfig {
        active_slot: ActiveSlot::from_index(selected.index)
            .ok_or(ConfigError::InvalidFlashRegion)?,
        next_sequence: selected.header.sequence.wrapping_add(1),
        config: selected.config,
        was_migrated: selected.was_migrated,
    })
}

#[cfg_attr(coverage, coverage(off))]
fn read_and_validate_slot(
    flash: FlashRegionInfo,
    slot_size: u32,
    slot_buf: &mut [u8; SLOT_BUF_SIZE],
    slot_index: usize,
) -> Result<ValidSlot, ConfigError> {
    let offset = flash.slot_offset(slot_size, slot_index)?;
    if !flash_read(offset, slot_size, slot_buf) {
        return Err(ConfigError::ReadbackVerifyFailed);
    }
    validate_slot_data(slot_buf, slot_size, slot_index)
}

#[cfg_attr(coverage, coverage(off))]
fn flash_read(offset: u32, slot_size: u32, slot_buf: &mut [u8; SLOT_BUF_SIZE]) -> bool {
    let status = unsafe { c_vp_flash_read(offset, slot_buf.as_mut_ptr(), slot_size) };
    status == VP_STATUS_OK as u8
}

fn validate_slot_data(
    slot_buf: &[u8; SLOT_BUF_SIZE],
    slot_size: u32,
    slot_index: usize,
) -> Result<ValidSlot, ConfigError> {
    let slot = &slot_buf[..slot_size as usize];
    let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
    header_bytes.copy_from_slice(&slot[..SlotHeader::ENCODED_LEN]);

    if header_bytes.iter().all(|b| *b == 0xFF) {
        return Err(ConfigError::StorageEmpty);
    }

    let header = slot_header_decode(&header_bytes);
    validate_slot_header(header, slot_size)?;

    let payload_len = header.payload_len as usize;
    let payload = &slot[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload_len];
    if crc32(payload) != header.payload_crc32 {
        return Err(ConfigError::PayloadCrcMismatch);
    }

    let (config, was_migrated) = if header.config_version == CURRENT_CONFIG_VERSION {
        let config: DeviceConfig = parse_bytes(payload)?;
        (config, false)
    } else {
        let config = migrate_payload(payload, header.config_version)?;
        (config, true)
    };

    validate_config(&config)?;

    Ok(ValidSlot {
        index: slot_index,
        header,
        config,
        was_migrated,
    })
}

fn validate_slot_header(header: SlotHeader, slot_size: u32) -> Result<(), ConfigError> {
    if header.magic != SLOT_MAGIC {
        return Err(ConfigError::InvalidMagic);
    }
    if header.storage_version != CURRENT_STORAGE_VERSION {
        return Err(ConfigError::UnsupportedStorageVersion);
    }
    if header.payload_len == 0 {
        return Err(ConfigError::InvalidPayloadLength);
    }
    let slot_payload_capacity = (slot_size as usize).saturating_sub(SlotHeader::ENCODED_LEN);
    if header.payload_len as usize > slot_payload_capacity {
        return Err(ConfigError::InvalidPayloadLength);
    }
    if compute_header_crc32(header) != header.header_crc32 {
        return Err(ConfigError::HeaderCrcMismatch);
    }
    Ok(())
}

fn pick_active_slot(slot_a: Option<ValidSlot>, slot_b: Option<ValidSlot>) -> Option<ValidSlot> {
    match (slot_a, slot_b) {
        (Some(a), Some(b)) => {
            if b.header.sequence > a.header.sequence {
                Some(b)
            } else {
                Some(a)
            }
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    fn make_valid_slot(index: usize, sequence: u32) -> ValidSlot {
        ValidSlot {
            index,
            header: SlotHeader {
                magic: SLOT_MAGIC,
                storage_version: CURRENT_STORAGE_VERSION,
                config_version: CURRENT_CONFIG_VERSION,
                payload_len: 10,
                sequence,
                payload_crc32: 0,
                header_crc32: 0,
                flags: 0,
            },
            config: DeviceConfig::default(),
            was_migrated: false,
        }
    }

    #[test]
    fn pick_both_none() {
        assert!(pick_active_slot(None, None).is_none());
    }

    #[test]
    fn pick_only_a() {
        assert_eq!(
            pick_active_slot(Some(make_valid_slot(0, 1)), None)
                .unwrap()
                .index,
            0
        );
    }

    #[test]
    fn pick_b_wins_by_higher_sequence() {
        let a = make_valid_slot(0, 10);
        let b = make_valid_slot(1, 20);
        assert_eq!(pick_active_slot(Some(a), Some(b)).unwrap().index, 1);
    }

    #[test]
    fn pick_a_wins_by_higher_sequence() {
        let a = make_valid_slot(0, 30);
        let b = make_valid_slot(1, 20);
        assert_eq!(pick_active_slot(Some(a), Some(b)).unwrap().index, 0);
    }

    #[test]
    fn pick_only_b() {
        assert_eq!(
            pick_active_slot(None, Some(make_valid_slot(1, 5)))
                .unwrap()
                .index,
            1
        );
    }

    #[test]
    fn pick_equal_sequence_prefers_a() {
        let a = make_valid_slot(0, 42);
        let b = make_valid_slot(1, 42);
        assert_eq!(pick_active_slot(Some(a), Some(b)).unwrap().index, 0);
    }

    #[test]
    fn validate_header_seal_passes() {
        let h = SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 30,
            sequence: 5,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        let sealed = crate::config::storage::seal_header(h);
        assert!(validate_slot_header(sealed, 4096).is_ok());
    }

    #[test]
    fn validate_header_bad_magic() {
        let h = SlotHeader {
            magic: 0xDEAD_BEEF,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 10,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        assert_eq!(
            validate_slot_header(h, 4096),
            Err(ConfigError::InvalidMagic)
        );
    }

    #[test]
    fn validate_header_zero_len() {
        let h = SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 0,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        assert_eq!(
            validate_slot_header(h, 4096),
            Err(ConfigError::InvalidPayloadLength)
        );
    }

    #[test]
    fn validate_header_oversize_payload() {
        let h = SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 5000,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        assert_eq!(
            validate_slot_header(h, 4096),
            Err(ConfigError::InvalidPayloadLength)
        );
    }

    #[test]
    fn validate_header_unsupported_storage_version() {
        let h = SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: 99,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 10,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        assert_eq!(
            validate_slot_header(h, 4096),
            Err(ConfigError::UnsupportedStorageVersion)
        );
    }

    #[test]
    fn validate_header_crc_mismatch() {
        let h = SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 10,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0xDEAD_BEEF,
            flags: 0,
        };
        assert_eq!(
            validate_slot_header(h, 4096),
            Err(ConfigError::HeaderCrcMismatch)
        );
    }

    #[test]
    fn active_slot_from_index_a() {
        assert_eq!(ActiveSlot::from_index(SLOT_A_INDEX), Some(ActiveSlot::A));
    }

    #[test]
    fn active_slot_from_index_b() {
        assert_eq!(ActiveSlot::from_index(SLOT_B_INDEX), Some(ActiveSlot::B));
    }

    #[test]
    fn active_slot_from_index_out_of_range() {
        assert_eq!(ActiveSlot::from_index(99), None);
    }

    #[test]
    fn active_slot_inactive_a_returns_b() {
        assert_eq!(ActiveSlot::A.inactive_index(), SLOT_B_INDEX);
    }

    #[test]
    fn active_slot_inactive_b_returns_a() {
        assert_eq!(ActiveSlot::B.inactive_index(), SLOT_A_INDEX);
    }

    #[test]
    fn validate_slot_data_bad_magic() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        let h = SlotHeader {
            magic: 0xDEAD_BEEF,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: 10,
            sequence: 1,
            payload_crc32: 0,
            header_crc32: 0,
            flags: 0,
        };
        let sealed = crate::config::storage::seal_header(h);
        crate::config::storage::slot_header_encode(sealed, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::InvalidMagic)
        );
    }

    #[test]
    fn validate_slot_data_empty_returns_storage_empty() {
        let slot_buf = [0xFFu8; SLOT_BUF_SIZE];
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::StorageEmpty)
        );
    }

    #[test]
    fn validate_slot_data_valid_slot() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let config = DeviceConfig::default();
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&config, &mut payload_buf).unwrap();
        let payload_crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: payload_crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        let result = validate_slot_data(&slot_buf, 4096, 0).unwrap();
        assert_eq!(result.index, 0);
        assert_eq!(result.header, header);
        assert_eq!(result.config, DeviceConfig::default());
    }

    #[test]
    fn validate_slot_data_crc_mismatch() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let config = DeviceConfig::default();
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&config, &mut payload_buf).unwrap();
        let payload_crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: payload_crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        slot_buf[SlotHeader::ENCODED_LEN] ^= 0xFF;
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::PayloadCrcMismatch)
        );
    }

    #[test]
    fn validate_slot_data_unsupported_config_version() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let config = DeviceConfig::default();
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&config, &mut payload_buf).unwrap();
        let payload_crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION + 1,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: payload_crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::UnsupportedConfigVersion)
        );
    }

    #[test]
    fn validate_slot_data_deserialize_failed() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let garbage = [0xDEu8; 10];
        let crc = crc32(&garbage);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: garbage.len() as u32,
            sequence: 1,
            payload_crc32: crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + garbage.len()]
            .copy_from_slice(&garbage);
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::DeserializeFailed)
        );
    }

    #[test]
    fn validate_slot_data_validation_failed() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let mut invalid = DeviceConfig::default();
        invalid.report.report_hz = 0.0;
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&invalid, &mut payload_buf).unwrap();
        let crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        assert_eq!(
            validate_slot_data(&slot_buf, 4096, 0),
            Err(ConfigError::ValidationFailed)
        );
    }

    #[test]
    fn select_persisted_config_both_none() {
        assert_eq!(
            select_persisted_config(None, None),
            Err(ConfigError::StorageEmpty)
        );
    }

    #[test]
    fn select_persisted_config_chooses_a() {
        let a = make_valid_slot(0, 10);
        let result = select_persisted_config(Some(a), None).unwrap();
        assert_eq!(result.active_slot, ActiveSlot::A);
        assert_eq!(result.next_sequence, 11);
    }

    #[test]
    fn select_persisted_config_chooses_b() {
        let b = make_valid_slot(1, 20);
        let result = select_persisted_config(None, Some(b)).unwrap();
        assert_eq!(result.active_slot, ActiveSlot::B);
        assert_eq!(result.next_sequence, 21);
    }

    #[test]
    fn select_persisted_config_invalid_index() {
        let bad = make_valid_slot(99, 1);
        assert_eq!(
            select_persisted_config(Some(bad), None),
            Err(ConfigError::InvalidFlashRegion)
        );
    }

    #[test]
    fn validate_slot_data_same_version_not_migrated() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let config = DeviceConfig::default();
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&config, &mut payload_buf).unwrap();
        let payload_crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: payload_crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        let result = validate_slot_data(&slot_buf, 4096, 0).unwrap();
        assert!(!result.was_migrated, "同版本不应标记为 was_migrated");
    }

    #[test]
    fn validate_slot_data_migration_fails_for_future_version() {
        let mut slot_buf = [0u8; SLOT_BUF_SIZE];
        let config = DeviceConfig::default();
        let mut payload_buf = [0u8; 256];
        let payload = postcard::to_slice(&config, &mut payload_buf).unwrap();
        let payload_crc = crc32(payload);
        let header = crate::config::storage::seal_header(SlotHeader {
            magic: SLOT_MAGIC,
            storage_version: CURRENT_STORAGE_VERSION,
            config_version: CURRENT_CONFIG_VERSION + 1,
            payload_len: payload.len() as u32,
            sequence: 1,
            payload_crc32: payload_crc,
            header_crc32: 0,
            flags: 0,
        });
        let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
        crate::config::storage::slot_header_encode(header, &mut header_bytes);
        slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
        slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
            .copy_from_slice(payload);
        let err = validate_slot_data(&slot_buf, 4096, 0).unwrap_err();
        assert_eq!(err, ConfigError::UnsupportedConfigVersion);
    }
}

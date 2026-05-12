use crate::config::flash_region::FlashRegionInfo;
use crate::config::storage::{compute_header_crc32, crc32, slot_header_decode};
use crate::config::types::{
    CURRENT_CONFIG_VERSION, CURRENT_STORAGE_VERSION, ConfigError, DeviceConfig, SLOT_A_INDEX,
    SLOT_B_INDEX, SLOT_BUF_SIZE, SLOT_MAGIC, SlotHeader,
};
use crate::config::validate::validate_config;
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_flash_read};

/// 当前哪个 slot 被选为活动槽
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

    /// 返回当前活动槽的"另一槽"索引，用于保存目标选择
    pub(crate) fn inactive_index(self) -> usize {
        match self {
            Self::A => SLOT_B_INDEX,
            Self::B => SLOT_A_INDEX,
        }
    }
}

/// 从 flash 成功加载的持久化配置
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PersistedConfig {
    pub(crate) active_slot: ActiveSlot,
    pub(crate) next_sequence: u32,
    pub(crate) config: DeviceConfig,
}

/// 通过完整验证链的有效 slot（验证顺序参见 CONFIG_SPEC.md §8）
#[derive(Clone, Copy, Debug, PartialEq)]
struct ValidSlot {
    index: usize,
    header: SlotHeader,
    config: DeviceConfig,
}

/// 扫描双 slot，选择有效的最新配置加载
pub(crate) fn load_persisted_config(
    flash: FlashRegionInfo,
    slot_size: u32,
    slot_buf: &mut [u8; SLOT_BUF_SIZE],
) -> Result<PersistedConfig, ConfigError> {
    let slot_a = read_and_validate_slot(flash, slot_size, slot_buf, SLOT_A_INDEX).ok();
    let slot_b = read_and_validate_slot(flash, slot_size, slot_buf, SLOT_B_INDEX).ok();
    let selected = pick_active_slot(slot_a, slot_b).ok_or(ConfigError::StorageEmpty)?;

    Ok(PersistedConfig {
        active_slot: ActiveSlot::from_index(selected.index)
            .ok_or(ConfigError::InvalidFlashRegion)?,
        next_sequence: selected.header.sequence.wrapping_add(1),
        config: selected.config,
    })
}

/// 从 flash 读取一个 slot 并走完完整验证链
fn read_and_validate_slot(
    flash: FlashRegionInfo,
    slot_size: u32,
    slot_buf: &mut [u8; SLOT_BUF_SIZE],
    slot_index: usize,
) -> Result<ValidSlot, ConfigError> {
    let offset = flash.slot_offset(slot_size, slot_index)?;
    if unsafe { c_vp_flash_read(offset, slot_buf.as_mut_ptr(), slot_size) } != VP_STATUS_OK as u8 {
        return Err(ConfigError::ReadbackVerifyFailed);
    }

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

    if header.config_version != CURRENT_CONFIG_VERSION {
        return Err(ConfigError::UnsupportedConfigVersion);
    }

    let config: DeviceConfig =
        postcard::from_bytes(payload).map_err(|_| ConfigError::DeserializeFailed)?;
    validate_config(&config)?;

    Ok(ValidSlot {
        index: slot_index,
        header,
        config,
    })
}

/// 校验 SlotHeader 中的元信息字段（magic / version / payload_len / header_crc32）
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

/// 选择活跃 slot：单有效则用，双有效取 sequence 更大者，相等选 A
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

    fn make_slot(index: usize, sequence: u32) -> ValidSlot {
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
        }
    }

    #[test]
    fn pick_both_none() {
        assert!(pick_active_slot(None, None).is_none());
    }

    #[test]
    fn pick_only_a() {
        assert_eq!(
            pick_active_slot(Some(make_slot(0, 1)), None).unwrap().index,
            0
        );
    }

    #[test]
    fn pick_b_wins_by_higher_sequence() {
        let a = make_slot(0, 10);
        let b = make_slot(1, 20);
        assert_eq!(pick_active_slot(Some(a), Some(b)).unwrap().index, 1);
    }

    #[test]
    fn pick_a_wins_by_higher_sequence() {
        let a = make_slot(0, 30);
        let b = make_slot(1, 20);
        assert_eq!(pick_active_slot(Some(a), Some(b)).unwrap().index, 0);
    }

    #[test]
    fn pick_only_b() {
        assert_eq!(
            pick_active_slot(None, Some(make_slot(1, 5))).unwrap().index,
            1
        );
    }

    #[test]
    fn pick_equal_sequence_prefers_a() {
        let a = make_slot(0, 42);
        let b = make_slot(1, 42);
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
}

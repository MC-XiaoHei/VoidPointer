use crate::config::flash_region::FlashRegionInfo;
use crate::config::load::ActiveSlot;
use crate::config::storage::{crc32, seal_header, slot_header_encode};
use crate::config::types::{
    CURRENT_CONFIG_VERSION, CURRENT_STORAGE_VERSION, ConfigError, SLOT_A_INDEX, SLOT_BUF_SIZE,
    SLOT_FLAGS_NONE, SLOT_MAGIC, SlotHeader,
};
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_flash_erase, c_vp_flash_read, c_vp_flash_write};

/// 将 payload 写入非活跃 slot，含擦除、写入、回读验证
pub(crate) fn save_persisted_config(
    flash: FlashRegionInfo,
    slot_size: u32,
    active_slot: Option<ActiveSlot>,
    next_sequence: u32,
    payload: &[u8],
    payload_crc32: u32,
    slot_buf: &mut [u8; SLOT_BUF_SIZE],
) -> Result<ActiveSlot, ConfigError> {
    let target_index = active_slot
        .map(ActiveSlot::inactive_index)
        .unwrap_or(SLOT_A_INDEX);
    let target_active =
        ActiveSlot::from_index(target_index).ok_or(ConfigError::InvalidFlashRegion)?;
    let offset = flash.slot_offset(slot_size, target_index)?;

    let header = seal_header(SlotHeader {
        magic: SLOT_MAGIC,
        storage_version: CURRENT_STORAGE_VERSION,
        config_version: CURRENT_CONFIG_VERSION,
        payload_len: payload.len() as u32,
        sequence: next_sequence,
        payload_crc32,
        header_crc32: 0,
        flags: SLOT_FLAGS_NONE,
    });

    let slot_len = slot_size as usize;
    let mut header_bytes = [0u8; SlotHeader::ENCODED_LEN];
    slot_header_encode(header, &mut header_bytes);
    slot_buf[..slot_len].fill(0xFF);
    slot_buf[..SlotHeader::ENCODED_LEN].copy_from_slice(&header_bytes);
    slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()]
        .copy_from_slice(payload);

    if unsafe { c_vp_flash_erase(offset, slot_size) } != VP_STATUS_OK as u8 {
        return Err(ConfigError::FlashEraseFailed);
    }

    if unsafe { c_vp_flash_write(offset, slot_buf.as_ptr(), slot_size) } != VP_STATUS_OK as u8 {
        return Err(ConfigError::FlashWriteFailed);
    }

    // 回读验证：header 完全一致，payload CRC 一致
    if unsafe { c_vp_flash_read(offset, slot_buf.as_mut_ptr(), slot_size) } != VP_STATUS_OK as u8 {
        return Err(ConfigError::ReadbackVerifyFailed);
    }

    if slot_buf[..SlotHeader::ENCODED_LEN] != header_bytes {
        return Err(ConfigError::ReadbackVerifyFailed);
    }
    if crc32(&slot_buf[SlotHeader::ENCODED_LEN..SlotHeader::ENCODED_LEN + payload.len()])
        != payload_crc32
    {
        return Err(ConfigError::ReadbackVerifyFailed);
    }

    Ok(target_active)
}

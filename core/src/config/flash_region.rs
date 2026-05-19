use crate::config::types::{ConfigError, SLOT_COUNT};
use crate::ffi::bindings::{VP_STATUS_OK, c_vp_flash_config_region, vp_flash_region_t};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FlashRegionInfo {
    pub(crate) offset: u32,
    pub(crate) length: u32,
    pub(crate) page_size: u32,
    pub(crate) write_alignment: u32,
}

impl FlashRegionInfo {
    pub(crate) fn slot_offset(
        &self,
        slot_size: u32,
        slot_index: usize,
    ) -> Result<u32, ConfigError> {
        if slot_index >= SLOT_COUNT {
            return Err(ConfigError::InvalidFlashRegion);
        }
        Ok(self.offset + slot_size * slot_index as u32)
    }
}

pub(crate) fn get_flash_region() -> Result<FlashRegionInfo, ConfigError> {
    let mut region = vp_flash_region_t {
        offset: 0,
        length: 0,
        page_size: 0,
        write_alignment: 0,
    };

    if unsafe { c_vp_flash_config_region(&mut region) } != VP_STATUS_OK as u8 {
        return Err(ConfigError::StorageUnavailable);
    }

    if region.length == 0
        || region.length % SLOT_COUNT as u32 != 0
        || region.page_size == 0
        || region.write_alignment == 0
    {
        return Err(ConfigError::InvalidFlashRegion);
    }

    Ok(FlashRegionInfo {
        offset: region.offset,
        length: region.length,
        page_size: region.page_size,
        write_alignment: region.write_alignment,
    })
}

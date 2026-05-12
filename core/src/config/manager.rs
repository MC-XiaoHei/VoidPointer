use crate::config::flash_region::{FlashRegionInfo, get_flash_region};
use crate::config::store::save_persisted_config;
use crate::config::load::{ActiveSlot, load_persisted_config};
use crate::config::storage::crc32;
use crate::config::types::{
    CURRENT_CONFIG_VERSION, CURRENT_STORAGE_VERSION, ConfigError, DeviceConfig, MAX_PAYLOAD_SIZE,
    SLOT_BUF_SIZE, SLOT_COUNT, SaveOutcome,
};
use crate::config::validate::validate_config;
use crate::config::write_session::WriteSession;

#[repr(align(4))]
struct AlignedBytes<const N: usize> {
    bytes: [u8; N],
}

impl<const N: usize> AlignedBytes<N> {
    const fn new() -> Self {
        Self { bytes: [0u8; N] }
    }

    fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
}

pub struct ConfigManager {
    current: DeviceConfig,
    payload_buf: AlignedBytes<MAX_PAYLOAD_SIZE>,
    payload_len: u32,
    payload_crc32: u32,
    dirty: bool,
    active_slot: Option<ActiveSlot>,
    next_sequence: u32,
    flash: FlashRegionInfo,
    slot_size: u32,
    slot_buf: AlignedBytes<SLOT_BUF_SIZE>,
    staged_payload_buf: AlignedBytes<MAX_PAYLOAD_SIZE>,
    write_session: WriteSession,
}

impl ConfigManager {
    pub fn new() -> Self {
        let flash = get_flash_region().unwrap_or_default();
        let slot_size = if flash.length >= SLOT_COUNT as u32 {
            flash.length / SLOT_COUNT as u32
        } else {
            0
        };

        let mut manager = Self {
            current: DeviceConfig::default(),
            payload_buf: AlignedBytes::new(),
            payload_len: 0,
            payload_crc32: 0,
            dirty: false,
            active_slot: None,
            next_sequence: 0,
            flash,
            slot_size,
            slot_buf: AlignedBytes::new(),
            staged_payload_buf: AlignedBytes::new(),
            write_session: WriteSession::default(),
        };

        manager
            .reencode_payload()
            .expect("default device config must encode within payload capacity");

        if let Ok(persisted) = load_persisted_config(
            manager.flash,
            manager.slot_size,
            &mut manager.slot_buf.bytes,
        ) {
            manager.current = persisted.config;
            manager
                .reencode_payload()
                .expect("persisted config must encode within payload capacity");
            manager.active_slot = Some(persisted.active_slot);
            manager.next_sequence = persisted.next_sequence;
            manager.clear_dirty();
        }

        manager
    }

    pub fn current_config_version(&self) -> u16 {
        CURRENT_CONFIG_VERSION
    }

    pub fn current_storage_version(&self) -> u16 {
        CURRENT_STORAGE_VERSION
    }

    pub fn current_config(&self) -> &DeviceConfig {
        &self.current
    }

    pub fn current_payload(&self) -> &[u8] {
        &self.payload_buf.as_slice()[..self.payload_len as usize]
    }

    pub fn current_payload_len(&self) -> u32 {
        self.payload_len
    }

    pub fn current_payload_crc32(&self) -> u32 {
        self.payload_crc32
    }

    pub fn replace_config(&mut self, config: DeviceConfig) -> Result<(), ConfigError> {
        validate_config(&config)?;
        self.current = config;
        self.reencode_payload()?;
        self.mark_dirty();
        Ok(())
    }

    pub fn restore_defaults(&mut self) -> Result<(), ConfigError> {
        self.replace_config(DeviceConfig::default())
    }

    pub fn save(&mut self) -> Result<SaveOutcome, ConfigError> {
        if !self.dirty {
            return Ok(SaveOutcome::Noop);
        }

        let payload_len = self.payload_len as usize;
        let payload_crc32 = self.payload_crc32;
        let mut payload_copy = [0u8; MAX_PAYLOAD_SIZE];
        payload_copy[..payload_len].copy_from_slice(self.current_payload());

        let new_active_slot = save_persisted_config(
            self.flash,
            self.slot_size,
            self.active_slot,
            self.next_sequence,
            &payload_copy[..payload_len],
            payload_crc32,
            &mut self.slot_buf.bytes,
        )?;
        self.active_slot = Some(new_active_slot);
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.clear_dirty();
        Ok(SaveOutcome::Saved)
    }

    pub fn begin_write(
        &mut self,
        expected_len: u32,
        expected_crc32: u32,
    ) -> Result<(), ConfigError> {
        self.write_session.begin(
            &mut self.staged_payload_buf.bytes,
            expected_len,
            expected_crc32,
        )
    }

    pub fn write_chunk(&mut self, offset: u32, chunk: &[u8]) -> Result<(), ConfigError> {
        self.write_session
            .write_chunk(&mut self.staged_payload_buf.bytes, offset, chunk)
    }

    pub fn commit_write(&mut self) -> Result<(), ConfigError> {
        let config = self.write_session.commit(&self.staged_payload_buf.bytes)?;
        self.replace_config(config)
    }

    pub fn abort_write(&mut self) {
        self.write_session.abort();
    }

    pub fn write_in_progress(&self) -> bool {
        self.write_session.is_active()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_persist(&self) -> bool {
        self.flash.length != 0 && self.slot_size as usize == self.slot_buf.as_slice().len()
    }

    pub fn poll(&mut self) {}

    /// 将当前 `DeviceConfig` 重新编码为 payload，更新 `payload_len` 和 `payload_crc32`
    fn reencode_payload(&mut self) -> Result<(), ConfigError> {
        let payload = postcard::to_slice(&self.current, self.payload_buf.as_mut_slice())
            .map_err(|_| ConfigError::EncodeFailed)?;
        self.payload_len = payload.len() as u32;
        self.payload_crc32 = crc32(payload);
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

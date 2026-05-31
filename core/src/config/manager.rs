use crate::config::flash_region::{FlashRegionInfo, get_flash_region};
use crate::config::load::{ActiveSlot, PersistedConfig, load_persisted_config};
use crate::config::storage::crc32;
use crate::config::store::save_persisted_config;
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
    pub(crate) fn from_flash(flash: FlashRegionInfo) -> Self {
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

        manager.reencode_payload();
        manager
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn new() -> Self {
        let flash = get_flash_region().unwrap_or_default();
        let mut manager = Self::from_flash(flash);
        if let Some(persisted) = Self::flash_load(
            manager.flash,
            manager.slot_size,
            &mut manager.slot_buf.bytes,
        ) {
            if validate_config(&persisted.config).is_ok() {
                manager.apply_persisted(persisted);
            }
        }
        manager
    }

    fn apply_persisted(&mut self, persisted: PersistedConfig) {
        self.current = persisted.config;
        self.reencode_payload();
        self.active_slot = Some(persisted.active_slot);
        self.next_sequence = persisted.next_sequence;
        if persisted.was_migrated {
            self.mark_dirty();
        } else {
            self.clear_dirty();
        }
    }

    #[cfg_attr(coverage, coverage(off))]
    fn flash_load(
        flash: FlashRegionInfo,
        slot_size: u32,
        slot_buf: &mut [u8; SLOT_BUF_SIZE],
    ) -> Option<PersistedConfig> {
        load_persisted_config(flash, slot_size, slot_buf).ok()
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
        self.reencode_payload();
        self.mark_dirty();
        Ok(())
    }

    pub fn restore_defaults(&mut self) -> Result<(), ConfigError> {
        self.replace_config(DeviceConfig::default())
    }

    #[cfg_attr(coverage, coverage(off))]
    pub fn save(&mut self) -> Result<SaveOutcome, ConfigError> {
        if !self.dirty {
            return Ok(SaveOutcome::Noop);
        }
        self.save_impl()
    }

    #[cfg_attr(coverage, coverage(off))]
    fn save_impl(&mut self) -> Result<SaveOutcome, ConfigError> {
        let payload_copy = self.prepare_save_payload();
        let new_active_slot = Self::flash_write(
            self.flash,
            self.slot_size,
            self.active_slot,
            self.next_sequence,
            &payload_copy[..self.payload_len as usize],
            self.payload_crc32,
            &mut self.slot_buf.bytes,
        )?;
        self.on_save_success(new_active_slot);
        Ok(SaveOutcome::Saved)
    }

    fn on_save_success(&mut self, new_active_slot: ActiveSlot) {
        self.active_slot = Some(new_active_slot);
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.clear_dirty();
    }

    fn prepare_save_payload(&self) -> [u8; MAX_PAYLOAD_SIZE] {
        let payload_len = self.payload_len as usize;
        let mut payload_copy = [0u8; MAX_PAYLOAD_SIZE];
        payload_copy[..payload_len].copy_from_slice(self.current_payload());
        payload_copy
    }

    #[cfg_attr(coverage, coverage(off))]
    fn flash_write(
        flash: FlashRegionInfo,
        slot_size: u32,
        active_slot: Option<ActiveSlot>,
        next_sequence: u32,
        payload: &[u8],
        payload_crc32: u32,
        slot_buf: &mut [u8; SLOT_BUF_SIZE],
    ) -> Result<ActiveSlot, ConfigError> {
        save_persisted_config(
            flash,
            slot_size,
            active_slot,
            next_sequence,
            payload,
            payload_crc32,
            slot_buf,
        )
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

    fn reencode_payload(&mut self) {
        let (len, c) = encode_config(&self.current, self.payload_buf.as_mut_slice())
            .expect("MAX_PAYLOAD_SIZE 足以容纳序列化后的任意 DeviceConfig");
        self.payload_len = len;
        self.payload_crc32 = c;
    }
}

pub(crate) fn encode_config(
    config: &DeviceConfig,
    buf: &mut [u8],
) -> Result<(u32, u32), ConfigError> {
    let payload = postcard::to_slice(config, buf).map_err(|_| ConfigError::EncodeFailed)?;
    let len = payload.len() as u32;
    let c = crc32(payload);
    Ok((len, c))
}

#[cfg_attr(coverage, coverage(off))]
impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::config::flash_region::FlashRegionInfo;
    use crate::config::load::{PersistedConfig, parse_bytes};
    use crate::config::types::DeviceConfig;

    fn make_manager() -> ConfigManager {
        ConfigManager::from_flash(FlashRegionInfo::default())
    }

    #[test]
    fn encode_default_config() {
        let config = DeviceConfig::default();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let (len, _crc) = encode_config(&config, &mut buf).unwrap();
        assert!(len > 0);
        assert!(len as usize <= MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn encode_and_decode_roundtrip() {
        let config = DeviceConfig::default();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let (len, _crc) = encode_config(&config, &mut buf).unwrap();
        let decoded: DeviceConfig = parse_bytes(&buf[..len as usize]).unwrap();
        assert_eq!(config, decoded);
    }

    #[test]
    fn encode_non_default_config() {
        let mut config = DeviceConfig::default();
        config.motion.sensitivity_x = 24000.0;
        config.motion.invert_y = true;
        config.power.suspend_timeout_ms = 30000;
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let (len, _crc) = encode_config(&config, &mut buf).unwrap();
        let decoded: DeviceConfig = parse_bytes(&buf[..len as usize]).unwrap();
        assert_eq!(config, decoded);
    }

    #[test]
    fn version_constants() {
        let m = make_manager();
        assert_eq!(m.current_config_version(), CURRENT_CONFIG_VERSION);
        assert_eq!(m.current_storage_version(), CURRENT_STORAGE_VERSION);
    }

    #[test]
    fn current_config_returns_current() {
        let m = make_manager();
        assert_eq!(*m.current_config(), DeviceConfig::default());
    }

    #[test]
    fn replace_config_valid() {
        let mut m = make_manager();
        let mut cfg = DeviceConfig::default();
        cfg.motion.invert_x = true;
        cfg.motion.invert_y = true;

        m.replace_config(cfg.clone()).unwrap();
        assert_eq!(*m.current_config(), cfg);
        assert!(m.is_dirty());
        assert!(m.current_payload_len() > 0);
        assert!(m.current_payload_crc32() != 0);
    }

    #[test]
    fn replace_config_invalid_rejected() {
        let mut m = make_manager();
        let mut cfg = DeviceConfig::default();
        cfg.report.report_hz = 0.0;

        let err = m.replace_config(cfg).unwrap_err();
        assert_eq!(err, ConfigError::ValidationFailed);
        assert_eq!(*m.current_config(), DeviceConfig::default());
        assert!(!m.is_dirty());
    }

    #[test]
    fn restore_defaults_works() {
        let mut m = make_manager();
        let mut cfg = DeviceConfig::default();
        cfg.motion.invert_x = true;
        m.replace_config(cfg).unwrap();
        assert!(m.is_dirty());

        m.restore_defaults().unwrap();
        assert_eq!(*m.current_config(), DeviceConfig::default());
        assert!(m.is_dirty());
    }

    #[test]
    fn save_noop_when_clean() {
        let mut m = make_manager();
        assert!(!m.is_dirty());
        assert_eq!(m.save().unwrap(), SaveOutcome::Noop);
    }

    #[test]
    fn dirty_flag_lifecycle() {
        let mut m = make_manager();
        assert!(!m.is_dirty());

        m.mark_dirty();
        assert!(m.is_dirty());

        m.clear_dirty();
        assert!(!m.is_dirty());
    }

    #[test]
    fn write_session_lifecycle() {
        let mut m = make_manager();
        let cfg = DeviceConfig::default();
        let mut tmp = [0u8; MAX_PAYLOAD_SIZE];
        let (payload_len, payload_crc) = encode_config(&cfg, &mut tmp).unwrap();

        assert!(!m.write_in_progress());
        m.begin_write(payload_len, payload_crc).unwrap();
        assert!(m.write_in_progress());

        m.write_chunk(0, &tmp[..payload_len as usize]).unwrap();
        m.commit_write().unwrap();
        assert!(!m.write_in_progress());
        assert!(m.is_dirty());
        assert_eq!(*m.current_config(), DeviceConfig::default());
    }

    #[test]
    fn abort_write_cancels() {
        let mut m = make_manager();
        let (len, crc) =
            encode_config(&DeviceConfig::default(), &mut [0u8; MAX_PAYLOAD_SIZE]).unwrap();

        m.begin_write(len, crc).unwrap();
        assert!(m.write_in_progress());
        m.abort_write();
        assert!(!m.write_in_progress());
    }

    #[test]
    fn can_persist_no_flash() {
        let m = make_manager();
        assert!(!m.can_persist());
    }

    #[test]
    fn from_flash_with_valid_flash() {
        let flash = FlashRegionInfo {
            offset: 0x1000,
            length: 8192,
            page_size: 256,
            write_alignment: 4,
        };
        let m = ConfigManager::from_flash(flash);
        assert!(m.can_persist());
        assert_eq!(m.flash.length, 8192);
    }

    #[test]
    fn from_flash_with_zero_flash() {
        let flash = FlashRegionInfo::default();
        let m = ConfigManager::from_flash(flash);
        assert!(!m.can_persist());
        assert_eq!(m.slot_size, 0);
    }

    #[test]
    fn from_flash_encodes_default_payload() {
        let m = ConfigManager::from_flash(FlashRegionInfo::default());
        assert!(m.current_payload_len() > 0);
        assert!(m.current_payload_crc32() != 0);
    }

    #[test]
    fn current_payload_after_replace() {
        let mut m = make_manager();
        let initial_len = m.current_payload_len();
        assert!(initial_len > 0);

        let mut cfg = DeviceConfig::default();
        cfg.motion.invert_x = true;
        m.replace_config(cfg).unwrap();

        assert!(m.current_payload_len() > 0);
        let decoded: DeviceConfig = parse_bytes(m.current_payload()).unwrap();
        assert_eq!(decoded.motion.invert_x, true);
    }

    #[test]
    fn prepare_save_payload_copies_current() {
        let mut m = make_manager();
        let mut cfg = DeviceConfig::default();
        cfg.motion.invert_x = true;
        m.replace_config(cfg).unwrap();

        let copy = m.prepare_save_payload();
        let expected = m.current_payload();
        assert_eq!(&copy[..expected.len()], expected);
    }

    #[test]
    fn on_save_success_updates_state() {
        let mut m = make_manager();
        m.next_sequence = 100;
        m.mark_dirty();

        m.on_save_success(ActiveSlot::B);
        assert_eq!(m.active_slot, Some(ActiveSlot::B));
        assert_eq!(m.next_sequence, 101);
        assert!(!m.is_dirty());
    }

    #[test]
    fn encode_config_buffer_too_small_returns_error() {
        let config = DeviceConfig::default();
        let mut buf = [0u8; 1];
        assert_eq!(
            encode_config(&config, &mut buf),
            Err(ConfigError::EncodeFailed)
        );
    }

    #[test]
    fn commit_write_rejects_bad_payload() {
        let mut m = make_manager();
        let garbage = [0xDE, 0xAD, 0xBE, 0xEF];
        let crc = crate::config::storage::crc32(&garbage);

        m.begin_write(garbage.len() as u32, crc).unwrap();
        m.write_chunk(0, &garbage).unwrap();
        assert_eq!(m.commit_write(), Err(ConfigError::DeserializeFailed));
    }

    #[test]
    fn apply_persisted_updates_state() {
        let mut m = make_manager();
        let mut persisted_cfg = DeviceConfig::default();
        persisted_cfg.motion.invert_y = true;

        let persisted = PersistedConfig {
            active_slot: ActiveSlot::B,
            next_sequence: 42,
            config: persisted_cfg,
            was_migrated: false,
        };

        m.apply_persisted(persisted);
        assert_eq!(m.active_slot, Some(ActiveSlot::B));
        assert_eq!(m.next_sequence, 42);
        assert_eq!(m.current.motion.invert_y, true);
        assert!(!m.is_dirty());
        assert!(m.current_payload_len() > 0);
    }

    #[test]
    fn apply_persisted_migrated_marks_dirty() {
        let mut m = make_manager();
        let mut persisted_cfg = DeviceConfig::default();
        persisted_cfg.motion.invert_x = true;

        let persisted = PersistedConfig {
            active_slot: ActiveSlot::A,
            next_sequence: 10,
            config: persisted_cfg,
            was_migrated: true,
        };

        m.apply_persisted(persisted);
        assert!(m.is_dirty());
        assert_eq!(*m.current_config(), persisted_cfg);
    }
}

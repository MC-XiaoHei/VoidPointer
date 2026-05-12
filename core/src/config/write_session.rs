use crate::config::storage::crc32;
use crate::config::types::{ConfigError, DeviceConfig, MAX_PAYLOAD_SIZE};
use crate::config::validate::validate_config;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WriteSession {
    active: bool,
    expected_len: u32,
    expected_crc32: u32,
    received_len: u32,
}

impl WriteSession {
    pub(crate) fn begin(
        &mut self,
        staging_buf: &mut [u8; MAX_PAYLOAD_SIZE],
        expected_len: u32,
        expected_crc32: u32,
    ) -> Result<(), ConfigError> {
        if self.active {
            return Err(ConfigError::WriteSessionBusy);
        }
        if expected_len == 0 || expected_len as usize > MAX_PAYLOAD_SIZE {
            return Err(ConfigError::InvalidPayloadLength);
        }

        staging_buf.fill(0);
        *self = Self {
            active: true,
            expected_len,
            expected_crc32,
            received_len: 0,
        };
        Ok(())
    }

    pub(crate) fn write_chunk(
        &mut self,
        staging_buf: &mut [u8; MAX_PAYLOAD_SIZE],
        offset: u32,
        chunk: &[u8],
    ) -> Result<(), ConfigError> {
        if !self.active {
            return Err(ConfigError::WriteSessionNotActive);
        }
        if offset != self.received_len {
            return Err(ConfigError::WriteSequenceMismatch);
        }

        let end = offset as usize + chunk.len();
        if end > self.expected_len as usize {
            return Err(ConfigError::InvalidPayloadLength);
        }

        staging_buf[offset as usize..end].copy_from_slice(chunk);
        self.received_len = end as u32;
        Ok(())
    }

    pub(crate) fn commit(
        &mut self,
        staging_buf: &[u8; MAX_PAYLOAD_SIZE],
    ) -> Result<DeviceConfig, ConfigError> {
        if !self.active {
            return Err(ConfigError::WriteSessionNotActive);
        }
        if self.received_len != self.expected_len {
            return Err(ConfigError::WriteSequenceMismatch);
        }

        let expected_len = self.expected_len as usize;
        let payload = &staging_buf[..expected_len];
        if crc32(payload) != self.expected_crc32 {
            return Err(ConfigError::PayloadCrcMismatch);
        }

        let config: DeviceConfig =
            postcard::from_bytes(payload).map_err(|_| ConfigError::DeserializeFailed)?;
        validate_config(&config)?;
        *self = Self::default();
        Ok(config)
    }

    pub(crate) fn abort(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }
}

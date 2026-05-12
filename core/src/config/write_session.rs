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

#[cfg(test)]
extern crate alloc;

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::config::storage::crc32;
    use alloc::vec::Vec;

    fn valid_payload() -> (Vec<u8>, u32) {
        let c = DeviceConfig::default();
        let mut buf = [0u8; 256];
        let encoded = postcard::to_slice(&c, &mut buf).unwrap();
        (encoded.to_vec(), crc32(encoded))
    }

    #[test]
    fn happy_path() {
        let (payload, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();

        s.begin(&mut buf, payload.len() as u32, crc).unwrap();
        s.write_chunk(&mut buf, 0, &payload).unwrap();
        let cfg = s.commit(&buf).unwrap();
        assert_eq!(cfg, DeviceConfig::default());
        assert!(!s.is_active());
    }

    #[test]
    fn abort_resets() {
        let (payload, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();

        s.begin(&mut buf, payload.len() as u32, crc).unwrap();
        assert!(s.is_active());
        s.abort();
        assert!(!s.is_active());
    }

    #[test]
    fn reject_write_chunk_when_inactive() {
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        assert_eq!(
            s.write_chunk(&mut buf, 0, &[1, 2, 3]),
            Err(ConfigError::WriteSessionNotActive)
        );
    }

    #[test]
    fn reject_commit_when_inactive() {
        let buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        assert_eq!(s.commit(&buf), Err(ConfigError::WriteSessionNotActive));
    }

    #[test]
    fn reject_oversize_len() {
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        assert_eq!(
            s.begin(&mut buf, MAX_PAYLOAD_SIZE as u32 + 1, 0),
            Err(ConfigError::InvalidPayloadLength)
        );
    }

    #[test]
    fn reject_zero_len() {
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        assert_eq!(
            s.begin(&mut buf, 0, 0),
            Err(ConfigError::InvalidPayloadLength)
        );
    }

    #[test]
    fn reject_begin_when_active() {
        let (pl, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, pl.len() as u32, crc).unwrap();
        assert_eq!(
            s.begin(&mut buf, pl.len() as u32, crc),
            Err(ConfigError::WriteSessionBusy)
        );
    }

    #[test]
    fn reject_wrong_offset() {
        let (pl, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, pl.len() as u32, crc).unwrap();
        assert_eq!(
            s.write_chunk(&mut buf, 1, &pl[1..]),
            Err(ConfigError::WriteSequenceMismatch)
        );
    }

    #[test]
    fn reject_chunk_exceeding_expected() {
        let (pl, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, (pl.len() - 1) as u32, crc).unwrap();
        assert_eq!(
            s.write_chunk(&mut buf, 0, &pl),
            Err(ConfigError::InvalidPayloadLength)
        );
    }

    #[test]
    fn reject_commit_incomplete() {
        let (pl, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, pl.len() as u32, crc).unwrap();
        s.write_chunk(&mut buf, 0, &pl[..pl.len() / 2]).unwrap();
        assert_eq!(s.commit(&buf), Err(ConfigError::WriteSequenceMismatch));
    }

    #[test]
    fn reject_crc_mismatch() {
        let (pl, _) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, pl.len() as u32, 0xDEAD_BEEF).unwrap();
        s.write_chunk(&mut buf, 0, &pl).unwrap();
        assert_eq!(s.commit(&buf), Err(ConfigError::PayloadCrcMismatch));
    }

    #[test]
    fn reject_commit_validation_fail() {
        // 合法 postcard 编码但配置校验不通过（report_hz = 0）
        let mut invalid = DeviceConfig::default();
        invalid.report.report_hz = 0.0;
        let mut tmp = [0u8; 256];
        let encoded = postcard::to_slice(&invalid, &mut tmp).unwrap();
        let crc = crate::config::storage::crc32(encoded);

        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, encoded.len() as u32, crc).unwrap();
        s.write_chunk(&mut buf, 0, encoded).unwrap();
        assert_eq!(s.commit(&buf), Err(ConfigError::ValidationFailed));
    }

    #[test]
    fn reject_commit_deserialize_fail() {
        // CRC 正确但 payload 不是合法 postcard 编码
        let garbage = [0xDE, 0xAD, 0xBE, 0xEF];
        let crc = crate::config::storage::crc32(&garbage);
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, garbage.len() as u32, crc).unwrap();
        s.write_chunk(&mut buf, 0, &garbage).unwrap();
        assert_eq!(s.commit(&buf), Err(ConfigError::DeserializeFailed));
    }

    #[test]
    fn multi_chunk_write() {
        let (pl, crc) = valid_payload();
        let mut buf = [0u8; MAX_PAYLOAD_SIZE];
        let mut s = WriteSession::default();
        s.begin(&mut buf, pl.len() as u32, crc).unwrap();

        let n = 4;
        let sz = pl.len() / n;
        for i in 0..n {
            let start = i * sz;
            let end = if i == n - 1 { pl.len() } else { (i + 1) * sz };
            s.write_chunk(&mut buf, start as u32, &pl[start..end])
                .unwrap();
        }
        assert_eq!(s.commit(&buf).unwrap(), DeviceConfig::default());
    }
}

use crate::config::{
    load::parse_bytes,
    types::{ConfigError, DeviceConfig},
};

pub(crate) fn migrate_payload(
    payload: &[u8],
    from_version: u16,
) -> Result<DeviceConfig, ConfigError> {
    let current = crate::config::types::CURRENT_CONFIG_VERSION;

    if from_version == current {
        return parse_bytes(payload);
    }
    if from_version > current {
        return Err(ConfigError::UnsupportedConfigVersion);
    }

    match from_version {
        _ => Err(ConfigError::MigrationFailed),
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;
    use crate::config::types::CURRENT_CONFIG_VERSION;

    #[test]
    fn migrate_same_version_direct() {
        let cfg = DeviceConfig::default();
        let mut buf = [0u8; 256];
        let payload = postcard::to_slice(&cfg, &mut buf).unwrap();
        let result = migrate_payload(payload, CURRENT_CONFIG_VERSION).unwrap();
        assert_eq!(result, cfg);
    }

    #[test]
    fn migrate_from_future_version_errors() {
        let err = migrate_payload(&[0u8; 10], CURRENT_CONFIG_VERSION + 1).unwrap_err();
        assert_eq!(err, ConfigError::UnsupportedConfigVersion);
    }

    #[test]
    fn migrate_corrupt_payload_errors() {
        let err = migrate_payload(&[0xDE, 0xAD], CURRENT_CONFIG_VERSION).unwrap_err();
        assert_eq!(err, ConfigError::DeserializeFailed);
    }

    #[test]
    fn migrate_payload_rejects_empty() {
        let err = migrate_payload(&[], CURRENT_CONFIG_VERSION).unwrap_err();
        assert_eq!(err, ConfigError::DeserializeFailed);
    }

    #[test]
    fn migrate_unimplemented_version_errors() {
        let err = migrate_payload(&[0u8; 10], 0).unwrap_err();
        assert_eq!(err, ConfigError::MigrationFailed);
    }
}

use crate::config::{ConfigManager, SaveOutcome};
use crate::hid::types::{CUSTOM_REPORT_PAYLOAD_CAPACITY, CustomReport};
use crate::power::{PowerManager, PowerState};
use crate::route::{HidRoute, HidRouter, UsbState};
use crate::vendor::VendorRxStats;

pub const CUSTOM_PROTOCOL_MAGIC: u8 = 0xA5;
pub const CUSTOM_PROTOCOL_VERSION: u8 = 1;
pub const CUSTOM_PROTOCOL_HEADER_LEN: usize = 16;
pub const CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN: usize =
    CUSTOM_REPORT_PAYLOAD_CAPACITY - CUSTOM_PROTOCOL_HEADER_LEN;

pub const CUSTOM_FLAG_REQUEST: u8 = 1 << 0;
pub const CUSTOM_FLAG_RESPONSE: u8 = 1 << 1;
pub const CUSTOM_FLAG_FRAGMENT: u8 = 1 << 2;

pub const CUSTOM_STATUS_OK: u16 = 0x0000;
pub const CUSTOM_STATUS_INVALID_COMMAND: u16 = 0x0001;
pub const CUSTOM_STATUS_INVALID_ARGUMENT: u16 = 0x0002;
pub const CUSTOM_STATUS_BAD_LENGTH: u16 = 0x0003;
pub const CUSTOM_STATUS_BAD_SEQUENCE: u16 = 0x0004;
pub const CUSTOM_STATUS_CRC_MISMATCH: u16 = 0x0005;
pub const CUSTOM_STATUS_BUSY: u16 = 0x0006;
pub const CUSTOM_STATUS_NOT_READY: u16 = 0x0007;
pub const CUSTOM_STATUS_STORAGE_ERROR: u16 = 0x0008;
pub const CUSTOM_STATUS_INTERNAL_ERROR: u16 = 0x0009;

pub const CUSTOM_CMD_PING: u16 = 0x0000;
pub const CUSTOM_CMD_GET_PROTOCOL_INFO: u16 = 0x0001;
pub const CUSTOM_CMD_GET_DEVICE_INFO: u16 = 0x0002;
pub const CUSTOM_CMD_GET_CONFIG_INFO: u16 = 0x0100;
pub const CUSTOM_CMD_READ_CONFIG: u16 = 0x0101;
pub const CUSTOM_CMD_WRITE_CONFIG_BEGIN: u16 = 0x0102;
pub const CUSTOM_CMD_WRITE_CONFIG_CHUNK: u16 = 0x0103;
pub const CUSTOM_CMD_WRITE_CONFIG_COMMIT: u16 = 0x0104;
pub const CUSTOM_CMD_WRITE_CONFIG_ABORT: u16 = 0x0105;
pub const CUSTOM_CMD_SAVE_CONFIG: u16 = 0x0106;
pub const CUSTOM_CMD_RESTORE_DEFAULTS: u16 = 0x0107;
pub const CUSTOM_CMD_GET_ROUTE_STATE: u16 = 0x0201;
pub const CUSTOM_CMD_GET_POWER_STATE: u16 = 0x0202;
pub const CUSTOM_CMD_GET_DIAGNOSTICS: u16 = 0x0300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomFrameView<'a> {
    pub magic: u8,
    pub version: u8,
    pub flags: u8,
    pub sequence: u8,
    pub command: u16,
    pub status: u16,
    pub offset: u16,
    pub total_len: u32,
    pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    TooShort,
    BadMagic,
    UnsupportedVersion,
    FragmentNotSupported,
    PayloadTooLarge,
    LengthMismatch,
    TotalLengthMismatch,
    NonZeroOffset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomFrameHeader {
    pub flags: u8,
    pub sequence: u8,
    pub command: u16,
    pub status: u16,
    pub offset: u16,
    pub total_len: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProtocolStats {
    pub rx_ok: u16,
    pub rx_invalid: u16,
    pub rx_unsupported: u16,
    pub tx_generated: u16,
    pub tx_dropped_no_route: u16,
}

pub fn parse_frame(buf: &[u8]) -> Result<CustomFrameView<'_>, ParseError> {
    if buf.len() < CUSTOM_PROTOCOL_HEADER_LEN {
        return Err(ParseError::TooShort);
    }

    if buf[0] != CUSTOM_PROTOCOL_MAGIC {
        return Err(ParseError::BadMagic);
    }

    if buf[1] != CUSTOM_PROTOCOL_VERSION {
        return Err(ParseError::UnsupportedVersion);
    }

    let flags = buf[2];
    if (flags & CUSTOM_FLAG_FRAGMENT) != 0 {
        return Err(ParseError::FragmentNotSupported);
    }

    let command = u16::from_le_bytes([buf[4], buf[5]]);
    let status = u16::from_le_bytes([buf[6], buf[7]]);
    let offset = u16::from_le_bytes([buf[8], buf[9]]);
    let total_len = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
    let payload_len = u16::from_le_bytes([buf[14], buf[15]]) as usize;

    if payload_len > CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN {
        return Err(ParseError::PayloadTooLarge);
    }

    let expected_len = CUSTOM_PROTOCOL_HEADER_LEN + payload_len;
    if buf.len() < expected_len {
        return Err(ParseError::LengthMismatch);
    }

    if offset != 0 {
        return Err(ParseError::NonZeroOffset);
    }

    if total_len != payload_len as u32 {
        return Err(ParseError::TotalLengthMismatch);
    }

    Ok(CustomFrameView {
        magic: buf[0],
        version: buf[1],
        flags,
        sequence: buf[3],
        command,
        status,
        offset,
        total_len,
        payload: &buf[CUSTOM_PROTOCOL_HEADER_LEN..expected_len],
    })
}

pub fn encode_frame(
    header: CustomFrameHeader,
    payload: &[u8],
    out: &mut CustomReport,
) -> Result<(), ParseError> {
    if payload.len() > CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN {
        return Err(ParseError::PayloadTooLarge);
    }

    out.data[0] = CUSTOM_PROTOCOL_MAGIC;
    out.data[1] = CUSTOM_PROTOCOL_VERSION;
    out.data[2] = header.flags;
    out.data[3] = header.sequence;
    out.data[4..6].copy_from_slice(&header.command.to_le_bytes());
    out.data[6..8].copy_from_slice(&header.status.to_le_bytes());
    out.data[8..10].copy_from_slice(&header.offset.to_le_bytes());
    out.data[10..14].copy_from_slice(&header.total_len.to_le_bytes());
    out.data[14..16].copy_from_slice(&(payload.len() as u16).to_le_bytes());

    if !payload.is_empty() {
        out.data[CUSTOM_PROTOCOL_HEADER_LEN..CUSTOM_PROTOCOL_HEADER_LEN + payload.len()]
            .copy_from_slice(payload);
    }

    // 填充到 64 字节，匹配 USB report descriptor 定义的 Input report 大小
    let frame_len = CUSTOM_PROTOCOL_HEADER_LEN + payload.len();
    for b in &mut out.data[frame_len..] {
        *b = 0;
    }
    out.len = CUSTOM_REPORT_PAYLOAD_CAPACITY as u16;
    Ok(())
}

pub fn encode_response(
    command: u16,
    sequence: u8,
    status: u16,
    payload: &[u8],
    out: &mut CustomReport,
) -> Result<(), ParseError> {
    encode_frame(
        CustomFrameHeader {
            flags: CUSTOM_FLAG_RESPONSE,
            sequence,
            command,
            status,
            offset: 0,
            total_len: payload.len() as u32,
        },
        payload,
        out,
    )
}

pub fn encode_error_response(
    command: u16,
    sequence: u8,
    status: u16,
    out: &mut CustomReport,
) -> Result<(), ParseError> {
    encode_response(command, sequence, status, &[], out)
}

pub fn build_protocol_info_payload() -> [u8; 4] {
    [
        CUSTOM_PROTOCOL_VERSION,
        CUSTOM_PROTOCOL_HEADER_LEN as u8,
        CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN as u8,
        0u8,
    ]
}

pub fn build_route_state_payload(router: &HidRouter) -> [u8; 5] {
    [
        router.preferred_mouse_route().as_ffi(),
        router.preferred_custom_route().as_ffi(),
        router.is_ble_connected() as u8,
        router.is_usb_configured() as u8,
        router.has_wireless_connection() as u8,
    ]
}

pub fn build_config_info_payload(config: &ConfigManager) -> [u8; 12] {
    let mut payload = [0u8; 12];
    payload[0..2].copy_from_slice(&config.current_config_version().to_le_bytes());
    payload[2] = config.is_dirty() as u8;
    payload[3] = config.write_in_progress() as u8;
    payload[4..8].copy_from_slice(&config.current_payload_len().to_le_bytes());
    payload[8..12].copy_from_slice(&config.current_payload_crc32().to_le_bytes());
    payload
}

fn parse_write_begin_payload(payload: &[u8]) -> Result<(u32, u32), u16> {
    if payload.len() != 8 {
        return Err(CUSTOM_STATUS_BAD_LENGTH);
    }

    let total_len = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let crc32 = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
    Ok((total_len, crc32))
}

fn parse_write_chunk_payload(payload: &[u8]) -> Result<(u32, &[u8]), u16> {
    if payload.len() < 4 {
        return Err(CUSTOM_STATUS_BAD_LENGTH);
    }

    let offset = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    Ok((offset, &payload[4..]))
}

fn map_config_error_to_status(err: crate::config::ConfigError) -> u16 {
    match err {
        crate::config::ConfigError::WriteSessionBusy => CUSTOM_STATUS_BUSY,
        crate::config::ConfigError::WriteSessionNotActive
        | crate::config::ConfigError::WriteSequenceMismatch => CUSTOM_STATUS_BAD_SEQUENCE,
        crate::config::ConfigError::PayloadCrcMismatch => CUSTOM_STATUS_CRC_MISMATCH,
        crate::config::ConfigError::InvalidPayloadLength => CUSTOM_STATUS_BAD_LENGTH,
        crate::config::ConfigError::ValidationFailed => CUSTOM_STATUS_INVALID_ARGUMENT,
        crate::config::ConfigError::StorageUnavailable
        | crate::config::ConfigError::StorageEmpty
        | crate::config::ConfigError::InvalidFlashRegion
        | crate::config::ConfigError::FlashEraseFailed
        | crate::config::ConfigError::FlashWriteFailed
        | crate::config::ConfigError::ReadbackVerifyFailed => CUSTOM_STATUS_STORAGE_ERROR,
        crate::config::ConfigError::EncodeFailed
        | crate::config::ConfigError::PayloadTooLarge
        | crate::config::ConfigError::DeserializeFailed
        | crate::config::ConfigError::HeaderCrcMismatch
        | crate::config::ConfigError::InvalidMagic
        | crate::config::ConfigError::UnsupportedStorageVersion
        | crate::config::ConfigError::UnsupportedConfigVersion
        | crate::config::ConfigError::MigrationFailed => CUSTOM_STATUS_INTERNAL_ERROR,
    }
}

pub fn build_power_state_payload(
    power: &PowerManager,
    router: &HidRouter,
    config: &ConfigManager,
) -> [u8; 12] {
    let mut payload = [0u8; 12];
    payload[0] = match power.state() {
        PowerState::Active => 0,
        PowerState::Suspend => 1,
        PowerState::Sleep => 2,
    };
    payload[1] = match router.usb_state() {
        UsbState::Detached => 0,
        UsbState::Attached => 1,
        UsbState::Configured => 2,
        UsbState::Suspended => 3,
        UsbState::Error => 4,
    };
    payload[2] = router.has_wireless_connection() as u8;
    payload[3] = config.is_dirty() as u8;

    let power_config = power.config();
    payload[4..8].copy_from_slice(&power_config.suspend_timeout_ms.to_le_bytes());
    payload[8..12].copy_from_slice(&power_config.disconnect_sleep_timeout_ms.to_le_bytes());
    payload
}

pub fn build_diagnostics_payload(
    protocol_stats: ProtocolStats,
    vendor_rx_stats: VendorRxStats,
) -> [u8; 14] {
    let mut payload = [0u8; 14];
    payload[0..2].copy_from_slice(&protocol_stats.rx_ok.to_le_bytes());
    payload[2..4].copy_from_slice(&protocol_stats.rx_invalid.to_le_bytes());
    payload[4..6].copy_from_slice(&protocol_stats.rx_unsupported.to_le_bytes());
    payload[6..8].copy_from_slice(&protocol_stats.tx_generated.to_le_bytes());
    payload[8..10].copy_from_slice(&protocol_stats.tx_dropped_no_route.to_le_bytes());
    payload[10..12].copy_from_slice(&vendor_rx_stats.dropped.to_le_bytes());
    payload[12..14].copy_from_slice(&vendor_rx_stats.too_large.to_le_bytes());
    payload
}

pub fn handle_request(
    frame: CustomFrameView<'_>,
    router: &HidRouter,
    config: &mut ConfigManager,
    power: &PowerManager,
    protocol_stats: ProtocolStats,
    vendor_rx_stats: VendorRxStats,
    out: &mut CustomReport,
) -> Result<(), u16> {
    if (frame.flags & CUSTOM_FLAG_REQUEST) == 0 || (frame.flags & CUSTOM_FLAG_RESPONSE) != 0 {
        return encode_error_response(
            frame.command,
            frame.sequence,
            CUSTOM_STATUS_INVALID_ARGUMENT,
            out,
        )
        .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR);
    }

    match frame.command {
        CUSTOM_CMD_PING => encode_response(
            frame.command,
            frame.sequence,
            CUSTOM_STATUS_OK,
            frame.payload,
            out,
        )
        .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
        CUSTOM_CMD_GET_PROTOCOL_INFO => {
            let payload = build_protocol_info_payload();
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                &payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_GET_DEVICE_INFO => {
            let payload = b"VoidPointer";
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_GET_CONFIG_INFO => {
            let payload = build_config_info_payload(config);
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                &payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_READ_CONFIG => encode_response(
            frame.command,
            frame.sequence,
            CUSTOM_STATUS_OK,
            config.current_payload(),
            out,
        )
        .map_err(|err| match err {
            ParseError::PayloadTooLarge => CUSTOM_STATUS_BAD_LENGTH,
            _ => CUSTOM_STATUS_INTERNAL_ERROR,
        }),
        CUSTOM_CMD_WRITE_CONFIG_BEGIN => {
            let (total_len, crc32) = parse_write_begin_payload(frame.payload)?;
            match config.begin_write(total_len, crc32) {
                Ok(()) => {
                    encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                        .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
                }
                Err(err) => encode_error_response(
                    frame.command,
                    frame.sequence,
                    map_config_error_to_status(err),
                    out,
                )
                .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
            }
        }
        CUSTOM_CMD_WRITE_CONFIG_CHUNK => {
            let (offset, chunk) = parse_write_chunk_payload(frame.payload)?;
            match config.write_chunk(offset, chunk) {
                Ok(()) => {
                    encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                        .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
                }
                Err(err) => encode_error_response(
                    frame.command,
                    frame.sequence,
                    map_config_error_to_status(err),
                    out,
                )
                .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
            }
        }
        CUSTOM_CMD_WRITE_CONFIG_COMMIT => match config.commit_write() {
            Ok(()) => encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
            Err(err) => encode_error_response(
                frame.command,
                frame.sequence,
                map_config_error_to_status(err),
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
        },
        CUSTOM_CMD_WRITE_CONFIG_ABORT => {
            config.abort_write();
            encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_SAVE_CONFIG => match config.save() {
            Ok(SaveOutcome::Noop) | Ok(SaveOutcome::Saved) => {
                encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                    .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
            }
            Err(_) => encode_error_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_STORAGE_ERROR,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
        },
        CUSTOM_CMD_RESTORE_DEFAULTS => match config.restore_defaults() {
            Ok(()) => encode_response(frame.command, frame.sequence, CUSTOM_STATUS_OK, &[], out)
                .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
            Err(_) => encode_error_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_INTERNAL_ERROR,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
        },
        CUSTOM_CMD_GET_ROUTE_STATE => {
            let payload = build_route_state_payload(router);
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                &payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_GET_POWER_STATE => {
            let payload = build_power_state_payload(power, router, config);
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                &payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        CUSTOM_CMD_GET_DIAGNOSTICS => {
            let payload = build_diagnostics_payload(protocol_stats, vendor_rx_stats);
            encode_response(
                frame.command,
                frame.sequence,
                CUSTOM_STATUS_OK,
                &payload,
                out,
            )
            .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR)
        }
        _ => encode_error_response(
            frame.command,
            frame.sequence,
            CUSTOM_STATUS_INVALID_COMMAND,
            out,
        )
        .map_err(|_| CUSTOM_STATUS_INTERNAL_ERROR),
    }
}

pub fn preferred_response_route(router: &HidRouter, request_route: u8) -> HidRoute {
    let request_route = HidRoute::from(request_route);
    if request_route != HidRoute::None {
        request_route
    } else {
        router.preferred_custom_route()
    }
}

#[cfg(test)]
#[cfg_attr(coverage, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn parse_frame_too_short() {
        let result = parse_frame(&[0xA5]);
        assert_eq!(result, Err(ParseError::TooShort));
    }

    #[test]
    fn parse_frame_bad_magic() {
        let mut buf = [0u8; CUSTOM_PROTOCOL_HEADER_LEN];
        buf[0] = 0x00;
        buf[1] = CUSTOM_PROTOCOL_VERSION;
        assert_eq!(parse_frame(&buf), Err(ParseError::BadMagic));
    }

    #[test]
    fn parse_frame_unsupported_version() {
        let mut buf = [0u8; CUSTOM_PROTOCOL_HEADER_LEN + 4];
        buf[0] = CUSTOM_PROTOCOL_MAGIC;
        buf[1] = 99;
        assert_eq!(parse_frame(&buf), Err(ParseError::UnsupportedVersion));
    }

    #[test]
    fn parse_frame_valid_empty_payload() {
        let mut buf = [0u8; CUSTOM_PROTOCOL_HEADER_LEN];
        buf[0] = CUSTOM_PROTOCOL_MAGIC;
        buf[1] = CUSTOM_PROTOCOL_VERSION;
        let result = parse_frame(&buf);
        assert!(result.is_ok());
        let frame = result.unwrap();
        assert_eq!(frame.total_len, 0);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn parse_frame_larger_buffer_ok() {
        // USB 总是传完整的 endpoint 大小（如 64 字节），
        // 但 frame 可能只有 16 字节（header + 空 payload）
        let mut buf = [0u8; 64];
        buf[0] = CUSTOM_PROTOCOL_MAGIC;
        buf[1] = CUSTOM_PROTOCOL_VERSION;
        buf[2] = CUSTOM_FLAG_REQUEST;
        buf[3] = 1;
        // 未显式赋值字段已由 `[0u8; 64]` 初始化为 0
        let result = parse_frame(&buf);
        assert!(result.is_ok());
        let frame = result.unwrap();
        assert_eq!(frame.sequence, 1);
        assert_eq!(frame.command, CUSTOM_CMD_PING);
        assert_eq!(frame.total_len, 0);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn encode_frame_payload_too_large() {
        let header = CustomFrameHeader {
            flags: CUSTOM_FLAG_RESPONSE,
            sequence: 0,
            command: 0,
            status: CUSTOM_STATUS_OK,
            offset: 0,
            total_len: 9999,
        };
        let huge = [0u8; CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN + 1];
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_frame(header, &huge, &mut out);
        assert_eq!(result, Err(ParseError::PayloadTooLarge));
    }

    #[test]
    fn build_protocol_info() {
        let payload = build_protocol_info_payload();
        assert_eq!(payload[0], CUSTOM_PROTOCOL_VERSION);
        assert_eq!(payload[1], CUSTOM_PROTOCOL_HEADER_LEN as u8);
    }

    #[test]
    fn preferred_response_route_respects_request() {
        let router = crate::route::HidRouter::new();
        assert_eq!(preferred_response_route(&router, 3), HidRoute::Usb);
    }

    #[test]
    fn preferred_response_route_falls_back() {
        let router = crate::route::HidRouter::new();
        assert_eq!(preferred_response_route(&router, 0), HidRoute::None);
    }

    #[test]
    fn parse_write_begin_ok() {
        let payload = [0x10, 0x00, 0x00, 0x00, 0xEF, 0xBE, 0xAD, 0xDE];
        let (len, crc) = parse_write_begin_payload(&payload).unwrap();
        assert_eq!(len, 16);
        assert_eq!(crc, 0xDEAD_BEEF);
    }

    #[test]
    fn parse_write_begin_bad_length() {
        assert_eq!(
            parse_write_begin_payload(&[0; 4]),
            Err(CUSTOM_STATUS_BAD_LENGTH)
        );
    }

    #[test]
    fn parse_write_chunk_ok() {
        let payload = [0x05, 0x00, 0x00, 0x00, 0xAA, 0xBB];
        let (offset, data) = parse_write_chunk_payload(&payload).unwrap();
        assert_eq!(offset, 5);
        assert_eq!(data, &[0xAA, 0xBB]);
    }

    #[test]
    fn parse_write_chunk_bad_length() {
        assert_eq!(
            parse_write_chunk_payload(&[0; 2]),
            Err(CUSTOM_STATUS_BAD_LENGTH)
        );
    }

    #[test]
    fn map_config_error_categories() {
        use crate::config::ConfigError;
        assert_eq!(
            map_config_error_to_status(ConfigError::WriteSessionBusy),
            CUSTOM_STATUS_BUSY
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::WriteSessionNotActive),
            CUSTOM_STATUS_BAD_SEQUENCE
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::WriteSequenceMismatch),
            CUSTOM_STATUS_BAD_SEQUENCE
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::PayloadCrcMismatch),
            CUSTOM_STATUS_CRC_MISMATCH
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::InvalidPayloadLength),
            CUSTOM_STATUS_BAD_LENGTH
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::ValidationFailed),
            CUSTOM_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::StorageUnavailable),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::StorageEmpty),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::InvalidFlashRegion),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::FlashEraseFailed),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::FlashWriteFailed),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::ReadbackVerifyFailed),
            CUSTOM_STATUS_STORAGE_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::EncodeFailed),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::PayloadTooLarge),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::DeserializeFailed),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::HeaderCrcMismatch),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::InvalidMagic),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::UnsupportedStorageVersion),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::UnsupportedConfigVersion),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
        assert_eq!(
            map_config_error_to_status(ConfigError::MigrationFailed),
            CUSTOM_STATUS_INTERNAL_ERROR
        );
    }

    #[test]
    fn encode_response_success() {
        let payload = [0x01, 0x02];
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_response(0x0100, 5, CUSTOM_STATUS_OK, &payload, &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[0], CUSTOM_PROTOCOL_MAGIC);
        assert_eq!(out.data[1], CUSTOM_PROTOCOL_VERSION);
        assert_eq!(out.data[3], 5);
    }

    #[test]
    fn encode_error_response_success() {
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_error_response(0x0100, 3, CUSTOM_STATUS_INVALID_COMMAND, &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[3], 3);
        assert_eq!(
            u16::from_le_bytes([out.data[6], out.data[7]]),
            CUSTOM_STATUS_INVALID_COMMAND
        );
    }

    #[test]
    fn build_route_state_payload_works() {
        let router = crate::route::HidRouter::new();
        let payload = build_route_state_payload(&router);
        assert_eq!(payload.len(), 5);
    }

    #[test]
    fn build_diagnostics_payload_works() {
        let stats = ProtocolStats {
            rx_ok: 1,
            rx_invalid: 2,
            rx_unsupported: 3,
            tx_generated: 4,
            tx_dropped_no_route: 5,
        };
        let rx_stats = VendorRxStats {
            dropped: 10,
            too_large: 20,
        };
        let payload = build_diagnostics_payload(stats, rx_stats);
        assert_eq!(payload.len(), 14);
        assert_eq!(payload[0], 1);
        assert_eq!(payload[2], 2);
    }

    #[test]
    fn encode_frame_empty_payload_success() {
        let header = CustomFrameHeader {
            flags: CUSTOM_FLAG_RESPONSE,
            sequence: 1,
            command: 0x0100,
            status: CUSTOM_STATUS_OK,
            offset: 0,
            total_len: 0,
        };
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_frame(header, &[], &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[0], CUSTOM_PROTOCOL_MAGIC);
        assert_eq!(out.data[1], CUSTOM_PROTOCOL_VERSION);
        assert_eq!(out.data[2], CUSTOM_FLAG_RESPONSE);
        assert_eq!(out.data[3], 1);
        // 协议头字段：command / status / offset / total_len / payload_len 均为 0
        assert_eq!(out.data[4], 0x00);
        assert_eq!(out.data[5], 0x01);
        assert_eq!(out.data[6], 0x00);
        assert_eq!(out.data[7], 0x00);
        assert_eq!(out.data[8], 0x00);
        assert_eq!(out.data[9], 0x00);
        assert_eq!(out.data[10], 0x00);
        assert_eq!(out.data[11], 0x00);
        assert_eq!(out.data[12], 0x00);
        assert_eq!(out.data[13], 0x00);
        assert_eq!(out.data[14], 0x00);
        assert_eq!(out.data[15], 0x00);
        // 末尾填充到 64 字节（USB report 大小）
        assert_eq!(out.len, 64);
        for i in 16..64 {
            assert_eq!(out.data[i], 0);
        }
    }

    #[test]
    fn encode_frame_with_payload_success() {
        let header = CustomFrameHeader {
            flags: CUSTOM_FLAG_RESPONSE,
            sequence: 2,
            command: 0x0001,
            status: CUSTOM_STATUS_OK,
            offset: 0,
            total_len: 4,
        };
        let payload = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_frame(header, &payload, &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[16], 0xDE);
        assert_eq!(out.data[17], 0xAD);
        assert_eq!(out.data[18], 0xBE);
        assert_eq!(out.data[19], 0xEF);
        assert_eq!(out.data[10], 0x04);
        assert_eq!(out.data[11], 0x00);
        assert_eq!(out.data[14], 0x04);
        assert_eq!(out.data[15], 0x00);
    }

    #[test]
    fn encode_response_empty_payload() {
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_response(0x0201, 7, CUSTOM_STATUS_OK, &[], &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[3], 7);
        assert_eq!(out.data[0], CUSTOM_PROTOCOL_MAGIC);
        assert_eq!(out.data[2], CUSTOM_FLAG_RESPONSE);
    }

    #[test]
    fn build_protocol_info_format() {
        let p = build_protocol_info_payload();
        assert_eq!(p[0], CUSTOM_PROTOCOL_VERSION);
        assert_eq!(p[1] as usize, CUSTOM_PROTOCOL_HEADER_LEN);
        assert_eq!(p[2] as usize, CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN);
        assert_eq!(p[3], 0);
    }

    #[test]
    fn build_route_state_payload_detailed() {
        let mut router = crate::route::HidRouter::new();
        router.set_usb_state(crate::route::UsbState::Configured);
        let payload = build_route_state_payload(&router);
        assert_eq!(payload[0], router.preferred_mouse_route().as_ffi());
        assert_eq!(payload[3], 1);
    }

    #[test]
    fn encode_error_response_format() {
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        let result = encode_error_response(0x0100, 9, CUSTOM_STATUS_INVALID_ARGUMENT, &mut out);
        assert!(result.is_ok());
        assert_eq!(out.data[3], 9);
        let status = u16::from_le_bytes([out.data[6], out.data[7]]);
        assert_eq!(status, CUSTOM_STATUS_INVALID_ARGUMENT);
    }

    #[test]
    fn encode_frame_huge_payload_fails() {
        let header = CustomFrameHeader {
            flags: CUSTOM_FLAG_RESPONSE,
            sequence: 0,
            command: 0,
            status: 0,
            offset: 0,
            total_len: 0,
        };
        let big = [0u8; CUSTOM_PROTOCOL_MAX_PAYLOAD_LEN + 1];
        let mut out = CustomReport {
            data: [0u8; 64],
            len: 0,
        };
        assert_eq!(
            encode_frame(header, &big, &mut out),
            Err(ParseError::PayloadTooLarge)
        );
    }
}

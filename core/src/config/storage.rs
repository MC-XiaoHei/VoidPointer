use crate::config::types::SlotHeader;

const CRC32_POLY_REVERSED: u32 = 0xEDB8_8320;

pub fn slot_header_encode(header: SlotHeader, out: &mut [u8; SlotHeader::ENCODED_LEN]) {
    out[0..4].copy_from_slice(&header.magic.to_le_bytes());
    out[4..6].copy_from_slice(&header.storage_version.to_le_bytes());
    out[6..8].copy_from_slice(&header.config_version.to_le_bytes());
    out[8..12].copy_from_slice(&header.payload_len.to_le_bytes());
    out[12..16].copy_from_slice(&header.sequence.to_le_bytes());
    out[16..20].copy_from_slice(&header.payload_crc32.to_le_bytes());
    out[20..24].copy_from_slice(&header.header_crc32.to_le_bytes());
    out[24..28].copy_from_slice(&header.flags.to_le_bytes());
}

pub fn slot_header_decode(bytes: &[u8; SlotHeader::ENCODED_LEN]) -> SlotHeader {
    SlotHeader {
        magic: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        storage_version: u16::from_le_bytes([bytes[4], bytes[5]]),
        config_version: u16::from_le_bytes([bytes[6], bytes[7]]),
        payload_len: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        sequence: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
        payload_crc32: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        header_crc32: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
        flags: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
    }
}

/// 将 header 各字段按 0 填充自身 CRC 字段后计算 CRC32
pub fn compute_header_crc32(header: SlotHeader) -> u32 {
    let mut bytes = [0u8; SlotHeader::ENCODED_LEN];
    let mut header_no_crc = header;
    header_no_crc.header_crc32 = 0;
    slot_header_encode(header_no_crc, &mut bytes);
    crc32(&bytes)
}

/// 填入 header_crc32 并返回"已封口"的 header
pub fn seal_header(mut header: SlotHeader) -> SlotHeader {
    header.header_crc32 = 0;
    header.header_crc32 = compute_header_crc32(header);
    header
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;

    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (CRC32_POLY_REVERSED & mask);
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"hello"), 0x3610a686);
    }

    #[test]
    fn crc32_empty_input() {
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    #[test]
    fn header_encode_decode_roundtrip() {
        let h = SlotHeader {
            magic: crate::config::SLOT_MAGIC,
            storage_version: 1,
            config_version: 2,
            payload_len: 100,
            sequence: 42,
            payload_crc32: 0x1234_5678,
            header_crc32: 0x8765_4321,
            flags: 0xFF,
        };
        let mut buf = [0u8; SlotHeader::ENCODED_LEN];
        slot_header_encode(h, &mut buf);
        assert_eq!(slot_header_decode(&buf), h);
    }

    #[test]
    fn seal_self_consistent() {
        let h = SlotHeader {
            magic: crate::config::SLOT_MAGIC,
            storage_version: 1,
            config_version: 1,
            payload_len: 50,
            sequence: 1,
            payload_crc32: 0xDEAD_BEEF,
            header_crc32: 0,
            flags: 0,
        };
        let sealed = seal_header(h);
        assert_eq!(compute_header_crc32(sealed), sealed.header_crc32);
    }
}

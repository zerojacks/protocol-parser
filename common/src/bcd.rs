//! BCD（二进制编码十进制）编解码工具。
//!
//! 用于帧层字段，如地址域 A1（省地市区县码）、数据时间域、时间标签 Tp 等，
//! 与 `spec-engine` 内容层自己的 BCD 解码（`decode_bcd_u64` 等，用于 DI 内容字段）互相独立，
//! 两边各管各的字节范围，不共享状态。

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BcdError {
    #[error("非法BCD半字节: 0x{0:02X}（每个半字节应为0~9）")]
    InvalidNibble(u8),
}

/// 将 BCD 字节解码为无符号整数。
///
/// 按标准里"低字节在前、高字节在后"的传输顺序，`bytes[0]` 是最低有效字节。
/// 每个字节的高 4 位是十进制的十位、低 4 位是个位（大端半字节序，如 0x23 -> 23）。
pub fn decode(bytes: &[u8]) -> Result<u64, BcdError> {
    let mut value: u64 = 0;
    for (i, &b) in bytes.iter().enumerate().rev() {
        let hi = b >> 4;
        let lo = b & 0x0F;
        if hi > 9 || lo > 9 {
            return Err(BcdError::InvalidNibble(b));
        }
        let _ = i;
        value = value * 100 + (hi as u64) * 10 + lo as u64;
    }
    Ok(value)
}

/// 将无符号整数编码为指定长度的 BCD 字节（低字节在前）。
///
/// `value` 每两位十进制数字对应一个输出字节；若 `value` 用给定 `len` 装不下，返回 `None`。
pub fn encode(value: u64, len: usize) -> Option<Vec<u8>> {
    let mut digits = value;
    let mut bytes = Vec::with_capacity(len);
    for _ in 0..len {
        let byte_value = digits % 100;
        let hi = (byte_value / 10) as u8;
        let lo = (byte_value % 10) as u8;
        bytes.push((hi << 4) | lo);
        digits /= 100;
    }
    if digits != 0 {
        // 剩余的数字装不进 len 个字节
        return None;
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_matches_natural_order() {
        // 0x23 0x01 按"低字节在前"读作 0123 -> 123
        assert_eq!(decode(&[0x23, 0x01]).unwrap(), 123);
    }

    #[test]
    fn decode_rejects_invalid_nibble() {
        assert_eq!(decode(&[0xAB]), Err(BcdError::InvalidNibble(0xAB)));
    }

    #[test]
    fn encode_roundtrips_with_decode() {
        let encoded = encode(123, 2).unwrap();
        assert_eq!(encoded, vec![0x23, 0x01]);
        assert_eq!(decode(&encoded).unwrap(), 123);
    }

    #[test]
    fn encode_rejects_overflow() {
        assert_eq!(encode(12345, 2), None);
    }
}

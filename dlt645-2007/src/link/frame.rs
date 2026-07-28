//! DL/T 645-2007 链路层帧解析
//!
//! 链路层只负责：
//! - 帧起始/结束标识
//! - 地址域和控制码
//! - 数据长度和校验和
//! - 数据偏移（+33H/-33H）
//!
//! 数据域的具体内容解析交由应用层处理

use crate::address::Address;
use crate::control::ControlCode;
use crate::error::{Error, Result};
use crate::{FRAME_END, FRAME_START, PREAMBLE};

/// DL/T 645-2007 链路层帧
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// 地址域（6字节）
    pub address: Address,
    /// 控制码
    pub control: ControlCode,
    /// 数据域原始字节（已去除 +33H 偏移）
    pub payload: Vec<u8>,
}

impl Frame {
    /// 最小帧长度（不含前导字节和数据域）
    /// 68H(1) + 地址(6) + 68H(1) + C(1) + L(1) + CS(1) + 16H(1) = 12 字节
    pub const MIN_LEN: usize = 12;

    /// 最大数据长度
    pub const MAX_DATA_LEN: usize = 200;

    /// 从字节切片解码帧
    ///
    /// # 参数
    ///
    /// - `buf`: 包含完整帧的字节切片（可以包含前导字节）
    ///
    /// # 返回
    ///
    /// - `Ok((Frame, consumed))`: 成功解析的帧和消耗的字节数
    /// - `Err(Error)`: 解析错误
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        let original_len = buf.len();
        let mut buf = buf;

        // 跳过前导字节（FE FE FE FE）
        while !buf.is_empty() && buf[0] == PREAMBLE {
            buf = &buf[1..];
        }

        let skipped = original_len - buf.len();

        // 检查最小长度
        if buf.len() < Self::MIN_LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::MIN_LEN,
                actual: buf.len(),
            });
        }

        // 验证第一个起始符
        if buf[0] != FRAME_START {
            return Err(Error::InvalidFrameStart {
                expected: FRAME_START,
                actual: buf[0],
            });
        }

        // 验证第二个起始符（位于第8个字节）
        if buf[7] != FRAME_START {
            return Err(Error::InvalidFrameStart {
                expected: FRAME_START,
                actual: buf[7],
            });
        }

        // 解析地址域（字节1-6）
        let (address, _) = Address::decode(&buf[1..])?;

        // 解析控制码（字节8）
        let control = ControlCode::from_byte(buf[8])?;

        // 解析数据长度（字节9）
        let data_len = buf[9] as usize;

        // 检查数据长度限制
        if data_len > Self::MAX_DATA_LEN {
            return Err(Error::DataLengthExceeded {
                max: Self::MAX_DATA_LEN,
                actual: data_len,
            });
        }

        // 计算完整帧长度
        let frame_len = 10 + data_len + 2; // 10(头部) + data_len + 2(CS+16H)

        if buf.len() < frame_len {
            return Err(Error::UnexpectedEof {
                needed: frame_len,
                actual: buf.len(),
            });
        }

        // 验证结束符
        if buf[frame_len - 1] != FRAME_END {
            return Err(Error::InvalidFrameEnd {
                expected: FRAME_END,
                actual: buf[frame_len - 1],
            });
        }

        // 计算并验证校验和
        let expected_cs = calculate_checksum(&buf[0..frame_len - 2]);
        let actual_cs = buf[frame_len - 2];
        if expected_cs != actual_cs {
            return Err(Error::ChecksumMismatch {
                expected: expected_cs,
                actual: actual_cs,
            });
        }

        // 提取数据域（字节10 到 10+data_len-1）
        let data_with_offset = &buf[10..10 + data_len];
        
        // 移除 +33H 偏移
        let payload = remove_offset(data_with_offset);

        let frame = Self {
            address,
            control,
            payload,
        };

        Ok((frame, skipped + frame_len))
    }

    /// 编码为字节序列
    ///
    /// # 参数
    ///
    /// - `include_preamble`: 是否包含前导字节（4个 FE）
    pub fn encode(&self, include_preamble: bool) -> Vec<u8> {
        let mut bytes = Vec::new();

        // 前导字节（可选）
        if include_preamble {
            bytes.extend_from_slice(&[PREAMBLE; 4]);
        }

        // 记录帧开始位置（用于计算校验和）
        let frame_start = bytes.len();

        // 第一个起始符
        bytes.push(FRAME_START);

        // 地址域
        bytes.extend_from_slice(self.address.as_bytes());

        // 第二个起始符
        bytes.push(FRAME_START);

        // 控制码
        bytes.push(self.control.to_byte());

        // 应用 +33H 偏移到数据域
        let data_with_offset = apply_offset(&self.payload);
        let data_len = data_with_offset.len() as u8;

        // 数据长度
        bytes.push(data_len);

        // 数据域
        bytes.extend_from_slice(&data_with_offset);

        // 校验和（从第一个68H到数据域末尾，不包含前导字节）
        let cs = calculate_checksum(&bytes[frame_start..]);
        bytes.push(cs);

        // 结束符
        bytes.push(FRAME_END);

        bytes
    }

    /// 获取帧长度（不含前导字节）
    pub fn len(&self) -> usize {
        // 68H(1) + 地址(6) + 68H(1) + C(1) + L(1) + DATA(L) + CS(1) + 16H(1)
        12 + self.payload.len()
    }

    /// 判断数据域是否为空
    pub fn is_empty(&self) -> bool {
        self.payload.is_empty()
    }
}

/// 计算校验和（累加和 mod 256）
///
/// 从第一个 68H 开始，到数据域结束（不包含 CS 和 16H）
fn calculate_checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// 对数据域应用 +33H 偏移
fn apply_offset(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b.wrapping_add(0x33)).collect()
}

/// 移除数据域的 +33H 偏移
fn remove_offset(data: &[u8]) -> Vec<u8> {
    data.iter().map(|&b| b.wrapping_sub(0x33)).collect()
}

/// 在缓冲区中查找下一个有效帧的起始位置
///
/// 返回第一个 68H 的位置，如果找不到则返回 None
pub fn find_frame_start(buf: &[u8]) -> Option<usize> {
    // 跳过前导字节
    let mut pos = 0;
    while pos < buf.len() && buf[pos] == PREAMBLE {
        pos += 1;
    }

    // 查找第一个 68H
    for i in pos..buf.len() {
        if buf[i] == FRAME_START {
            // 验证第二个 68H（如果有足够字节）
            if i + 7 < buf.len() && buf[i + 7] == FRAME_START {
                return Some(i);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::FunctionCode;

    #[test]
    fn test_checksum() {
        let bytes = vec![0x68, 0x01, 0x02, 0x03];
        let cs = calculate_checksum(&bytes);
        assert_eq!(cs, 0x68 + 0x01 + 0x02 + 0x03);
    }

    #[test]
    fn test_frame_decode_with_payload() {
        // 构造一个简单的帧
        let mut bytes = vec![
            0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x68,
            0x11, 0x04, 0x33, 0x33, 0x34, 0x33, // 数据: 00 00 01 00 (DI) + 33H
            0x00, 0x16, // CS和结束符（先占位）
        ];

        // 计算正确的校验和
        let len = bytes.len();
        let cs = calculate_checksum(&bytes[0..len - 2]);
        bytes[len - 2] = cs;

        let (frame, consumed) = Frame::decode(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(frame.address.to_decimal_string(), "123456789012");
        assert_eq!(frame.control.function, FunctionCode::ReadData);
        assert_eq!(frame.payload, vec![0x00, 0x00, 0x01, 0x00]); // 已去除+33H
    }

    #[test]
    fn test_frame_encode_decode_roundtrip() {
        let address = Address::from_decimal_str("000000000001").unwrap();
        let control = ControlCode::master_request(FunctionCode::ReadData);
        let payload = vec![0x00, 0x00, 0x01, 0x00]; // 原始数据

        let frame = Frame {
            address,
            control,
            payload,
        };

        let encoded = frame.encode(false);
        let (decoded, _) = Frame::decode(&encoded).unwrap();

        assert_eq!(decoded.address.to_decimal_string(), frame.address.to_decimal_string());
        assert_eq!(decoded.control.to_byte(), frame.control.to_byte());
        assert_eq!(decoded.payload, frame.payload);
    }

    #[test]
    fn test_frame_with_preamble() {
        let address = Address::from_decimal_str("123456789012").unwrap();
        let control = ControlCode::master_request(FunctionCode::ReadData);
        let payload = vec![0x00, 0x01, 0x00, 0x00];

        let frame = Frame {
            address,
            control,
            payload,
        };

        let with_preamble = frame.encode(true);
        let without_preamble = frame.encode(false);

        // 带前导的应该多4字节
        assert_eq!(with_preamble.len(), without_preamble.len() + 4);

        // 两种都能解码
        let (decoded1, _) = Frame::decode(&with_preamble).unwrap();
        let (decoded2, _) = Frame::decode(&without_preamble).unwrap();

        assert_eq!(decoded1.payload, decoded2.payload);
    }

    #[test]
    fn test_find_frame_start() {
        let bytes = vec![
            0xFE, 0xFE, 0xFE, 0xFE,
            0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x68,
        ];

        let pos = find_frame_start(&bytes);
        assert_eq!(pos, Some(4));
    }
}

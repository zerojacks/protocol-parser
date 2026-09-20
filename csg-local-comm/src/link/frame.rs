//! 链路层帧格式解析
//!
//! 帧格式：
//! ```text
//! 68H | L(2B) | C(1B) | [A(12B)] | 用户数据区 | CS(1B) | 16H
//! ```
//!
//! - **68H**: 起始符（固定）
//! - **L**: 帧长度（2字节 BIN，小端），L = 整个帧的字节总数
//! - **C**: 控制字节（1字节）
//! - **A**: 地址域（12字节，仅当 C.ADD=1 时存在）
//! - **用户数据区**: AFN + SEQ + DI + 数据标识内容
//! - **CS**: 校验和（1字节），C + 用户数据区的算术和（mod 256）
//! - **16H**: 结束符（固定）

use crate::control::ControlByte;
use crate::error::{Error, Result};
use crate::link::address::AddressDomain;
use proto_common::checksum;

/// 起始符
const START_CHAR: u8 = 0x68;
/// 结束符
const END_CHAR: u8 = 0x16;

/// 链路层帧
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// 控制字节
    pub control: ControlByte,
    /// 地址域（可选，当 control.has_address = true 时存在）
    pub address: Option<AddressDomain>,
    /// 用户数据区（AFN + SEQ + DI + 数据标识内容）
    pub payload: Vec<u8>,
    /// 校验和
    pub checksum: u8,
}

impl Frame {
    /// 从字节流解码帧，返回帧和消耗的字节数
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < 6 {
            return Err(Error::UnexpectedEof {
                needed: 6,
                actual: buf.len(),
            });
        }

        // 检查起始符
        if buf[0] != START_CHAR {
            return Err(Error::InvalidStartChar { found: buf[0] });
        }

        // 读取帧长度 L（2字节，小端）
        let frame_len = u16::from_le_bytes([buf[1], buf[2]]) as usize;

        // 检查缓冲区是否足够
        if buf.len() < frame_len {
            return Err(Error::UnexpectedEof {
                needed: frame_len,
                actual: buf.len(),
            });
        }

        // 检查结束符
        if buf[frame_len - 1] != END_CHAR {
            return Err(Error::InvalidEndChar {
                found: buf[frame_len - 1],
            });
        }

        // 解析控制字节
        let control = ControlByte::from_byte(buf[3])?;

        let mut offset = 4; // 跳过 68H + L(2B) + C(1B)

        // 解析地址域（如果存在）
        let address = if control.has_address {
            if offset + AddressDomain::LEN > frame_len {
                return Err(Error::UnexpectedEof {
                    needed: offset + AddressDomain::LEN,
                    actual: frame_len,
                });
            }
            let addr = AddressDomain::decode(&buf[offset..offset + AddressDomain::LEN])?;
            offset += AddressDomain::LEN;
            Some(addr)
        } else {
            None
        };

        // 用户数据区 = 从 offset 到 CS 之前
        let cs_index = frame_len - 2; // CS 在倒数第二个位置
        if offset >= cs_index {
            return Err(Error::InvalidDataFormat {
                reason: "用户数据区长度为 0".to_string(),
            });
        }

        let payload = buf[offset..cs_index].to_vec();

        // 验证校验和
        let expected_cs = buf[cs_index];
        let actual_cs = Self::calculate_checksum(&buf[3..cs_index]); // C + 用户数据区

        if expected_cs != actual_cs {
            return Err(Error::ChecksumMismatch {
                expected: expected_cs,
                actual: actual_cs,
            });
        }

        Ok((
            Self {
                control,
                address,
                payload,
                checksum: expected_cs,
            },
            frame_len,
        ))
    }

    /// 编码为字节数组
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        // 68H
        buf.push(START_CHAR);

        // 计算帧长度
        let addr_len = if self.address.is_some() {
            AddressDomain::LEN
        } else {
            0
        };
        let frame_len = 1 + 2 + 1 + addr_len + self.payload.len() + 1 + 1;
        // = 68H + L(2) + C(1) + [A(12)] + payload + CS(1) + 16H

        // L（2字节，小端）
        let len_bytes = (frame_len as u16).to_le_bytes();
        buf.extend_from_slice(&len_bytes);

        // C
        let control_byte = self.control.to_byte();
        buf.push(control_byte);

        // A（如果存在）
        if let Some(addr) = &self.address {
            buf.extend_from_slice(&addr.encode());
        }

        // 用户数据区
        buf.extend_from_slice(&self.payload);

        // CS（C + 用户数据区的校验和）
        let cs = self.calculated_checksum();
        buf.push(cs);

        // 16H
        buf.push(END_CHAR);

        Ok(buf)
    }

    /// 计算当前帧的校验和：控制字节、地址域和用户数据区的算术和（mod 256）。
    fn calculated_checksum(&self) -> u8 {
        let control = [self.control.to_byte()];
        let address = self
            .address
            .as_ref()
            .map(AddressDomain::encode)
            .unwrap_or_default();

        let mut data = Vec::with_capacity(control.len() + address.len() + self.payload.len());
        data.extend_from_slice(&control);
        data.extend_from_slice(&address);
        data.extend_from_slice(&self.payload);
        Self::calculate_checksum(&data)
    }

    /// 计算校验和：C + 用户数据区的算术和（mod 256）
    fn calculate_checksum(data: &[u8]) -> u8 {
        checksum::arithmetic_sum(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::Direction;

    #[test]
    fn test_frame_decode_without_address() {
        // 构造一个简单的测试帧（无地址域）
        // 帧长度 = 1(68) + 2(L) + 1(C) + 3(payload) + 1(CS) + 1(16) = 9
        let frame_bytes = vec![
            0x68, // 起始符
            0x09, 0x00, // L = 9
            0x40, // C: 下行, 启动站, 无地址域
            0x01, 0x02, 0x03, // 用户数据区（3字节）
            0x46, // CS = 0x40 + 0x01 + 0x02 + 0x03 = 0x46
            0x16, // 结束符
        ];

        let (frame, consumed) = Frame::decode(&frame_bytes).unwrap();
        assert_eq!(consumed, 9);
        assert_eq!(frame.control.direction, Direction::Downlink);
        assert!(frame.control.is_primary);
        assert!(!frame.control.has_address);
        assert_eq!(frame.address, None);
        assert_eq!(frame.payload, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_frame_decode_with_address() {
        // 帧长度 = 1(68) + 2(L) + 1(C) + 12(addr) + 3(payload) + 1(CS) + 1(16) = 21
        // CS = 0x60 + 所有地址字节 + payload 字节
        // CS = 0x60 + 0x01 + 0x02 + 0x03 + 0x04 + 0x05 + 0x06 + 0x07 + 0x08 + 0x09 + 0x0A + 0x0B + 0x0C + 0x01 + 0x02 + 0x03
        // CS = 0x60 + 0x51 + 0x06 = 0xB7
        let frame_bytes = vec![
            0x68, // 起始符
            0x15, 0x00, // L = 21
            0x60, // C: 下行, 启动站, 有地址域
            // ASRC (6字节)
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // ADST (6字节)
            0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, // 用户数据区（3字节）
            0x01, 0x02, 0x03, 0xB4, // CS
            0x16, // 结束符
        ];

        let (frame, consumed) = Frame::decode(&frame_bytes).unwrap();
        assert_eq!(consumed, 21);
        assert!(frame.control.has_address);
        assert!(frame.address.is_some());
        assert_eq!(frame.payload, vec![0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_frame_encode_decode_roundtrip() {
        use crate::link::address::Address;

        let frame = Frame {
            control: ControlByte::downlink_primary_with_address(),
            address: Some(AddressDomain {
                source: Address::from_bytes([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]),
                destination: Address::from_bytes([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
            }),
            payload: vec![0x01, 0x02, 0x03, 0x04],
            checksum: 0xCA,
        };

        let bytes = frame.encode().unwrap();
        let (frame2, _) = Frame::decode(&bytes).unwrap();
        assert_eq!(frame, frame2);
    }

    #[test]
    fn test_invalid_start_char() {
        let bytes = vec![0x69, 0x0A, 0x00, 0x40, 0x01, 0x02, 0x03, 0x46, 0x16];
        let result = Frame::decode(&bytes);
        assert!(matches!(result, Err(Error::InvalidStartChar { .. })));
    }

    #[test]
    fn test_checksum_mismatch() {
        let bytes = vec![
            0x68, 0x09, 0x00, 0x40, 0x01, 0x02, 0x03, 0xFF, // 错误的校验和
            0x16,
        ];
        let result = Frame::decode(&bytes);
        assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
    }
}

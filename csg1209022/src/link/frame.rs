//! 链路层帧格式（标准 5.1.1~5.1.5，FT1.2 异步式传输帧）。
//!
//! ```text
//! 68H | 长度L(2字节) | 长度L(重复) | 68H | 控制域C | 地址域A | 链路用户数据(应用层) | 校验和CS | 16H
//! ```
//!
//! - 长度L：BIN编码，2字节低字节在前，表示"控制域+地址域+应用层"的字节总数，两次出现的L必须一致。
//! - 校验和CS：用户数据区（控制域+地址域+应用层）所有字节的算术和，不考虑进位。
//! - 帧头帧尾字符固定为 `0x68` / `0x16`。

use crate::error::{ProtoError, Result};
use crate::link::address::AddressField;
use crate::link::control::ControlField;

pub const START_END_MARKER_HEAD: u8 = 0x68;
pub const END_MARKER: u8 = 0x16;

/// 固定帧头长度：68H + L(2) + L(2) + 68H
const FIXED_HEADER_LEN: usize = 6;
/// 帧尾长度：CS(1) + 16H(1)
const FIXED_TRAILER_LEN: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub control: ControlField,
    pub address: AddressField,
    /// 应用层原始字节，不做进一步解析——交给 `app` 模块继续处理
    pub payload: Vec<u8>,
}

impl Frame {
    /// 从字节流解码一帧，返回解码结果和消耗的字节数（便于流式/半包场景继续从后面接着解析）。
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < FIXED_HEADER_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: FIXED_HEADER_LEN,
                actual: buf.len(),
            });
        }
        if buf[0] != START_END_MARKER_HEAD || buf[5] != START_END_MARKER_HEAD {
            return Err(ProtoError::InvalidFrameMarker);
        }
        let l1 = u16::from_le_bytes([buf[1], buf[2]]);
        let l2 = u16::from_le_bytes([buf[3], buf[4]]);
        if l1 != l2 {
            return Err(ProtoError::LengthMismatch {
                first: l1,
                second: l2,
            });
        }
        let user_data_len = l1 as usize;
        let total_len = FIXED_HEADER_LEN + user_data_len + FIXED_TRAILER_LEN;
        if buf.len() < total_len {
            return Err(ProtoError::UnexpectedEof {
                needed: total_len,
                actual: buf.len(),
            });
        }

        let user_data = &buf[FIXED_HEADER_LEN..FIXED_HEADER_LEN + user_data_len];
        let cs_byte = buf[FIXED_HEADER_LEN + user_data_len];
        let end_marker = buf[FIXED_HEADER_LEN + user_data_len + 1];
        if end_marker != END_MARKER {
            return Err(ProtoError::InvalidFrameMarker);
        }
        let actual_cs = proto_common::checksum::arithmetic_sum(user_data);
        if actual_cs != cs_byte {
            return Err(ProtoError::ChecksumMismatch {
                expected: cs_byte,
                actual: actual_cs,
            });
        }

        if user_data.is_empty() {
            return Err(ProtoError::UnexpectedEof {
                needed: 1,
                actual: 0,
            });
        }
        let control = ControlField::decode(user_data[0]);
        let (address, addr_consumed) = AddressField::decode(&user_data[1..])?;
        let payload = user_data[1 + addr_consumed..].to_vec();

        Ok((
            Self {
                control,
                address,
                payload,
            },
            total_len,
        ))
    }

    /// 将本帧编码为完整的链路层字节序列（含帧头帧尾与校验和）。
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut user_data =
            Vec::with_capacity(1 + crate::link::address::ADDRESS_LEN + self.payload.len());
        user_data.push(self.control.encode());
        user_data.extend_from_slice(&self.address.encode()?);
        user_data.extend_from_slice(&self.payload);

        let l = user_data.len() as u16;
        let cs = proto_common::checksum::arithmetic_sum(&user_data);

        let mut out = Vec::with_capacity(FIXED_HEADER_LEN + user_data.len() + FIXED_TRAILER_LEN);
        out.push(START_END_MARKER_HEAD);
        out.extend_from_slice(&l.to_le_bytes());
        out.extend_from_slice(&l.to_le_bytes());
        out.push(START_END_MARKER_HEAD);
        out.extend_from_slice(&user_data);
        out.push(cs);
        out.push(END_MARKER);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::address::RegionCode;
    use crate::link::control::Direction;

    fn sample_frame() -> Frame {
        Frame {
            control: ControlField::new_up(false, false, false, 8).unwrap(),
            address: AddressField {
                region: RegionCode {
                    province: 44,
                    city: 1,
                    county: 5,
                },
                terminal_addr: 123456,
                master_addr: 1,
            },
            payload: vec![0x0C, 0x00, 0x01, 0x02, 0x03],
        }
    }

    #[test]
    fn roundtrip_encode_decode() {
        let frame = sample_frame();
        let bytes = frame.encode().unwrap();
        let (decoded, consumed) = Frame::decode(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded, frame);
        assert_eq!(decoded.control.direction, Direction::Up);
    }

    #[test]
    fn decode_stops_after_this_frame_even_with_trailing_bytes() {
        let frame = sample_frame();
        let mut bytes = frame.encode().unwrap();
        bytes.extend_from_slice(&[0xAA, 0xBB, 0xCC]); // 模拟流里紧跟着下一帧
        let (decoded, consumed) = Frame::decode(&bytes).unwrap();
        assert_eq!(decoded, frame);
        assert!(consumed < bytes.len());
    }

    #[test]
    fn rejects_mismatched_length_fields() {
        let frame = sample_frame();
        let mut bytes = frame.encode().unwrap();
        bytes[3] ^= 0xFF; // 破坏第二个长度域
        assert!(matches!(
            Frame::decode(&bytes),
            Err(ProtoError::LengthMismatch { .. })
        ));
    }

    #[test]
    fn rejects_bad_checksum() {
        let frame = sample_frame();
        let mut bytes = frame.encode().unwrap();
        let cs_index = bytes.len() - 2;
        bytes[cs_index] ^= 0xFF;
        assert!(matches!(
            Frame::decode(&bytes),
            Err(ProtoError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn rejects_missing_end_marker() {
        let frame = sample_frame();
        let mut bytes = frame.encode().unwrap();
        let last = bytes.len() - 1;
        bytes[last] = 0x00;
        assert!(matches!(
            Frame::decode(&bytes),
            Err(ProtoError::InvalidFrameMarker)
        ));
    }
}

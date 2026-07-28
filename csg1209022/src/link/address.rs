//! 链路层地址域 A（标准 5.1.4）。
//!
//! 地址域由三部分组成，共 7 字节：
//!
//! | 字段 | 格式 | 字节数 |
//! | --- | --- | --- |
//! | 省地市区县码 A1 | BCD | 3 |
//! | 终端地址 A2 | BIN | 3 |
//! | 主站地址 A3 | BIN | 1 |
//!
//! A1 高字节=省份、中间字节=地市、低字节=区县（按 GB 2260），传输顺序是低字节（区县）在前。
//! A2 传输顺序同样低字节在前；`0x000000` 为无效地址，`0xFFFFFF` 为系统广播地址。

use crate::error::{ProtoError, Result};

pub const ADDRESS_LEN: usize = 7;

/// 终端地址 A2 = 0 表示无效地址
pub const TERMINAL_ADDR_INVALID: u32 = 0x000000;
/// 终端地址 A2 = 0xFFFFFF 表示系统广播地址
pub const TERMINAL_ADDR_BROADCAST: u32 = 0xFFFFFF;

/// 省地市区县码 A1（GB 2260），每个码字节用 BCD 表示 0~99。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionCode {
    pub province: u8,
    pub city: u8,
    pub county: u8,
}

impl RegionCode {
    fn decode(bytes: [u8; 3]) -> Result<Self> {
        // 传输顺序：区县(低字节) 在前，地市，省份(高字节) 在后
        let county = proto_common::bcd::decode(&[bytes[0]])? as u8;
        let city = proto_common::bcd::decode(&[bytes[1]])? as u8;
        let province = proto_common::bcd::decode(&[bytes[2]])? as u8;
        Ok(Self {
            province,
            city,
            county,
        })
    }

    fn encode(self) -> Result<[u8; 3]> {
        let county = proto_common::bcd::encode(self.county as u64, 1)
            .ok_or(ProtoError::OutOfRange(self.county as u32))?[0];
        let city = proto_common::bcd::encode(self.city as u64, 1)
            .ok_or(ProtoError::OutOfRange(self.city as u32))?[0];
        let province = proto_common::bcd::encode(self.province as u64, 1)
            .ok_or(ProtoError::OutOfRange(self.province as u32))?[0];
        Ok([county, city, province])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressField {
    pub region: RegionCode,
    /// 终端地址 A2：1~16777216 有效，0 为无效地址，0xFFFFFF 为广播地址
    pub terminal_addr: u32,
    /// 主站地址 A3：0~255；终端启动的发送帧此值应为 0
    pub master_addr: u8,
}

impl AddressField {
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < ADDRESS_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: ADDRESS_LEN,
                actual: buf.len(),
            });
        }
        let region = RegionCode::decode([buf[0], buf[1], buf[2]])?;
        // A2 三字节 BIN，低字节在前
        let terminal_addr =
            buf[3] as u32 | (buf[4] as u32) << 8 | (buf[5] as u32) << 16;
        let master_addr = buf[6];
        Ok((
            Self {
                region,
                terminal_addr,
                master_addr,
            },
            ADDRESS_LEN,
        ))
    }

    pub fn encode(&self) -> Result<[u8; ADDRESS_LEN]> {
        let region_bytes = self.region.encode()?;
        let [a2_0, a2_1, a2_2] = [
            (self.terminal_addr & 0xFF) as u8,
            ((self.terminal_addr >> 8) & 0xFF) as u8,
            ((self.terminal_addr >> 16) & 0xFF) as u8,
        ];
        Ok([
            region_bytes[0],
            region_bytes[1],
            region_bytes[2],
            a2_0,
            a2_1,
            a2_2,
            self.master_addr,
        ])
    }

    pub fn is_broadcast(&self) -> bool {
        self.terminal_addr == TERMINAL_ADDR_BROADCAST
    }

    pub fn is_invalid_terminal(&self) -> bool {
        self.terminal_addr == TERMINAL_ADDR_INVALID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_normal_address() {
        let addr = AddressField {
            region: RegionCode {
                province: 44,
                city: 1,
                county: 5,
            },
            terminal_addr: 123456,
            master_addr: 1,
        };
        let bytes = addr.encode().unwrap();
        let (decoded, consumed) = AddressField::decode(&bytes).unwrap();
        assert_eq!(consumed, ADDRESS_LEN);
        assert_eq!(decoded, addr);
    }

    #[test]
    fn broadcast_terminal_address() {
        let bytes = [0x05, 0x01, 0x44, 0xFF, 0xFF, 0xFF, 0x00];
        let (decoded, _) = AddressField::decode(&bytes).unwrap();
        assert!(decoded.is_broadcast());
    }

    #[test]
    fn eof_when_buffer_too_short() {
        let bytes = [0x00; 5];
        assert!(matches!(
            AddressField::decode(&bytes),
            Err(ProtoError::UnexpectedEof { .. })
        ));
    }
}

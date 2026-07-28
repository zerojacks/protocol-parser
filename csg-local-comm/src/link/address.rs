//! 地址域 A 解析（12字节）
//!
//! 地址域由源地址和目标地址组成，每个地址 6 字节，BIN 编码：
//! - `ASRC (6B)` + `ADST (6B)`
//! - 广播地址：`99 99 99 99 99 99H`
//!
//! 在下行帧中：
//! - ASRC = 集中器/采集器侧 MAC 地址
//! - ADST = 本地通信模块 MAC 地址
//!
//! 在上行帧中：
//! - ASRC = 本地通信模块 MAC 地址
//! - ADST = 集中器/采集器侧 MAC 地址

use crate::error::{Error, Result};

/// 地址（6字节 BIN）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Address {
    /// 地址字节（6字节）
    bytes: [u8; 6],
}

impl Address {
    /// 地址长度（6字节）
    pub const LEN: usize = 6;

    /// 广播地址
    pub const BROADCAST: Address = Address {
        bytes: [0x99, 0x99, 0x99, 0x99, 0x99, 0x99],
    };

    /// 从字节数组创建地址
    pub fn from_bytes(bytes: [u8; 6]) -> Self {
        Self { bytes }
    }

    /// 从字节切片解析地址
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: buf.len(),
            });
        }

        let mut bytes = [0u8; 6];
        bytes.copy_from_slice(&buf[..6]);
        Ok(Self { bytes })
    }

    /// 转换为字节数组
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.bytes
    }

    /// 判断是否为广播地址
    pub fn is_broadcast(&self) -> bool {
        self.bytes == Self::BROADCAST.bytes
    }

    /// 转换为十六进制字符串（格式：XX-XX-XX-XX-XX-XX）
    pub fn to_hex_string(&self) -> String {
        self.bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// 地址域（12字节：源地址 + 目标地址）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressDomain {
    /// 源地址 ASRC（6字节）
    pub source: Address,
    /// 目标地址 ADST（6字节）
    pub destination: Address,
}

impl AddressDomain {
    /// 地址域长度（12字节）
    pub const LEN: usize = 12;

    /// 从字节切片解析地址域
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: buf.len(),
            });
        }

        let source = Address::decode(&buf[0..6])?;
        let destination = Address::decode(&buf[6..12])?;

        Ok(Self {
            source,
            destination,
        })
    }

    /// 编码为字节数组
    pub fn encode(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        bytes[0..6].copy_from_slice(self.source.as_bytes());
        bytes[6..12].copy_from_slice(self.destination.as_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_address() {
        let addr = Address::BROADCAST;
        assert!(addr.is_broadcast());
        assert_eq!(addr.to_hex_string(), "99-99-99-99-99-99");
    }

    #[test]
    fn test_address_roundtrip() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let addr = Address::from_bytes(bytes);
        assert_eq!(addr.as_bytes(), &bytes);
        assert_eq!(addr.to_hex_string(), "01-02-03-04-05-06");
    }

    #[test]
    fn test_address_domain_decode() {
        let bytes = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // ASRC
            0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, // ADST
        ];
        let domain = AddressDomain::decode(&bytes).unwrap();
        assert_eq!(domain.source.as_bytes(), &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        assert_eq!(
            domain.destination.as_bytes(),
            &[0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C]
        );
    }

    #[test]
    fn test_address_domain_roundtrip() {
        let domain = AddressDomain {
            source: Address::from_bytes([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]),
            destination: Address::from_bytes([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
        };

        let bytes = domain.encode();
        let domain2 = AddressDomain::decode(&bytes).unwrap();
        assert_eq!(domain, domain2);
    }

    #[test]
    fn test_address_decode_insufficient_length() {
        let bytes = [0x01, 0x02, 0x03]; // 只有3字节
        let result = Address::decode(&bytes);
        assert!(result.is_err());
    }
}

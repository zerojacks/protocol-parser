//! 控制字节 C 解析（1字节）
//!
//! 位布局（D7 = MSB）：
//! ```text
//! | D7  | D6  | D5  | D4-D3 | D2-D0 |
//! | DIR | PRM | ADD | VER   | 保留  |
//! ```
//!
//! - **DIR**: 0 = 下行（集中器/采集器 → 模块），1 = 上行（模块 → 集中器/采集器）
//! - **PRM**: 1 = 启动站发出，0 = 从动站发出
//! - **ADD**: 1 = 包含地址域，0 = 无地址域
//! - **VER**: 协议版本，当前固定为 0
//! - **保留**: 固定为 0

use crate::error::{Error, Result};

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 下行：集中器/采集器 → 本地通信模块
    Downlink,
    /// 上行：本地通信模块 → 集中器/采集器
    Uplink,
}

/// 控制字节（1字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlByte {
    /// 传输方向
    pub direction: Direction,
    /// 是否为启动站发出
    pub is_primary: bool,
    /// 是否包含地址域
    pub has_address: bool,
    /// 协议版本（当前固定为 0）
    pub version: u8,
}

impl ControlByte {
    /// 从字节解析控制字节
    pub fn from_byte(byte: u8) -> Result<Self> {
        let direction = if byte & 0x80 != 0 {
            Direction::Uplink
        } else {
            Direction::Downlink
        };

        let is_primary = (byte & 0x40) != 0;
        let has_address = (byte & 0x20) != 0;
        let version = (byte >> 3) & 0x03;
        let reserved = byte & 0x07;

        // 检查保留位是否为 0
        if reserved != 0 {
            return Err(Error::InvalidControlByte {
                byte,
                reason: format!("保留位应为 0，实际为 {:03b}B", reserved),
            });
        }

        // 检查版本是否为 0
        if version != 0 {
            return Err(Error::InvalidControlByte {
                byte,
                reason: format!("协议版本应为 0，实际为 {}", version),
            });
        }

        Ok(Self {
            direction,
            is_primary,
            has_address,
            version,
        })
    }

    /// 转换为字节
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;

        if self.direction == Direction::Uplink {
            byte |= 0x80;
        }

        if self.is_primary {
            byte |= 0x40;
        }

        if self.has_address {
            byte |= 0x20;
        }

        byte |= (self.version & 0x03) << 3;

        byte
    }

    /// 创建下行启动帧控制字节（带地址域）
    pub fn downlink_primary_with_address() -> Self {
        Self {
            direction: Direction::Downlink,
            is_primary: true,
            has_address: true,
            version: 0,
        }
    }

    /// 创建下行启动帧控制字节（无地址域）
    pub fn downlink_primary_without_address() -> Self {
        Self {
            direction: Direction::Downlink,
            is_primary: true,
            has_address: false,
            version: 0,
        }
    }

    /// 创建上行响应帧控制字节（带地址域）
    pub fn uplink_response_with_address() -> Self {
        Self {
            direction: Direction::Uplink,
            is_primary: false,
            has_address: true,
            version: 0,
        }
    }

    /// 创建上行响应帧控制字节（无地址域）
    pub fn uplink_response_without_address() -> Self {
        Self {
            direction: Direction::Uplink,
            is_primary: false,
            has_address: false,
            version: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_byte_downlink_primary_with_address() {
        let ctrl = ControlByte::downlink_primary_with_address();
        assert_eq!(ctrl.direction, Direction::Downlink);
        assert!(ctrl.is_primary);
        assert!(ctrl.has_address);
        assert_eq!(ctrl.version, 0);

        let byte = ctrl.to_byte();
        assert_eq!(byte, 0x60); // 0110 0000
    }

    #[test]
    fn test_control_byte_uplink_response_with_address() {
        let ctrl = ControlByte::uplink_response_with_address();
        assert_eq!(ctrl.direction, Direction::Uplink);
        assert!(!ctrl.is_primary);
        assert!(ctrl.has_address);
        assert_eq!(ctrl.version, 0);

        let byte = ctrl.to_byte();
        assert_eq!(byte, 0xA0); // 1010 0000
    }

    #[test]
    fn test_control_byte_roundtrip() {
        let ctrl = ControlByte {
            direction: Direction::Downlink,
            is_primary: true,
            has_address: false,
            version: 0,
        };

        let byte = ctrl.to_byte();
        let ctrl2 = ControlByte::from_byte(byte).unwrap();
        assert_eq!(ctrl, ctrl2);
    }

    #[test]
    fn test_invalid_reserved_bits() {
        let byte = 0x61; // 保留位不为 0
        let result = ControlByte::from_byte(byte);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let byte = 0x68; // 版本为 1
        let result = ControlByte::from_byte(byte);
        assert!(result.is_err());
    }
}

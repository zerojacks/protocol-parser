//! DI (数据标识编码) 定义
//!
//! 数据标识由 4 字节组成：DI3 DI2 DI1 DI0
//!
//! - **DI3**: 节点角色
//!   - E8H = 集中器与本地模块通信
//!   - EAH = 采集器与本地模块通信
//! - **DI2**: 消息方向/内容类型
//!   - 00H: 上下行都用，下行无数据内容
//!   - 01H: 上下行都用，数据内容格式相同
//!   - 02H: 仅下行，对应上行为确认/否认
//!   - 03H: 仅下行，带数据；对应上行 DI2=04
//!   - 04H: 仅上行，带数据；对应下行 DI2=03
//!   - 05H: 仅上行；对应下行为确认/否认
//!   - 06H: 上下行都用，上行无数据内容
//! - **DI1**: 与 AFN 值一致
//! - **DI0**: 子功能码

use crate::error::{Error, Result};

/// 节点角色（DI3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// E8H: 集中器
    Concentrator,
    /// EAH: 采集器
    Collector,
    /// 其他未定义角色
    Other(u8),
}

impl NodeRole {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0xE8 => NodeRole::Concentrator,
            0xEA => NodeRole::Collector,
            other => NodeRole::Other(other),
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            NodeRole::Concentrator => 0xE8,
            NodeRole::Collector => 0xEA,
            NodeRole::Other(byte) => *byte,
        }
    }
}

/// 消息方向类型（DI2）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageDirection {
    /// 00H: 上下行都用，下行无数据内容
    BothNoDownlinkData,
    /// 01H: 上下行都用，数据内容格式相同
    BothSameFormat,
    /// 02H: 仅下行，对应上行为确认/否认
    DownlinkOnlyAckResponse,
    /// 03H: 仅下行，带数据
    DownlinkWithData,
    /// 04H: 仅上行，带数据
    UplinkWithData,
    /// 05H: 仅上行，对应下行为确认/否认
    UplinkOnlyAckResponse,
    /// 06H: 上下行都用，上行无数据内容
    BothNoUplinkData,
    /// 其他
    Other(u8),
}

impl MessageDirection {
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => MessageDirection::BothNoDownlinkData,
            0x01 => MessageDirection::BothSameFormat,
            0x02 => MessageDirection::DownlinkOnlyAckResponse,
            0x03 => MessageDirection::DownlinkWithData,
            0x04 => MessageDirection::UplinkWithData,
            0x05 => MessageDirection::UplinkOnlyAckResponse,
            0x06 => MessageDirection::BothNoUplinkData,
            other => MessageDirection::Other(other),
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            MessageDirection::BothNoDownlinkData => 0x00,
            MessageDirection::BothSameFormat => 0x01,
            MessageDirection::DownlinkOnlyAckResponse => 0x02,
            MessageDirection::DownlinkWithData => 0x03,
            MessageDirection::UplinkWithData => 0x04,
            MessageDirection::UplinkOnlyAckResponse => 0x05,
            MessageDirection::BothNoUplinkData => 0x06,
            MessageDirection::Other(byte) => *byte,
        }
    }
}

/// 数据标识（4字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataIdentifier {
    /// DI3: 节点角色
    pub node_role: NodeRole,
    /// DI2: 消息方向类型
    pub direction: MessageDirection,
    /// DI1: 与 AFN 值一致
    pub afn_match: u8,
    /// DI0: 子功能码
    pub sub_function: u8,
}

impl DataIdentifier {
    /// 数据标识长度（4字节）
    pub const LEN: usize = 4;

    /// 从字节数组创建（小端序：DI0 DI1 DI2 DI3）
    /// 
    /// 传输顺序：DI0(低字节) DI1 DI2 DI3(高字节)
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            sub_function: bytes[0],  // DI0
            afn_match: bytes[1],     // DI1
            direction: MessageDirection::from_byte(bytes[2]), // DI2
            node_role: NodeRole::from_byte(bytes[3]),        // DI3
        }
    }

    /// 从字节切片解析（小端序）
    pub fn decode(buf: &[u8]) -> Result<Self> {
        if buf.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: buf.len(),
            });
        }

        Ok(Self::from_bytes([buf[0], buf[1], buf[2], buf[3]]))
    }

    /// 转换为字节数组（小端序：DI0 DI1 DI2 DI3）
    pub fn to_bytes(&self) -> [u8; 4] {
        [
            self.sub_function,           // DI0
            self.afn_match,              // DI1
            self.direction.to_byte(),    // DI2
            self.node_role.to_byte(),    // DI3
        ]
    }

    /// 转换为 u32（用于显示，大端序：DI3 DI2 DI1 DI0）
    pub fn to_u32(&self) -> u32 {
        let bytes = [
            self.node_role.to_byte(),    // DI3 (高字节)
            self.direction.to_byte(),    // DI2
            self.afn_match,              // DI1
            self.sub_function,           // DI0 (低字节)
        ];
        u32::from_be_bytes(bytes)
    }

    /// 创建确认 DI（xx 01 00 01）
    pub fn ack(node_role: NodeRole) -> Self {
        Self {
            node_role,
            direction: MessageDirection::BothSameFormat,
            afn_match: 0x00,
            sub_function: 0x01,
        }
    }

    /// 创建否认 DI（xx 01 00 02）
    pub fn nack(node_role: NodeRole) -> Self {
        Self {
            node_role,
            direction: MessageDirection::BothSameFormat,
            afn_match: 0x00,
            sub_function: 0x02,
        }
    }

    /// 判断是否为确认
    pub fn is_ack(&self) -> bool {
        self.afn_match == 0x00 && self.sub_function == 0x01
    }

    /// 判断是否为否认
    pub fn is_nack(&self) -> bool {
        self.afn_match == 0x00 && self.sub_function == 0x02
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_di_roundtrip() {
        let di = DataIdentifier {
            node_role: NodeRole::Concentrator,
            direction: MessageDirection::BothSameFormat,
            afn_match: 0x03,
            sub_function: 0x01,
        };

        let bytes = di.to_bytes();
        let di2 = DataIdentifier::from_bytes(bytes);
        assert_eq!(di, di2);
    }

    #[test]
    fn test_di_ack() {
        let di = DataIdentifier::ack(NodeRole::Concentrator);
        assert!(di.is_ack());
        assert!(!di.is_nack());
        assert_eq!(di.to_u32(), 0xE8010001);
    }

    #[test]
    fn test_di_nack() {
        let di = DataIdentifier::nack(NodeRole::Collector);
        assert!(di.is_nack());
        assert!(!di.is_ack());
        assert_eq!(di.to_u32(), 0xEA010002);
    }

    #[test]
    fn test_node_role() {
        assert_eq!(NodeRole::from_byte(0xE8), NodeRole::Concentrator);
        assert_eq!(NodeRole::from_byte(0xEA), NodeRole::Collector);
        assert_eq!(NodeRole::Concentrator.to_byte(), 0xE8);
    }
}

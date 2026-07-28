//! DL/T 645-2007 多功能电能表通信协议解析器
//!
//! 本库实现了 DL/T 645-2007《多功能电能表通信协议》的完整解析功能。
//!
//! # 架构设计
//!
//! 本库采用分层架构，与 CSG1209022 保持一致：
//!
//! - **链路层** (`link`): 负责帧格式、地址域、控制码、校验和
//! - **应用层** (`app`): 负责数据域的具体内容解析
//! - **顶层引擎** (`engine`): 将链路层和应用层串联起来
//!
//! ## 帧格式
//!
//! ```text
//! 68H | A0..A5 | 68H | C | L | DATA(L字节) | CS | 16H
//!      └─地址6字节─┘         └─数据域─────┘
//! ```
//!
//! ## 关键特性
//!
//! - **地址域**：6字节BCD编码，表示12位十进制地址
//! - **数据偏移**：DATA域所有字节发送时+33H，接收时-33H
//! - **数据标识**：4字节DI3DI2DI1DI0（低字节先传）
//! - **校验和**：从第一个68H到DATA末尾的累加和(mod 256)
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use dlt645_2007::engine::decode_message;
//!
//! // 解析原始字节（完整报文）
//! let bytes = [0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x68, 
//!              0x11, 0x04, 0x33, 0x33, 0x34, 0x33, 0xEB, 0x16];
//! let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "国网")?;
//!
//! println!("地址: {}", msg.frame.address);
//! println!("控制码: {:?}", msg.frame.control);
//! println!("功能码: {:?}", msg.application.function);
//! println!("数据: {:?}", msg.application.body);
//! ```

pub mod address;
pub mod app;
pub mod control;
pub mod data_identifier;
pub mod engine;
pub mod error;
pub mod link;
pub mod report;

pub use address::Address;
pub use app::{ApplicationBody, ApplicationLayer, BroadcastTimeData, DataItem, FreezeTime};
pub use control::{ControlCode, Direction, FunctionCode};
pub use data_identifier::DataIdentifier;
pub use engine::{decode_message, encode_message, Message};
pub use error::{Error, Result};
pub use link::Frame;
pub use proto_common::FieldValue;

/// DL/T 645-2007 协议版本标识
pub const PROTOCOL_VERSION: &str = "DL/T 645-2007";

/// 帧起始符（出现两次）
pub const FRAME_START: u8 = 0x68;

/// 帧结束符
pub const FRAME_END: u8 = 0x16;

/// 数据域字节偏移量（发送时+33H，接收时-33H）
pub const DATA_OFFSET: u8 = 0x33;

/// 前导字节（用于唤醒从站）
pub const PREAMBLE: u8 = 0xFE;

/// 广播地址（12个9）
pub const BROADCAST_ADDRESS: &str = "999999999999";

/// 通配符字节（用于地址的"不关心"匹配）
pub const WILDCARD_BYTE: u8 = 0xAA;

/// 检测给定的字节数组是否为 DL/T 645-2007 协议帧
///
/// 检测规则：
/// 1. 起始符必须是 68H（可能有前导 FE）
/// 2. 地址域后必须有第二个 68H
/// 3. 结束符必须为 16H
/// 4. 数据长度域必须合理
pub fn is_dlt645_frame(buf: &[u8]) -> bool {
    // 跳过可能的前导字节 FE
    let mut start = 0;
    while start < buf.len() && buf[start] == 0xFE {
        start += 1;
    }

    // 最小长度：68H + A(6B) + 68H + C + L + CS + 16H = 12字节
    if buf.len() - start < 12 {
        return false;
    }

    // 检查第一个起始符
    if buf[start] != 0x68 {
        return false;
    }

    // 检查第二个起始符（地址域后）
    if buf[start + 7] != 0x68 {
        return false;
    }

    // 检查数据长度
    let data_len = buf[start + 9] as usize;
    let expected_frame_len = 12 + data_len;
    
    if buf.len() - start < expected_frame_len {
        return false;
    }

    // 检查结束符
    if buf[start + expected_frame_len - 1] != 0x16 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_dlt645_frame_valid() {
        // 构造一个基本的 DLT645 帧
        let frame = vec![
            0x68, // 起始符
            0x12, 0x90, 0x78, 0x56, 0x34, 0x12, // 地址域
            0x68, // 第二个起始符
            0x11, // 控制码
            0x04, // 数据长度
            0x33, 0x33, 0x34, 0x33, // 数据域（+33H后）
            0xEB, // 校验和
            0x16, // 结束符
        ];
        assert!(is_dlt645_frame(&frame));
    }

    #[test]
    fn test_is_dlt645_frame_with_preamble() {
        // 带前导字节的帧
        let frame = vec![
            0xFE, 0xFE, 0xFE, 0xFE, // 前导字节
            0x68, // 起始符
            0x12, 0x90, 0x78, 0x56, 0x34, 0x12, // 地址域
            0x68, // 第二个起始符
            0x11, 0x04, 0x33, 0x33, 0x34, 0x33, 0xEB, 0x16,
        ];
        assert!(is_dlt645_frame(&frame));
    }

    #[test]
    fn test_is_dlt645_frame_wrong_second_start() {
        let frame = vec![
            0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12,
            0x69, // 错误的第二个起始符
            0x11, 0x04, 0x33, 0x33, 0x34, 0x33, 0xEB, 0x16,
        ];
        assert!(!is_dlt645_frame(&frame));
    }
}

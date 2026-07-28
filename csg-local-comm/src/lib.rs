//! Q/CSG1209021-2019 计量自动化终端本地通信模块接口协议解析器
//!
//! 本库实现了 **Q/CSG1209021-2019《计量自动化终端本地通信模块接口协议》**的完整解析功能。
//! 该协议定义了集中器/采集器与其本地通信模块（载波/微功率无线/以太网）之间的通信规范。
//!
//! ## 协议特点
//!
//! - 帧格式：`68H | L(2B) | C(1B) | [A(12B)] | AFN(1B) | SEQ(1B) | DI(4B) | 数据内容 | CS(1B) | 16H`
//! - 两种节点角色：集中器（DI3=E8H）、采集器（DI3=EAH）
//! - 支持广播地址：`99 99 99 99 99 99H`
//! - 三种传输服务类：发送/无回答、发送/确认、请求/响应
//!
//! ## 架构设计
//!
//! 本库采用三层架构，与 DLT645-2007 和 CSG1209022 保持一致：
//!
//! - **链路层 (link)**：负责帧格式、地址域、控制字节的编解码
//! - **应用层 (app)**：负责 AFN、DI、数据内容的解析
//! - **引擎层 (engine)**：串联链路层和应用层，提供统一的编解码接口，集成 spec-engine
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use csg1209021::decode_message;
//!
//! let bytes = vec![0x68, 0x25, 0x00, ...]; // 完整帧
//! let (msg, consumed) = decode_message(&bytes, "csg-local-comm", "南网")?;
//!
//! println!("控制字节: {:?}", msg.frame.control);
//! println!("AFN: {:?}", msg.application.afn);
//! println!("DI: {:08X}H", msg.application.di.to_u32());
//!
//! // 渲染成树形结构供前端展示
//! let tree = msg.to_value_tree(&bytes[..consumed])?;
//! ```

pub mod app;
pub mod control;
pub mod engine;
pub mod error;
pub mod link;
pub mod report;

pub use app::{Afn, ApplicationBody, ApplicationLayer, DataIdentifier};
pub use control::{ControlByte, Direction};
pub use engine::{decode_message, encode_message, Message};
pub use error::{Error, Result};
pub use link::{Address, Frame};
pub use proto_common::FieldValue;

/// 检测给定的字节数组是否为 Q/CSG1209021-2019 协议帧
///
/// 检测规则：
/// 1. 起始符必须是 68H
/// 2. 长度域必须合理（至少包含基本帧结构）
/// 3. 结束符必须在正确的位置且为 16H
/// 4. 控制字节的版本位必须为 0
pub fn is_csg_local_comm_frame(buf: &[u8]) -> bool {
    // 最小长度检查：68H + L(2B) + C(1B) + payload(至少1B) + CS(1B) + 16H = 7字节
    if buf.len() < 7 {
        return false;
    }

    // 检查起始符
    if buf[0] != 0x68 {
        return false;
    }

    // 检查长度域
    let frame_len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
    if frame_len < 7 || frame_len > buf.len() {
        return false;
    }

    // 检查结束符位置
    if buf[frame_len - 1] != 0x16 {
        return false;
    }

    // 检查控制字节的版本位（D4-D3 必须为 00）
    let control = buf[3];
    let version = (control >> 3) & 0x03;
    if version != 0 {
        return false;
    }

    // 检查保留位（D2-D0 必须为 000）
    let reserved = control & 0x07;
    if reserved != 0 {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_csg_local_comm_frame_valid() {
        // 构造一个有效的 CSG 本地通信帧
        let frame = vec![
            0x68, 0x0C, 0x00, 0x80, // 起始符 + 长度 + 控制字
            0x00, 0x01, 0x01, 0x00, 0x01, 0xE8, // AFN + SEQ + DI
            0x6B, 0x16, // 校验和 + 结束符
        ];
        assert!(is_csg_local_comm_frame(&frame));
    }

    #[test]
    fn test_is_csg_local_comm_frame_wrong_start() {
        let frame = vec![0x69, 0x0C, 0x00, 0x80, 0x00, 0x01, 0x6B, 0x16];
        assert!(!is_csg_local_comm_frame(&frame));
    }

    #[test]
    fn test_is_csg_local_comm_frame_wrong_end() {
        let frame = vec![0x68, 0x08, 0x00, 0x80, 0x00, 0x01, 0x6B, 0x17];
        assert!(!is_csg_local_comm_frame(&frame));
    }

    #[test]
    fn test_is_csg_local_comm_frame_too_short() {
        let frame = vec![0x68, 0x0C, 0x00];
        assert!(!is_csg_local_comm_frame(&frame));
    }
}

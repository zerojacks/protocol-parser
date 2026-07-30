//! 中国南方电网《计量自动化终端上行通信规约》（Q/CSG1209022-2019）解析器。
//!
//! 分层对应标准的三层参考模型：
//!
//! - [`link`]：链路层——FT1.2 帧格式、控制域 C、地址域 A（标准第5章）
//! - [`app`]：应用层——AFN、SEQ、信息点标识 DA、数据标识 DI、数据时间域、时间标签 Tp（标准第6章）
//! - [`engine`]：顶层入口，把链路层和应用层串起来
//!
//! DI 数据标识**内容**本身的解析完全委托给独立发布的 [`spec_engine`](https://github.com/zerojacks/spec-engine)
//! （只是普通依赖引用，不在本 workspace 内），本 crate 只负责帧结构、寻址、AFN分发。
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use csg1209022::decode_message;
//!
//! let (msg, consumed) = decode_message(&raw_bytes, "csg13", "南网")?;
//! println!("{:?}", msg.application.body);
//! ```

pub mod app;
pub mod engine;
pub mod error;
pub mod link;
pub mod report;

pub use engine::{decode_message, encode_message, Message};
pub use error::{ProtoError, Result};
pub use proto_common::FieldValue;
pub use report::decode_message_as_value;

/// 检测给定的字节数组是否为 Q/CSG1209022-2019 协议帧
///
/// 检测规则：
/// 1. 起始符必须是 68H
/// 2. 长度域必须合理且两次出现一致
/// 3. 第二个起始符必须是 68H
/// 4. 结束符必须在正确的位置且为 16H
pub fn is_csg1209022_frame(buf: &[u8]) -> bool {
    
    // 最小长度：68H + L(2) + L(2) + 68H + C(1) + A(7) + CS(1) + 16H = 22字节
    if buf.len() < 22 {
        return false;
    }

    // 检查起始符
    if buf[0] != 0x68 || buf[5] != 0x68 {
        return false;
    }

    // 检查长度域（两次出现必须一致）
    let frame_len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
    let frame_len2 = u16::from_le_bytes([buf[3], buf[4]]) as usize;
    if frame_len != frame_len2 {
        return false;
    }

    // 检查长度合理性
    if frame_len < 7 || frame_len > 65535 {
        return false;
    }

    if (frame_len + 8 ) != buf.len() {
        return false;
    }

    // 检查结束符
    if buf[frame_len - 1] != 0x16 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_is_csg1209022_frame() {
        // 这里需要一个真实的 CSG1209022 帧进行测试
        // 暂时只测试基本格式
        let _frame = vec![
            0x68, 0x10, 0x00, 0x43, // 起始符 + 长度 + 控制域
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, // 地址域
            0x00, 0x01, // AFN + SEQ
            0x00, 0x00, // 校验和（示例）
            0x16, // 结束符
        ];
        // 由于校验和可能不正确，这里只是示例
        // 实际使用时需要真实的帧数据
    }
}

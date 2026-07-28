//! 本 crate 统一的错误类型。
//!
//! 分两类：帧/链路层的结构性错误（帧头帧尾、长度、校验和），以及应用层内容
//! 解析错误——后者透传自 `spec-engine::DictError`，帧层完全不关心 DI 具体怎么解。

use proto_common::BcdError;

#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    #[error("数据不足：需要至少 {needed} 字节，实际只有 {actual} 字节")]
    UnexpectedEof { needed: usize, actual: usize },

    #[error("帧起始/结束标志字节非法（期望 0x68...0x16）")]
    InvalidFrameMarker,

    #[error("长度域L两次出现的值不一致：{first} != {second}")]
    LengthMismatch { first: u16, second: u16 },

    #[error("帧校验和不匹配：期望 0x{expected:02X}，实际 0x{actual:02X}")]
    ChecksumMismatch { expected: u8, actual: u8 },

    #[error("未知的应用层功能码 AFN: 0x{0:02X}")]
    UnknownAfn(u8),

    #[error("信息点标识（PSEQ/RSEQ 等）超出范围：{0}")]
    OutOfRange(u32),

    #[error("BCD解码失败: {0}")]
    Bcd(#[from] BcdError),

    #[error("DI内容解析失败: {0}")]
    Dict(#[from] spec_engine::DictError),
}

pub type Result<T> = std::result::Result<T, ProtoError>;

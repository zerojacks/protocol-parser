//! 错误类型定义

use std::fmt;

/// CSG1209021 协议解析错误
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// 意外的 EOF（期望 N 字节，实际只有 M 字节）
    UnexpectedEof { needed: usize, actual: usize },

    /// 无效的起始符（期望 68H）
    InvalidStartChar { found: u8 },

    /// 无效的结束符（期望 16H）
    InvalidEndChar { found: u8 },

    /// 校验和不匹配
    ChecksumMismatch { expected: u8, actual: u8 },

    /// 无效的控制字节
    InvalidControlByte { byte: u8, reason: String },

    /// 无效的 AFN 功能码
    InvalidAfn { code: u8 },

    /// 无效的数据标识
    InvalidDataIdentifier { reason: String },

    /// 无效的数据格式
    InvalidDataFormat { reason: String },

    /// 帧长度不匹配
    LengthMismatch { declared: usize, actual: usize },

    /// spec-engine 解析错误
    SpecEngineError { message: String },

    /// 其他错误
    Other { message: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnexpectedEof { needed, actual } => {
                write!(f, "意外的 EOF：需要 {} 字节，实际只有 {} 字节", needed, actual)
            }
            Error::InvalidStartChar { found } => {
                write!(f, "无效的起始符：期望 68H，实际 {:02X}H", found)
            }
            Error::InvalidEndChar { found } => {
                write!(f, "无效的结束符：期望 16H，实际 {:02X}H", found)
            }
            Error::ChecksumMismatch { expected, actual } => {
                write!(f, "校验和不匹配：期望 {:02X}H，实际 {:02X}H", expected, actual)
            }
            Error::InvalidControlByte { byte, reason } => {
                write!(f, "无效的控制字节 {:02X}H：{}", byte, reason)
            }
            Error::InvalidAfn { code } => {
                write!(f, "无效的 AFN 功能码：{:02X}H", code)
            }
            Error::InvalidDataIdentifier { reason } => {
                write!(f, "无效的数据标识：{}", reason)
            }
            Error::InvalidDataFormat { reason } => {
                write!(f, "无效的数据格式：{}", reason)
            }
            Error::LengthMismatch { declared, actual } => {
                write!(f, "帧长度不匹配：声明 {} 字节，实际 {} 字节", declared, actual)
            }
            Error::SpecEngineError { message } => {
                write!(f, "spec-engine 解析错误：{}", message)
            }
            Error::Other { message } => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for Error {}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, Error>;

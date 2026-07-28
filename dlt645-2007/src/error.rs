//! 错误类型定义

use std::fmt;

/// DL/T 645-2007 协议解析错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// 帧起始符不匹配（期望 68H）
    InvalidFrameStart { expected: u8, actual: u8 },
    
    /// 帧结束符不匹配（期望 16H）
    InvalidFrameEnd { expected: u8, actual: u8 },
    
    /// 校验和错误
    ChecksumMismatch { expected: u8, actual: u8 },
    
    /// 缓冲区长度不足
    UnexpectedEof { needed: usize, actual: usize },
    
    /// 无效的BCD编码
    InvalidBcd { byte: u8 },
    
    /// 无效的控制码
    InvalidControlCode { byte: u8 },
    
    /// 无效的功能码
    InvalidFunctionCode { code: u8 },
    
    /// 数据长度超出限制
    DataLengthExceeded { max: usize, actual: usize },
    
    /// 地址格式错误
    InvalidAddress { reason: String },
    
    /// 数据标识未知
    UnknownDataIdentifier { di: u32 },
    
    /// 数据格式错误
    InvalidDataFormat { reason: String },
    
    /// 密码错误或未授权
    PasswordError,
    
    /// 从站返回错误响应
    SlaveError { error_bits: u8 },
    
    /// 其他错误
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidFrameStart { expected, actual } => {
                write!(f, "帧起始符错误：期望 0x{:02X}，实际 0x{:02X}", expected, actual)
            }
            Error::InvalidFrameEnd { expected, actual } => {
                write!(f, "帧结束符错误：期望 0x{:02X}，实际 0x{:02X}", expected, actual)
            }
            Error::ChecksumMismatch { expected, actual } => {
                write!(f, "校验和错误：期望 0x{:02X}，实际 0x{:02X}", expected, actual)
            }
            Error::UnexpectedEof { needed, actual } => {
                write!(f, "缓冲区不足：需要 {} 字节，实际 {} 字节", needed, actual)
            }
            Error::InvalidBcd { byte } => {
                write!(f, "无效的BCD编码：0x{:02X}", byte)
            }
            Error::InvalidControlCode { byte } => {
                write!(f, "无效的控制码：0x{:02X}", byte)
            }
            Error::InvalidFunctionCode { code } => {
                write!(f, "无效的功能码：0x{:02X}", code)
            }
            Error::DataLengthExceeded { max, actual } => {
                write!(f, "数据长度超限：最大 {} 字节，实际 {} 字节", max, actual)
            }
            Error::InvalidAddress { reason } => {
                write!(f, "地址格式错误：{}", reason)
            }
            Error::UnknownDataIdentifier { di } => {
                write!(f, "未知的数据标识：0x{:08X}", di)
            }
            Error::InvalidDataFormat { reason } => {
                write!(f, "数据格式错误：{}", reason)
            }
            Error::PasswordError => {
                write!(f, "密码错误或未授权")
            }
            Error::SlaveError { error_bits } => {
                write!(f, "从站错误响应：错误码 0x{:02X}", error_bits)
            }
            Error::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result 类型别名
pub type Result<T> = std::result::Result<T, Error>;

/// 从站错误码位定义（用于异常应答的DATA域）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlaveErrorBits {
    /// bit0: 其他错误
    pub other_error: bool,
    /// bit1: 无请求数据
    pub no_requested_data: bool,
    /// bit2: 密码错/未授权
    pub password_error: bool,
    /// bit3: 通信速率不能更改
    pub baud_rate_cannot_change: bool,
    /// bit4: 年时区数超
    pub year_time_zone_exceeded: bool,
    /// bit5: 日时段数超
    pub day_time_segment_exceeded: bool,
    /// bit6: 费率数超
    pub tariff_number_exceeded: bool,
    /// bit7: 保留
    pub reserved: bool,
}

impl SlaveErrorBits {
    /// 从字节解析错误位
    pub fn from_byte(byte: u8) -> Self {
        Self {
            other_error: byte & 0x01 != 0,
            no_requested_data: byte & 0x02 != 0,
            password_error: byte & 0x04 != 0,
            baud_rate_cannot_change: byte & 0x08 != 0,
            year_time_zone_exceeded: byte & 0x10 != 0,
            day_time_segment_exceeded: byte & 0x20 != 0,
            tariff_number_exceeded: byte & 0x40 != 0,
            reserved: byte & 0x80 != 0,
        }
    }
    
    /// 转换为字节
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.other_error { byte |= 0x01; }
        if self.no_requested_data { byte |= 0x02; }
        if self.password_error { byte |= 0x04; }
        if self.baud_rate_cannot_change { byte |= 0x08; }
        if self.year_time_zone_exceeded { byte |= 0x10; }
        if self.day_time_segment_exceeded { byte |= 0x20; }
        if self.tariff_number_exceeded { byte |= 0x40; }
        if self.reserved { byte |= 0x80; }
        byte
    }
    
    /// 获取可读的错误描述
    pub fn descriptions(&self) -> Vec<&'static str> {
        let mut descs = Vec::new();
        if self.other_error { descs.push("其他错误"); }
        if self.no_requested_data { descs.push("无请求数据"); }
        if self.password_error { descs.push("密码错/未授权"); }
        if self.baud_rate_cannot_change { descs.push("通信速率不能更改"); }
        if self.year_time_zone_exceeded { descs.push("年时区数超"); }
        if self.day_time_segment_exceeded { descs.push("日时段数超"); }
        if self.tariff_number_exceeded { descs.push("费率数超"); }
        if self.reserved { descs.push("保留位"); }
        descs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slave_error_bits() {
        let byte = 0b00000110; // bit1 和 bit2
        let bits = SlaveErrorBits::from_byte(byte);
        assert!(bits.no_requested_data);
        assert!(bits.password_error);
        assert!(!bits.other_error);
        
        assert_eq!(bits.to_byte(), byte);
        
        let descs = bits.descriptions();
        assert_eq!(descs.len(), 2);
        assert!(descs.contains(&"无请求数据"));
        assert!(descs.contains(&"密码错/未授权"));
    }
}

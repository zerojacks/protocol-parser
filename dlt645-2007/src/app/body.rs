//! DL/T 645-2007 应用层数据体
//!
//! 应用层数据体根据功能码的不同有不同的格式

use crate::control::FunctionCode;
use crate::data_identifier::DataIdentifier;
use crate::error::{Error, Result};

/// 应用层数据体
///
/// 根据功能码和方向的不同，数据体有不同的格式
#[derive(Debug, Clone, PartialEq)]
pub enum ApplicationBody {
    /// 空数据（长度为0）
    Empty,
    
    /// 读数据请求：一个或多个数据标识
    ReadRequest {
        identifiers: Vec<DataIdentifier>,
    },
    
    /// 读数据响应：数据标识 + 原始数据值
    ReadResponse {
        items: Vec<DataItem>,
    },
    
    /// 写数据请求：数据标识 + 密码 + 操作员代码 + 数据
    WriteRequest {
        identifier: DataIdentifier,
        password: Password,
        operator_code: OperatorCode,
        data: Vec<u8>,
    },
    
    /// 写数据响应：成功（无数据）
    WriteResponse,
    
    /// 广播校时：时间数据（6字节：秒 分 时 日 月 年）
    BroadcastTime {
        time: BroadcastTimeData,
    },
    
    /// 冻结命令：冻结时间或模式
    Freeze {
        freeze_time: FreezeTime,
    },
    
    /// 从站错误响应：错误码（1字节）
    Error {
        error_bits: u8,
    },
    
    /// 原始数据（未解析）
    Raw(Vec<u8>),
}

impl ApplicationBody {
    /// 从原始字节解析应用层数据体
    ///
    /// # 参数
    ///
    /// - `buf`: 数据域字节（已去除 +33H 偏移）
    /// - `function`: 功能码
    /// - `is_error`: 是否为错误响应
    pub fn parse(buf: &[u8], function: FunctionCode, is_error: bool) -> Result<Self> {
        if buf.is_empty() {
            return Ok(ApplicationBody::Empty);
        }

        // 错误响应只有1字节
        if is_error {
            if buf.len() != 1 {
                return Err(Error::InvalidDataFormat {
                    reason: format!("错误响应数据长度应为1字节，实际{}字节", buf.len()),
                });
            }
            return Ok(ApplicationBody::Error {
                error_bits: buf[0],
            });
        }

        match function {
            FunctionCode::ReadData | FunctionCode::ReadFollowUp => {
                // 读数据请求或响应
                Self::parse_read_data(buf)
            }
            FunctionCode::WriteData => {
                // 写数据请求或响应
                if buf.is_empty() {
                    Ok(ApplicationBody::WriteResponse)
                } else {
                    Self::parse_write_request(buf)
                }
            }
            FunctionCode::BroadcastTime => {
                // 广播校时
                if buf.len() != 6 {
                    return Err(Error::InvalidDataFormat {
                        reason: format!("广播校时数据应为6字节，实际{}字节", buf.len()),
                    });
                }
                Ok(ApplicationBody::BroadcastTime {
                    time: BroadcastTimeData::from_bytes(buf)?,
                })
            }
            FunctionCode::Freeze => {
                // 冻结命令
                if buf.len() != 4 {
                    return Err(Error::InvalidDataFormat {
                        reason: format!("冻结命令数据应为4字节，实际{}字节", buf.len()),
                    });
                }
                Ok(ApplicationBody::Freeze {
                    freeze_time: FreezeTime::from_bytes(buf)?,
                })
            }
            _ => {
                // 其他功能码暂不解析，保留原始数据
                Ok(ApplicationBody::Raw(buf.to_vec()))
            }
        }
    }

    /// 解析读数据（请求或响应）
    fn parse_read_data(buf: &[u8]) -> Result<Self> {
        let mut offset = 0;
        let mut identifiers = Vec::new();
        let mut items = Vec::new();

        // 尝试解析为请求（只有DI）或响应（DI+数据）
        while offset < buf.len() {
            if offset + DataIdentifier::LEN > buf.len() {
                break;
            }

            let (di, consumed) = DataIdentifier::decode(&buf[offset..])?;
            offset += consumed;
            
            // 如果后面还有数据，则为响应；否则为请求
            if offset < buf.len() {
                // 这是响应，后面跟着数据值
                // 数据值的长度由 spec-engine 确定，这里先保留所有剩余字节
                let data = buf[offset..].to_vec();
                items.push(DataItem {
                    identifier: di,
                    raw_data: data.clone(),
                    parsed_value: None, // 在 ApplicationLayer::decode 中解析
                });
                offset = buf.len(); // 消费所有数据
            } else {
                // 这是请求，只有DI
                identifiers.push(di);
            }
        }

        if !items.is_empty() {
            Ok(ApplicationBody::ReadResponse { items })
        } else if !identifiers.is_empty() {
            Ok(ApplicationBody::ReadRequest { identifiers })
        } else {
            Ok(ApplicationBody::Raw(buf.to_vec()))
        }
    }

    /// 解析写数据请求
    fn parse_write_request(buf: &[u8]) -> Result<Self> {
        // 格式：DI(4) + 密码(4) + 操作员码(4) + 数据(N)
        const MIN_LEN: usize = DataIdentifier::LEN + Password::LEN + OperatorCode::LEN;
        
        if buf.len() < MIN_LEN {
            return Err(Error::InvalidDataFormat {
                reason: format!("写数据请求至少需要{}字节", MIN_LEN),
            });
        }

        let (identifier, _) = DataIdentifier::decode(&buf[0..])?;
        let password = Password::from_bytes(&buf[4..8])?;
        let operator_code = OperatorCode::from_bytes(&buf[8..12])?;
        let data = buf[12..].to_vec();

        Ok(ApplicationBody::WriteRequest {
            identifier,
            password,
            operator_code,
            data,
        })
    }

    /// 编码为字节
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ApplicationBody::Empty => Vec::new(),
            ApplicationBody::ReadRequest { identifiers } => {
                let mut bytes = Vec::new();
                for di in identifiers {
                    bytes.extend_from_slice(di.as_bytes());
                }
                bytes
            }
            ApplicationBody::ReadResponse { items } => {
                let mut bytes = Vec::new();
                for item in items {
                    bytes.extend_from_slice(item.identifier.as_bytes());
                    bytes.extend_from_slice(&item.raw_data);
                }
                bytes
            }
            ApplicationBody::WriteRequest {
                identifier,
                password,
                operator_code,
                data,
            } => {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(identifier.as_bytes());
                bytes.extend_from_slice(&password.encode());
                bytes.extend_from_slice(&operator_code.encode());
                bytes.extend_from_slice(data);
                bytes
            }
            ApplicationBody::WriteResponse => Vec::new(),
            ApplicationBody::BroadcastTime { time } => time.to_bytes(),
            ApplicationBody::Freeze { freeze_time } => freeze_time.to_bytes(),
            ApplicationBody::Error { error_bits } => vec![*error_bits],
            ApplicationBody::Raw(bytes) => bytes.clone(),
        }
    }
}

/// 数据项（数据标识 + 原始数据值 + 解析后的值）
#[derive(Debug, Clone, PartialEq)]
pub struct DataItem {
    /// 数据标识
    pub identifier: DataIdentifier,
    /// 原始数据值（未解析格式）
    pub raw_data: Vec<u8>,
    /// spec-engine 解析后的值（如果解析成功）
    pub parsed_value: Option<proto_common::FieldValue>,
}

/// 密码（4字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Password {
    /// 权限等级（PA，4位）
    pub permission_level: u8,
    /// 密码值（P0 P1 P2，3字节）
    pub value: [u8; 3],
}

impl Password {
    pub const LEN: usize = 4;

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            permission_level: (bytes[0] >> 4) & 0x0F,
            value: [bytes[1], bytes[2], bytes[3]],
        })
    }

    pub fn encode(&self) -> [u8; 4] {
        [
            (self.permission_level << 4) | (self.value[0] >> 4),
            self.value[0],
            self.value[1],
            self.value[2],
        ]
    }
}

/// 操作员代码（4字节）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorCode {
    pub code: [u8; 4],
}

impl OperatorCode {
    pub const LEN: usize = 4;

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: bytes.len(),
            });
        }

        let mut code = [0u8; 4];
        code.copy_from_slice(&bytes[0..4]);
        Ok(Self { code })
    }

    pub fn encode(&self) -> [u8; 4] {
        self.code
    }
}

/// 广播校时数据（6字节：秒 分 时 日 月 年，BCD）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BroadcastTimeData {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u8,
}

impl BroadcastTimeData {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 6 {
            return Err(Error::UnexpectedEof {
                needed: 6,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            second: bytes[0],
            minute: bytes[1],
            hour: bytes[2],
            day: bytes[3],
            month: bytes[4],
            year: bytes[5],
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![
            self.second,
            self.minute,
            self.hour,
            self.day,
            self.month,
            self.year,
        ]
    }
}

/// 冻结时间（4字节：分 时 日 月，BCD）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeTime {
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
}

impl FreezeTime {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(Error::UnexpectedEof {
                needed: 4,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            minute: bytes[0],
            hour: bytes[1],
            day: bytes[2],
            month: bytes[3],
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        vec![self.minute, self.hour, self.day, self.month]
    }

    /// 判断是否为周期冻结模式
    pub fn is_periodic(&self) -> bool {
        self.month == 0x99
    }

    /// 判断是否为瞬时冻结
    pub fn is_instant(&self) -> bool {
        self.minute == 0x99
            && self.hour == 0x99
            && self.day == 0x99
            && self.month == 0x99
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_time() {
        let time = BroadcastTimeData {
            second: 0x59,
            minute: 0x59,
            hour: 0x23,
            day: 0x31,
            month: 0x12,
            year: 0x23,
        };
        
        let bytes = time.to_bytes();
        assert_eq!(bytes.len(), 6);
        
        let parsed = BroadcastTimeData::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, time);
    }

    #[test]
    fn test_freeze_time_instant() {
        let freeze = FreezeTime {
            minute: 0x99,
            hour: 0x99,
            day: 0x99,
            month: 0x99,
        };
        
        assert!(freeze.is_instant());
        assert!(freeze.is_periodic());
    }
}

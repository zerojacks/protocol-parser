//! DL/T 645-2007 应用层
//!
//! 应用层负责解析数据域的具体内容，
//! 包括数据标识和数据内容的解析

pub mod body;

pub use body::{
    ApplicationBody, BroadcastTimeData, DataItem, FreezeTime, OperatorCode, Password,
};

use std::sync::OnceLock;

use crate::control::{Direction, FunctionCode};
use crate::data_identifier::DataIdentifier;
use crate::error::Result;
use proto_common::{from_spec_engine, FieldValue};

fn spec_engine() -> &'static spec_engine::Engine {
    static ENGINE: OnceLock<spec_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(spec_engine::Engine::new_default)
}

/// 使用 spec-engine 解析 DI 对应的数据内容
fn parse_di_with_spec_engine(
    di: &DataIdentifier,
    data: &[u8],
    protocol: &str,
    region: &str,
    dir: Option<&str>,  // 传输方向（"down"/"up"）
) -> Result<FieldValue> {
    // spec_engine::parse_di 的签名是:
    // pub fn parse_di(protocol: &str, di: u32, region: &str, endian: Option<&str>, data: &[u8]) -> Result<(Value, usize)>
    // 注意：spec-engine 的当前实现中，endian 参数实际上用于传递方向信息
    
    let di_u32 = di.to_u32();
    
    match spec_engine().parse_di(protocol, di_u32, region, dir, data) {
        Ok((value, _consumed)) => Ok(from_spec_engine(&value)),
        Err(e) => {
            // spec-engine 解析失败，返回一个 Invalid 值
            Ok(FieldValue::Invalid {
                reason: format!("spec-engine parse error: {}", e),
            })
        }
    }
}

/// DL/T 645-2007 应用层
///
/// 包含功能码和数据体
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationLayer {
    /// 功能码（从控制码继承）
    pub function: FunctionCode,
    /// 应用层数据体
    pub body: ApplicationBody,
}

impl ApplicationLayer {
    /// 从原始字节解析应用层
    ///
    /// # 参数
    ///
    /// - `payload`: 数据域字节（已去除 +33H 偏移）
    /// - `function`: 功能码
    /// - `direction`: 传输方向（用于区分请求/响应，传给 spec-engine）
    /// - `is_error`: 是否为错误响应
    /// - `protocol`: 协议标识（传给 spec-engine，如 "dlt645-2007"）
    /// - `region`: 区域标识（传给 spec-engine，如 "国网" 或 "南网"）
    pub fn decode(
        payload: &[u8],
        function: FunctionCode,
        direction: Direction,
        is_error: bool,
        protocol: &str,
        region: &str,
    ) -> Result<Self> {
        // 解析应用层数据体
        let mut body = ApplicationBody::parse(payload, function, is_error)?;

        // 将 direction 转换为 spec-engine 需要的字符串格式
        // spec-engine 约定：0=下行（主站→从站/请求），1=上行（从站→主站/响应）
        let dir: Option<&str> = match direction {
            Direction::MasterToSlave => Some("0"),
            Direction::SlaveToMaster => Some("1"),
        };

        // 如果是读数据响应，使用 spec-engine 解析每个数据项
        if let ApplicationBody::ReadResponse { items } = &mut body {
            for item in items.iter_mut() {
                if let Ok(parsed) = parse_di_with_spec_engine(
                    &item.identifier,
                    &item.raw_data,
                    protocol,
                    region,
                    dir,
                ) {
                    item.parsed_value = Some(parsed);
                }
            }
        }

        Ok(Self { function, body })
    }

    /// 编码为字节序列
    pub fn encode(&self) -> Vec<u8> {
        self.body.encode()
    }

    /// 判断是否为错误响应
    pub fn is_error(&self) -> bool {
        matches!(self.body, ApplicationBody::Error { .. })
    }

    /// 判断数据域是否为空
    pub fn is_empty(&self) -> bool {
        matches!(self.body, ApplicationBody::Empty)
    }
}

/// 数据标识符（DA+DI的组合，类似 CSG1209022）
///
/// 在 DL/T 645-2007 中，DA 概念不明显，主要是 DI
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataId {
    /// 数据标识（4字节）
    pub di: DataIdentifier,
}

impl DataId {
    /// 创建新的数据标识符
    pub fn new(di: DataIdentifier) -> Self {
        Self { di }
    }

    /// 编码为字节
    pub fn encode(&self) -> Vec<u8> {
        self.di.as_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::FunctionCode;

    #[test]
    fn test_decode_read_request() {
        // 读数据请求：只有 DI
        let payload = vec![0x00, 0x00, 0x01, 0x00]; // DI = 00 01 00 00
        let app = ApplicationLayer::decode(
            &payload,
            FunctionCode::ReadData,
            Direction::MasterToSlave,
            false,
            "dlt645-2007",
            "国网",
        )
        .unwrap();

        assert_eq!(app.function, FunctionCode::ReadData);
        assert!(!app.is_error());
        assert!(!app.is_empty());

        match &app.body {
            ApplicationBody::ReadRequest { identifiers } => {
                assert_eq!(identifiers.len(), 1);
                assert_eq!(identifiers[0].to_u32(), 0x00010000);
            }
            _ => panic!("期望 ReadRequest"),
        }
    }

    #[test]
    fn test_decode_broadcast_time() {
        // 广播校时
        let payload = vec![0x07, 0x08, 0x14, 0x15, 0x03, 0x24]; // 2024-03-15 14:08:07
        let app = ApplicationLayer::decode(
            &payload,
            FunctionCode::BroadcastTime,
            Direction::MasterToSlave,
            false,
            "dlt645-2007",
            "国网",
        )
        .unwrap();

        assert_eq!(app.function, FunctionCode::BroadcastTime);

        match &app.body {
            ApplicationBody::BroadcastTime { time } => {
                assert_eq!(time.second, 0x07);
                assert_eq!(time.minute, 0x08);
                assert_eq!(time.hour, 0x14);
            }
            _ => panic!("期望 BroadcastTime"),
        }
    }

    #[test]
    fn test_decode_error_response() {
        // 错误响应
        let payload = vec![0x04]; // 错误码
        let app = ApplicationLayer::decode(
            &payload,
            FunctionCode::ReadData,
            Direction::SlaveToMaster,
            true, // is_error = true
            "dlt645-2007",
            "国网",
        )
        .unwrap();

        assert!(app.is_error());

        match &app.body {
            ApplicationBody::Error { error_bits } => {
                assert_eq!(*error_bits, 0x04);
            }
            _ => panic!("期望 Error"),
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // 往返测试
        let original = ApplicationLayer {
            function: FunctionCode::ReadData,
            body: ApplicationBody::ReadRequest {
                identifiers: vec![DataIdentifier::from_bytes([0x00, 0x00, 0x01, 0x00])],
            },
        };

        let encoded = original.encode();
        let decoded = ApplicationLayer::decode(
            &encoded,
            FunctionCode::ReadData,
            Direction::MasterToSlave,
            false,
            "dlt645-2007",
            "国网",
        )
        .unwrap();

        assert_eq!(decoded.function, original.function);
        assert_eq!(decoded.body, original.body);
    }
}

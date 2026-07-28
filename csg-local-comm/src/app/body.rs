//! 应用层数据体
//!
//! 根据 DI 的不同，数据体有不同的格式

use crate::app::di::DataIdentifier;
use crate::error::{Error, Result};

/// 应用层数据体
#[derive(Debug, Clone, PartialEq)]
pub enum ApplicationBody {
    /// 空数据
    Empty,

    /// 确认（无数据）
    Ack,

    /// 否认（带 1 字节错误状态字）
    Nack { error_code: u8 },

    /// 带数据内容（原始字节 + spec-engine 解析结果）
    WithData {
        raw_data: Vec<u8>,
        parsed_value: Option<proto_common::FieldValue>,
    },
}

impl ApplicationBody {
    /// 从原始字节解析数据体
    ///
    /// # 参数
    ///
    /// - `buf`: 数据内容字节
    /// - `di`: 数据标识
    pub fn parse(buf: &[u8], di: &DataIdentifier) -> Result<Self> {
        // 先检查确认/否认（根据 DI 判断），即使数据为空
        if di.is_ack() {
            return Ok(ApplicationBody::Ack);
        }

        if di.is_nack() {
            if buf.len() != 1 {
                return Err(Error::InvalidDataFormat {
                    reason: format!("否认帧应包含 1 字节错误码，实际 {} 字节", buf.len()),
                });
            }
            return Ok(ApplicationBody::Nack {
                error_code: buf[0],
            });
        }

        // 然后检查是否为空数据
        if buf.is_empty() {
            return Ok(ApplicationBody::Empty);
        }

        // 其他数据，保留原始字节，等待 spec-engine 解析
        Ok(ApplicationBody::WithData {
            raw_data: buf.to_vec(),
            parsed_value: None,
        })
    }

    /// 编码为字节
    pub fn encode(&self) -> Vec<u8> {
        match self {
            ApplicationBody::Empty => Vec::new(),
            ApplicationBody::Ack => Vec::new(),
            ApplicationBody::Nack { error_code } => vec![*error_code],
            ApplicationBody::WithData { raw_data, .. } => raw_data.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::di::NodeRole;

    #[test]
    fn test_ack_body() {
        let di = DataIdentifier::ack(NodeRole::Concentrator);
        let body = ApplicationBody::parse(&[], &di).unwrap();
        assert_eq!(body, ApplicationBody::Ack);
    }

    #[test]
    fn test_nack_body() {
        let di = DataIdentifier::nack(NodeRole::Concentrator);
        let body = ApplicationBody::parse(&[0x01], &di).unwrap();
        assert_eq!(
            body,
            ApplicationBody::Nack { error_code: 0x01 }
        );
    }

    #[test]
    fn test_with_data_body() {
        let di = DataIdentifier {
            node_role: NodeRole::Concentrator,
            direction: crate::app::di::MessageDirection::BothSameFormat,
            afn_match: 0x03,
            sub_function: 0x01,
        };
        let body = ApplicationBody::parse(&[0x01, 0x02, 0x03], &di).unwrap();
        match body {
            ApplicationBody::WithData { raw_data, .. } => {
                assert_eq!(raw_data, vec![0x01, 0x02, 0x03]);
            }
            _ => panic!("Expected WithData"),
        }
    }
}

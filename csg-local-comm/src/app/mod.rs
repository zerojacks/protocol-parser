//! 应用层模块
//!
//! 负责 AFN、SEQ、DI、数据内容的解析

pub mod afn;
pub mod body;
pub mod di;

use crate::control::Direction;
use crate::error::{Error, Result};
pub use afn::Afn;
pub use body::ApplicationBody;
pub use di::{DataIdentifier, MessageDirection, NodeRole};
use proto_common::from_spec_engine;
use std::sync::OnceLock;

fn spec_engine() -> &'static spec_engine::Engine {
    static ENGINE: OnceLock<spec_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(spec_engine::Engine::new_default)
}

/// 应用层
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationLayer {
    /// 应用功能码 AFN
    pub afn: Afn,
    /// 帧序号 SEQ（1字节，0-255 循环）
    pub seq: u8,
    /// 数据标识 DI（4字节）
    pub di: DataIdentifier,
    /// 数据体
    pub body: ApplicationBody,
}

impl ApplicationLayer {
    /// 从用户数据区解析应用层
    ///
    /// # 参数
    ///
    /// - `payload`: 用户数据区字节（AFN + SEQ + DI + 数据内容）
    /// - `direction`: 传输方向（用于 spec-engine）
    /// - `protocol`: 协议标识（传给 spec-engine）
    /// - `region`: 区域标识（传给 spec-engine）
    pub fn decode(
        payload: &[u8],
        direction: Direction,
        protocol: &str,
        region: &str,
    ) -> Result<Self> {
        if payload.len() < 6 {
            // 至少需要 AFN(1) + SEQ(1) + DI(4)
            return Err(Error::UnexpectedEof {
                needed: 6,
                actual: payload.len(),
            });
        }

        let afn = Afn::from_byte(payload[0]);
        let seq = payload[1];
        let di = DataIdentifier::decode(&payload[2..6])?;

        // 数据内容从第 6 字节开始
        let data_content = &payload[6..];

        // 解析数据体
        let mut body = ApplicationBody::parse(data_content, &di)?;

        // 如果有数据内容，使用 spec-engine 解析
        if let ApplicationBody::WithData {
            ref raw_data,
            ref mut parsed_value,
        } = body
        {
            // 调用 spec-engine 解析数据标识内容
            // spec_engine::parse_di(protocol: &str, di: u32, region: &str, endian: Option<&str>, data: &[u8])
            let dir_str = match direction {
                Direction::Downlink => Some("0"),
                Direction::Uplink => Some("1"),
            };

            let di_u32 = di.to_u32();

            match spec_engine().parse_di(protocol, di_u32, region, dir_str, raw_data) {
                Ok((value, _consumed)) => {
                    *parsed_value = Some(from_spec_engine(&value));
                }
                Err(e) => {
                    // spec-engine 解析失败，保留原始数据
                    *parsed_value = Some(proto_common::FieldValue::Invalid {
                        reason: format!("spec-engine 解析错误: {}", e),
                    });
                }
            }
        }

        Ok(Self {
            afn,
            seq,
            di,
            body,
        })
    }

    /// 编码为用户数据区字节
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();

        // AFN
        buf.push(self.afn.to_byte());

        // SEQ
        buf.push(self.seq);

        // DI
        buf.extend_from_slice(&self.di.to_bytes());

        // 数据体
        buf.extend_from_slice(&self.body.encode());

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_layer_decode_ack() {
        // AFN=00H, SEQ=01, DI=01 00 01 E8 (确认，小端序：DI0 DI1 DI2 DI3)
        // DI3=E8(集中器), DI2=01(相同格式), DI1=00, DI0=01(确认)
        let payload = vec![0x00, 0x01, 0x01, 0x00, 0x01, 0xE8];
        let app = ApplicationLayer::decode(&payload, Direction::Uplink, "csg-local-comm", "南网")
            .unwrap();

        assert_eq!(app.afn, Afn::AckNack);
        assert_eq!(app.seq, 0x01);
        assert!(app.di.is_ack());
        assert_eq!(app.body, ApplicationBody::Ack);
    }

    #[test]
    fn test_application_layer_decode_nack() {
        // AFN=00H, SEQ=02, DI=02 00 01 E8 (否认，小端序), 错误码=0x05
        // DI3=E8(集中器), DI2=01(相同格式), DI1=00, DI0=02(否认)
        let payload = vec![0x00, 0x02, 0x02, 0x00, 0x01, 0xE8, 0x05];
        let app = ApplicationLayer::decode(&payload, Direction::Uplink, "csg-local-comm", "南网")
            .unwrap();

        assert_eq!(app.afn, Afn::AckNack);
        assert_eq!(app.seq, 0x02);
        assert!(app.di.is_nack());
        assert_eq!(
            app.body,
            ApplicationBody::Nack { error_code: 0x05 }
        );
    }

    #[test]
    fn test_application_layer_encode_decode_roundtrip() {
        let app = ApplicationLayer {
            afn: Afn::QueryParam,
            seq: 0x10,
            di: DataIdentifier {
                node_role: NodeRole::Concentrator,
                direction: MessageDirection::BothSameFormat,
                afn_match: 0x03,
                sub_function: 0x01,
            },
            body: ApplicationBody::Empty,
        };

        let bytes = app.encode().unwrap();
        // bytes应该是: [03, 10, 01, 03, 01, E8] (AFN, SEQ, DI0-DI3)
        let app2 = ApplicationLayer::decode(&bytes, Direction::Downlink, "csg-local-comm", "南网")
            .unwrap();

        assert_eq!(app.afn, app2.afn);
        assert_eq!(app.seq, app2.seq);
        assert_eq!(app.di, app2.di);
    }
}

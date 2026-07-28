//! 顶层引擎模块
//!
//! 提供完整的报文编解码接口，整合链路层和应用层

use crate::app::ApplicationLayer;
use crate::error::Result;
use crate::link::Frame;

/// 完整的报文消息
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// 链路层帧
    pub frame: Frame,
    /// 应用层
    pub app: ApplicationLayer,
}

impl Message {
    /// 从原始字节解码完整报文
    ///
    /// # 参数
    ///
    /// - `buf`: 完整的帧字节（包括起始符和结束符）
    /// - `protocol`: 协议标识（传给 spec-engine，默认 "csg-local-comm"）
    /// - `region`: 区域标识（传给 spec-engine，默认 "南网"）
    pub fn decode(buf: &[u8], protocol: Option<&str>, region: Option<&str>) -> Result<(Self, usize)> {
        let protocol = protocol.unwrap_or("csg16");
        let region = region.unwrap_or("南网");

        // 1. 解析链路层
        let (frame, consumed) = Frame::decode(buf)?;

        // 2. 获取传输方向
        let direction = frame.control.direction;

        // 3. 解析应用层
        let app = ApplicationLayer::decode(&frame.payload, direction, protocol, region)?;

        Ok((Self { frame, app }, consumed))
    }

    /// 编码为完整的原始字节
    pub fn encode(&self) -> Result<Vec<u8>> {
        // 1. 编码应用层得到用户数据区
        let payload = self.app.encode()?;

        // 2. 构建链路层帧
        let mut frame = self.frame.clone();
        frame.payload = payload;

        // 3. 编码链路层
        frame.encode()
    }
}

/// 便捷函数：解码报文
pub fn decode_message(buf: &[u8]) -> Result<(Message, usize)> {
    Message::decode(buf, None, None)
}

/// 便捷函数：编码报文
pub fn encode_message(msg: &Message) -> Result<Vec<u8>> {
    msg.encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Afn, ApplicationBody, ApplicationLayer, DataIdentifier, NodeRole};
    use crate::control::ControlByte;
    use crate::link::{Address, AddressDomain, Frame};

    #[test]
    fn test_decode_encode_roundtrip() {
        // 构建一个简单的查询参数报文（下行）
        let frame = Frame {
            control: ControlByte::downlink_primary_with_address(),
            address: Some(AddressDomain {
                source: Address::from_bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
                destination: Address::from_bytes([0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C]),
            }),
            payload: vec![
                0x03, // AFN = QueryParam
                0x01, // SEQ = 1
                0xE8, 0x01, 0x03, 0x01, // DI
            ],
        };

        let app = ApplicationLayer {
            afn: Afn::QueryParam,
            seq: 0x01,
            di: DataIdentifier {
                node_role: NodeRole::Concentrator,
                direction: crate::app::di::MessageDirection::BothSameFormat,
                afn_match: 0x03,
                sub_function: 0x01,
            },
            body: ApplicationBody::Empty,
        };

        let msg = Message { frame, app };

        // 编码
        let encoded = msg.encode().unwrap();

        // 解码
        let (decoded, _) = Message::decode(&encoded, None, None).unwrap();

        // 验证
        assert_eq!(msg.app.afn, decoded.app.afn);
        assert_eq!(msg.app.seq, decoded.app.seq);
        assert_eq!(msg.app.di, decoded.app.di);
    }
}

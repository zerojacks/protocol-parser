//! 顶层入口：把链路层 `Frame` 和应用层 `ApplicationLayer` 串起来。

use crate::app::ApplicationLayer;
use crate::error::Result;
use crate::link::Frame;
use proto_common::FieldValue;

/// 一条完整解析出来的报文：链路层帧 + 已解析的应用层内容。
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub frame: Frame,
    pub application: ApplicationLayer,
}

impl Message {
    /// 将报文渲染成树形 FieldValue 结构，适合前端 UI 展示
    ///
    /// # 参数
    ///
    /// - `protocol`: 协议标识（如 "csg13"）
    /// - `region`: 区域标识（如 "南网"）
    ///
    /// # 返回
    ///
    /// 根节点为 "Q/CSG1209022-2019 报文" 的 FieldValue 树，包含所有字段的 name/raw/value 信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (msg, consumed) = decode_message(&bytes, "csg13", "南网")?;
    /// let tree = msg.to_value_tree("csg13", "南网")?;
    /// // 现在可以序列化 tree 发送给前端，或递归遍历展示
    /// ```
    pub fn to_value_tree(&self, protocol: &str, region: &str) -> Result<FieldValue> {
        crate::report::render_message_as_value(self, protocol, region)
    }
}

/// 从字节流解码一条完整报文（链路层 + 应用层），返回解析结果和消耗的字节数。
///
/// `protocol`/`region` 透传给 spec-engine 做 DI 内容解析，例如 `decode_message(buf, "csg13", "南网")`。
pub fn decode_message(buf: &[u8], protocol: &str, region: &str) -> Result<(Message, usize)> {
    let (frame, consumed) = Frame::decode(buf)?;
    let application =
        ApplicationLayer::decode(&frame.payload, frame.control.direction, protocol, region)?;
    Ok((Message { frame, application }, consumed))
}

/// 将链路层帧和应用层内容重新编码为完整字节序列。
pub fn encode_message(frame_without_payload: &Frame, application: &ApplicationLayer) -> Result<Vec<u8>> {
    let mut frame = frame_without_payload.clone();
    frame.payload = application.encode()?;
    frame.encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ack::DI_ACK_ALL, Afn, ApplicationBody, DataAddress, DataIdentifier, SeqField};
    use crate::link::address::RegionCode;
    use crate::link::{AddressField, ControlField};

    fn sample_address() -> AddressField {
        AddressField {
            region: RegionCode {
                province: 44,
                city: 1,
                county: 5,
            },
            terminal_addr: 123_456,
            master_addr: 1,
        }
    }

    /// Ack/ReadCurrentDataResponse 的内容要经过 spec_engine::parse_di 解析，
    /// spec-engine 目前没有编码接口，所以这类"带内容"的报文只测解码方向，
    /// 手工拼出原始字节（和 tests/real_frame.rs 里的做法一致）。
    #[test]
    fn end_to_end_decode_ack_all() {
        let mut payload = vec![Afn::Ack.to_u8(), SeqField::single_frame(false, 0).unwrap().encode()];
        payload.extend_from_slice(&DataAddress::terminal().encode());
        payload.extend_from_slice(&DI_ACK_ALL.to_le_bytes());
        payload.push(0x00); // 内容：0=确认

        let frame = Frame {
            control: ControlField::new_up(false, false, false, 8).unwrap(),
            address: sample_address(),
            payload,
        };
        let bytes = frame.encode().unwrap();

        let (msg, consumed) = decode_message(&bytes, "csg13", "南网").unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(msg.application.afn, Afn::Ack);
        match &msg.application.body {
            ApplicationBody::Ack(units) => {
                assert_eq!(units.len(), 1);
                assert_eq!(units[0].di, DI_ACK_ALL);
            }
            other => panic!("期望 Ack，实际 {other:?}"),
        }
    }

    /// 不带内容、只有 DA+DI 的请求报文两个方向都支持编解码，用它测完整的
    /// `encode_message` -> `decode_message` 往返。
    #[test]
    fn end_to_end_encode_decode_read_current_data_request() {
        let id = DataIdentifier {
            da: DataAddress::from_point_number(1).unwrap(),
            di: 0x0001_0000,
        };
        let application = ApplicationLayer {
            afn: Afn::ReadCurrentData,
            seq: SeqField::single_frame(true, 0).unwrap(),
            body: ApplicationBody::ReadCurrentDataRequest(vec![id]),
            tp: None,
        };
        let frame = Frame {
            control: ControlField::new_down(true, false, true, 11).unwrap(),
            address: sample_address(),
            payload: Vec::new(), // 由 encode_message 填充
        };

        let bytes = encode_message(&frame, &application).unwrap();
        let (decoded, consumed) = decode_message(&bytes, "csg13", "南网").unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.application, application);
        assert_eq!(decoded.frame.address, frame.address);
    }
}

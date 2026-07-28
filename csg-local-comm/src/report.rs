//! 报文渲染模块
//!
//! 将解析后的报文渲染成树形结构，用于前端展示

use crate::engine::Message;
use crate::error::Result;
use proto_common::FieldValue as Value;

/// 将已解析的 Message 渲染成一棵 `Value` 树
///
/// 提供三列格式的数据结构：
/// - name: 字段名
/// - raw: 原始字节（十六进制）
/// - value: 解析后的值
pub fn render_message_as_value(msg: &Message) -> Result<Value> {
    let mut fields = Vec::new();

    // 起始符 68H
    fields.push(leaf("起始符", vec![0x68], "68H".to_string()));

    // 控制字
    let control_byte = msg.frame.control.to_byte();
    let direction_str = match msg.frame.control.direction {
        crate::control::Direction::Downlink => "下行",
        crate::control::Direction::Uplink => "上行",
    };
    fields.push(leaf(
        "控制字",
        vec![control_byte],
        format!("{} ({:02X}H)", direction_str, control_byte),
    ));

    // 地址域（如果存在，12字节）
    if let Some(ref addr) = msg.frame.address {
        let addr_bytes = addr.encode();
        let addr_desc = if addr.source.is_broadcast() || addr.destination.is_broadcast() {
            "广播地址".to_string()
        } else {
            format!("源地址:{:?}, 目标地址:{:?}", addr.source, addr.destination)
        };
        fields.push(leaf("地址域", addr_bytes.to_vec(), addr_desc));
    }

    // AFN（应用功能码）
    let afn_byte = msg.app.afn.to_byte();
    fields.push(leaf(
        "AFN",
        vec![afn_byte],
        format!("{:02X}H - {}", afn_byte, msg.app.afn.description()),
    ));

    // SEQ（帧序号）
    fields.push(leaf("SEQ", vec![msg.app.seq], format!("{}", msg.app.seq)));

    // DI（数据标识，4字节）
    let di_bytes = msg.app.di.to_bytes();
    let di_desc = format!(
        "DI={:08X}H (角色:{:?}, 方向:{:?})",
        msg.app.di.to_u32(),
        msg.app.di.node_role,
        msg.app.di.direction
    );
    fields.push(leaf("DI", di_bytes.to_vec(), di_desc));

    // 数据内容
    match &msg.app.body {
        crate::app::ApplicationBody::Empty => {
            fields.push(leaf("数据内容", vec![], "无".to_string()));
        }
        crate::app::ApplicationBody::Ack => {
            fields.push(leaf("数据内容", vec![], "确认 (ACK)".to_string()));
        }
        crate::app::ApplicationBody::Nack { error_code } => {
            fields.push(leaf(
                "数据内容",
                vec![*error_code],
                format!("否认 (NACK) - 错误码: 0x{:02X}", error_code),
            ));
        }
        crate::app::ApplicationBody::WithData {
            raw_data,
            parsed_value,
        } => {
            // 如果 spec-engine 解析成功，展示解析结果
            if let Some(ref pv) = parsed_value {
                fields.push(Value::Node {
                    name: "数据内容".to_string(),
                    raw: raw_data.clone(),
                    value: Box::new(pv.clone()),
                });
            } else {
                // 否则只展示原始字节
                fields.push(leaf(
                    "数据内容",
                    raw_data.clone(),
                    format!("原始数据 ({} 字节)", raw_data.len()),
                ));
            }
        }
    }

    // 结束符 16H
    fields.push(leaf("结束符", vec![0x16], "16H".to_string()));

    // 返回根节点
    Ok(Value::Node {
        name: "Q/CSG1209021-2019 报文".to_string(),
        raw: vec![], // 完整原始字节由调用者决定是否填充
        value: Box::new(Value::List(fields)),
    })
}

/// 简单叶子节点：name + 原始字节 + 描述文字
fn leaf(name: &str, raw: Vec<u8>, desc: String) -> Value {
    Value::Node {
        name: name.to_string(),
        raw,
        value: Box::new(Value::Str(desc)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Afn, ApplicationBody, ApplicationLayer, DataIdentifier, NodeRole};
    use crate::control::ControlByte;
    use crate::engine::Message;
    use crate::link::Frame;

    #[test]
    fn test_render_simple_ack() {
        let frame = Frame {
            control: ControlByte::uplink_response_with_address(),
            address: None,
            payload: vec![
                0x00, // AFN = AckNack
                0x01, // SEQ = 1
                0xE8, 0x01, 0x00, 0x01, // DI = 确认
            ],
        };

        let app = ApplicationLayer {
            afn: Afn::AckNack,
            seq: 0x01,
            di: DataIdentifier::ack(NodeRole::Concentrator),
            body: ApplicationBody::Ack,
        };

        let msg = Message { frame, app };
        let tree = render_message_as_value(&msg).unwrap();

        // 验证是一个节点
        match tree {
            Value::Node { name, .. } => {
                assert_eq!(name, "Q/CSG1209021-2019 报文");
            }
            _ => panic!("Expected Node"),
        }
    }

    #[test]
    fn test_render_with_address() {
        use crate::link::{Address, AddressDomain};

        let frame = Frame {
            control: ControlByte::downlink_primary_with_address(),
            address: Some(AddressDomain {
                source: Address::from_bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
                destination: Address::from_bytes([0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C]),
            }),
            payload: vec![
                0x03, // AFN = QueryParam
                0x10, // SEQ = 16
                0xE8, 0x01, 0x03, 0x01, // DI
            ],
        };

        let app = ApplicationLayer {
            afn: Afn::QueryParam,
            seq: 0x10,
            di: DataIdentifier {
                node_role: NodeRole::Concentrator,
                direction: crate::app::di::MessageDirection::BothSameFormat,
                afn_match: 0x03,
                sub_function: 0x01,
            },
            body: ApplicationBody::Empty,
        };

        let msg = Message { frame, app };
        let tree = render_message_as_value(&msg).unwrap();

        // 验证包含地址域
        match tree {
            Value::Node { value, .. } => {
                match *value {
                    Value::List(ref fields) => {
                        let has_address = fields.iter().any(|f| match f {
                            Value::Node { name, .. } => name == "地址域",
                            _ => false,
                        });
                        assert!(has_address, "应该包含地址域");
                    }
                    _ => panic!("Expected List"),
                }
            }
            _ => panic!("Expected Node"),
        }
    }
}

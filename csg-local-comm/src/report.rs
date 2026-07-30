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

    // 控制字（展开位域）
    fields.push(control_byte_node(&msg.frame.control));

    // 地址域（如果存在，12字节）
    if let Some(ref addr) = msg.frame.address {
        fields.push(address_domain_node(addr));
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

    // DI（数据标识，4字节）- 展开显示
    fields.push(di_node(&msg.app.di));

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

/// 控制字节节点：展开显示位域
fn control_byte_node(ctrl: &crate::control::ControlByte) -> Value {
    let byte = ctrl.to_byte();
    
    let mut bits = Vec::new();
    
    // D7: 传输方向
    let dir_desc = match ctrl.direction {
        crate::control::Direction::Downlink => "下行(集中器→模块)",
        crate::control::Direction::Uplink => "上行(模块→集中器)",
    };
    bits.push(bit_node("D7传输方向位DIR", 7, 7, byte, dir_desc.to_string()));
    
    // D6: 启动标志
    let prm_desc = if ctrl.is_primary {
        "启动站发出"
    } else {
        "从动站发出"
    };
    bits.push(bit_node("D6启动标志位PRM", 6, 6, byte, prm_desc.to_string()));
    
    // D5: 地址域标志
    let add_desc = if ctrl.has_address {
        "包含地址域"
    } else {
        "无地址域"
    };
    bits.push(bit_node("D5地址域标志ADD", 5, 5, byte, add_desc.to_string()));
    
    // D4-D3: 协议版本
    bits.push(bit_node("D4~D3协议版本VER", 4, 3, byte, format!("版本={}", ctrl.version)));
    
    // D2-D0: 保留位
    bits.push(bit_node("D2~D0保留位", 2, 0, byte, "保留(固定为0)".to_string()));
    
    Value::Node {
        name: "控制字".to_string(),
        raw: vec![byte],
        value: Box::new(Value::List(bits)),
    }
}

/// 地址域节点：展开显示源地址和目标地址
fn address_domain_node(addr: &crate::link::AddressDomain) -> Value {
    let addr_bytes = addr.encode();
    
    let mut children = Vec::new();
    
    // 源地址 (前6字节)
    let source_desc = if addr.source.is_broadcast() {
        "广播地址(FFFFFFFFFFFF)".to_string()
    } else {
        format!("源地址={:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                addr_bytes[0], addr_bytes[1], addr_bytes[2],
                addr_bytes[3], addr_bytes[4], addr_bytes[5])
    };
    children.push(leaf("源地址", addr_bytes[0..6].to_vec(), source_desc));
    
    // 目标地址 (后6字节)
    let dest_desc = if addr.destination.is_broadcast() {
        "广播地址(FFFFFFFFFFFF)".to_string()
    } else {
        format!("目标地址={:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
                addr_bytes[6], addr_bytes[7], addr_bytes[8],
                addr_bytes[9], addr_bytes[10], addr_bytes[11])
    };
    children.push(leaf("目标地址", addr_bytes[6..12].to_vec(), dest_desc));
    
    Value::Node {
        name: "地址域".to_string(),
        raw: addr_bytes.to_vec(),
        value: Box::new(Value::List(children)),
    }
}

/// DI（数据标识）节点：展开显示各个字段（中文输出）
fn di_node(di: &crate::app::DataIdentifier) -> Value {
    let di_bytes = di.to_bytes();
    
    let mut children = Vec::new();
    
    // 节点角色 (中文)
    let role_desc = match di.node_role {
        crate::app::di::NodeRole::Concentrator => "集中器".to_string(),
        crate::app::di::NodeRole::Collector => "采集器".to_string(),
        crate::app::di::NodeRole::Other(byte) => format!("其他(0x{:02X})", byte),
    };
    children.push(leaf("节点角色", vec![], role_desc));
    
    // 消息方向 (中文)
    let direction_desc = match di.direction {
        crate::app::di::MessageDirection::BothNoDownlinkData => "上下行都用,下行无数据内容".to_string(),
        crate::app::di::MessageDirection::BothSameFormat => "上下行都用,数据内容格式相同".to_string(),
        crate::app::di::MessageDirection::DownlinkOnlyAckResponse => "仅下行,对应上行为确认/否认".to_string(),
        crate::app::di::MessageDirection::DownlinkWithData => "仅下行,带数据".to_string(),
        crate::app::di::MessageDirection::UplinkWithData => "仅上行,带数据".to_string(),
        crate::app::di::MessageDirection::UplinkOnlyAckResponse => "仅上行,对应下行为确认/否认".to_string(),
        crate::app::di::MessageDirection::BothNoUplinkData => "上下行都用,上行无数据内容".to_string(),
        crate::app::di::MessageDirection::Other(byte) => format!("其他(0x{:02X})", byte),
    };
    children.push(leaf("消息方向", vec![], direction_desc));
    
    // AFN 匹配
    children.push(leaf(
        "AFN匹配",
        vec![],
        format!("AFN={:02X}H", di.afn_match),
    ));
    
    // 子功能
    children.push(leaf(
        "子功能",
        vec![],
        format!("0x{:02X}", di.sub_function),
    ));
    
    Value::Node {
        name: "DI".to_string(),
        raw: di_bytes.to_vec(),
        value: Box::new(Value::List(children)),
    }
}

/// 位字段节点
fn bit_node(name: &str, bit_start: usize, bit_end: usize, byte: u8, desc: String) -> Value {
    let bit_value = extract_bits(byte, bit_start, bit_end);
    Value::Node {
        name: name.to_string(),
        raw: Vec::new(),
        value: Box::new(Value::Bit {
            bit_start,
            bit_end,
            bit_value,
            bit_byte: vec![byte],
            value: Some(Box::new(Value::Str(desc))),
        }),
    }
}

fn extract_bits(byte: u8, bit_start: usize, bit_end: usize) -> u64 {
    let width = bit_start - bit_end + 1;
    let mask: u64 = if width >= 8 { 0xFF } else { (1u64 << width) - 1 };
    ((byte as u64) >> bit_end) & mask
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

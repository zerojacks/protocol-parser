//! 把一整条 DL/T 645-2007 报文（链路层 + 应用层 + 数据内容）渲染成一棵 [`proto_common::FieldValue`] 树。
//!
//! 目的：上层 UI 只需要认识 `FieldValue::Node{name, raw, value}` 这一种"三列表格"形状
//! （帧域名称 / 原始字节 / 解析值），就能把链路层字段和数据标识内容用同一套渲染逻辑
//! 展示成一张表——不需要为"帧层字段"和"数据内容"分别写两套展示代码。
//!
//! spec-engine 解析后的数据标识内容已经是 `FieldValue` 树了，这里只需要把链路层字段
//! （地址、控制码）和应用层字段（功能码、数据长度）也包装成相同的 `Node` 格式，
//! 然后组合成完整的报文树。

use crate::address::Address;
use crate::app::{ApplicationBody, ApplicationLayer, DataItem};
use crate::control::{ControlCode, Direction, FunctionCode};
use crate::engine::{decode_message, Message};
use crate::error::Result;
use proto_common::FieldValue as Value;

/// 解析一条完整报文并渲染成一棵 `Value` 树，返回渲染结果和消耗的字节数。
///
/// 根节点 `name` 固定为 `"报文"`，`raw` 是整条报文的原始字节，`value` 是按标准顺序
/// 排列的字段列表（`Value::List`）：起始符/地址域/起始符/控制码/数据长度/数据域/校验码/结束符。
pub fn decode_message_as_value(
    buf: &[u8],
    protocol: &str,
    region: &str,
) -> Result<(Value, usize)> {
    let (msg, consumed) = decode_message(buf, protocol, region)?;
    let tree = render_message_as_value(&msg, &buf[..consumed])?;
    Ok((tree, consumed))
}

/// 将已解析的 Message 渲染成一棵 `Value` 树
///
/// 这是 `Message::to_value_tree()` 的内部实现函数。
pub fn render_message_as_value(msg: &Message, raw_bytes: &[u8]) -> Result<Value> {
    let mut rows = Vec::new();

    // 起始符 68H
    rows.push(leaf("起始符", vec![raw_bytes[0]], "起始符 68H".to_string()));

    // 地址域 A0~A5 (6字节)
    rows.push(address_node(&msg.frame.address, &raw_bytes[1..7]));

    // 起始符 68H
    rows.push(leaf("起始符", vec![raw_bytes[7]], "起始符 68H".to_string()));

    // 控制码 C
    rows.push(control_code_node(&msg.frame.control, raw_bytes[8]));

    // 数据长度 L
    let l = msg.frame.payload.len();
    rows.push(leaf(
        "数据长度",
        vec![raw_bytes[9]],
        format!("数据长度={} 字节", l),
    ));

    // 数据域
    rows.push(application_node(&msg.application, &raw_bytes[10..10 + l]));

    // 校验码 CS
    let cs_index = raw_bytes.len() - 2;
    rows.push(leaf(
        "校验码",
        vec![raw_bytes[cs_index]],
        "校验正确".to_string(),
    ));

    // 结束符 16H
    rows.push(leaf("结束符", vec![raw_bytes[raw_bytes.len() - 1]], "结束符 16H".to_string()));

    Ok(Value::Node {
        name: "报文".to_string(),
        raw: raw_bytes.to_vec(),
        value: Box::new(Value::List(rows)),
    })
}

/// 简单叶子节点：`name` + 原始字节 + 一段人类可读的描述文字。
fn leaf(name: &str, raw: Vec<u8>, desc: String) -> Value {
    Value::Node {
        name: name.to_string(),
        raw,
        value: Box::new(Value::Str(desc)),
    }
}

/// 地址域节点：展示 BCD 格式的地址
fn address_node(addr: &Address, raw: &[u8]) -> Value {
    let desc = if addr.is_broadcast() {
        "广播地址 999999999999".to_string()
    } else {
        format!("地址={}", addr.to_decimal_string())
    };

    Value::Node {
        name: "地址域".to_string(),
        raw: raw.to_vec(),
        value: Box::new(Value::Str(desc)),
    }
}

/// 控制码节点：展示方向、功能码、从站状态等信息
fn control_code_node(ctrl: &ControlCode, raw: u8) -> Value {
    let direction_str = match ctrl.direction {
        Direction::MasterToSlave => "主站→从站",
        Direction::SlaveToMaster => "从站→主站",
    };

    let function_desc = match ctrl.function {
        FunctionCode::ReadData => "读数据",
        FunctionCode::ReadFollowUp => "读后续数据",
        FunctionCode::ReadAddress => "读通信地址",
        FunctionCode::WriteData => "写数据",
        FunctionCode::WriteAddress => "写通信地址",
        FunctionCode::BroadcastTime => "广播校时",
        FunctionCode::Freeze => "冻结命令",
        FunctionCode::ChangeBaudRate => "更改通信速率",
        FunctionCode::ChangePassword => "修改密码",
        FunctionCode::ClearMaxDemand => "最大需量清零",
        FunctionCode::ClearMeter => "电表清零",
        FunctionCode::ClearEvent => "事件清零",
        FunctionCode::Reserved => "保留",
    };

    let status_desc = if ctrl.direction == Direction::SlaveToMaster {
        match ctrl.slave_status {
            crate::control::SlaveStatus::Normal => "从站正常",
            crate::control::SlaveStatus::Abnormal => "从站异常",
        }
    } else {
        "主站发起"
    };

    let desc = format!(
        "控制码={:02X}H, 方向={}, 功能={}, 状态={}",
        raw, direction_str, function_desc, status_desc
    );

    Value::Node {
        name: "控制码".to_string(),
        raw: vec![raw],
        value: Box::new(Value::Str(desc)),
    }
}

/// 应用层节点：展示功能码和数据域
fn application_node(app: &ApplicationLayer, raw: &[u8]) -> Value {
    let function_desc = match app.function {
        FunctionCode::ReadData => "读数据",
        FunctionCode::ReadFollowUp => "读后续数据",
        FunctionCode::ReadAddress => "读通信地址",
        FunctionCode::WriteData => "写数据",
        FunctionCode::WriteAddress => "写通信地址",
        FunctionCode::BroadcastTime => "广播校时",
        FunctionCode::Freeze => "冻结命令",
        FunctionCode::ChangeBaudRate => "更改通信速率",
        FunctionCode::ChangePassword => "修改密码",
        FunctionCode::ClearMaxDemand => "最大需量清零",
        FunctionCode::ClearMeter => "电表清零",
        FunctionCode::ClearEvent => "事件清零",
        FunctionCode::Reserved => "保留",
    };

    let body_node = application_body_node(&app.body);

    Value::Node {
        name: "数据域".to_string(),
        raw: raw.to_vec(),
        value: Box::new(Value::List(vec![
            leaf("功能码", vec![], function_desc.to_string()),
            body_node,
        ])),
    }
}

/// 应用层数据体节点：根据不同的数据类型展示不同的内容
fn application_body_node(body: &ApplicationBody) -> Value {
    match body {
        ApplicationBody::ReadRequest { identifiers } => {
            let mut items = Vec::new();
            for (i, di) in identifiers.iter().enumerate() {
                items.push(leaf(
                    &format!("数据标识 {}", i + 1),
                    vec![],
                    format!("DI={:08X}H", di.to_u32()),
                ));
            }
            Value::Node {
                name: "读数据请求".to_string(),
                raw: vec![],
                value: Box::new(Value::List(items)),
            }
        }
        ApplicationBody::ReadResponse { items } => {
            let mut rows = Vec::new();
            for (i, item) in items.iter().enumerate() {
                rows.push(data_item_node(i + 1, item));
            }
            Value::Node {
                name: "读数据应答".to_string(),
                raw: vec![],
                value: Box::new(Value::List(rows)),
            }
        }
        ApplicationBody::WriteRequest { identifier, password, operator_code, data } => {
            let items = vec![
                leaf(
                    "数据标识",
                    vec![],
                    format!("DI={:08X}H", identifier.to_u32()),
                ),
                leaf(
                    "密码",
                    vec![],
                    format!("权限={:X}H, 值={:02X}{:02X}{:02X}H", 
                           password.permission_level, 
                           password.value[0], password.value[1], password.value[2]),
                ),
                leaf(
                    "操作者代码",
                    vec![],
                    format!("{:02X}{:02X}{:02X}{:02X}H", 
                           operator_code.code[0], operator_code.code[1], 
                           operator_code.code[2], operator_code.code[3]),
                ),
                leaf(
                    "数据",
                    data.clone(),
                    format!("{} 字节", data.len()),
                ),
            ];
            Value::Node {
                name: "写数据请求".to_string(),
                raw: vec![],
                value: Box::new(Value::List(items)),
            }
        }
        ApplicationBody::WriteResponse => leaf("写数据应答", vec![], "写数据成功".to_string()),
        ApplicationBody::BroadcastTime { time } => Value::Node {
            name: "广播校时".to_string(),
            raw: vec![],
            value: Box::new(Value::Str(format!(
                "时间={:02}/{:02}/{:02} {:02}:{:02}:{:02}",
                time.year, time.month, time.day, time.hour, time.minute, time.second
            ))),
        },
        ApplicationBody::Freeze { freeze_time } => Value::Node {
            name: "冻结命令".to_string(),
            raw: vec![],
            value: Box::new(Value::Str(format!(
                "冻结时间={:02}:{:02} {:02}日",
                freeze_time.minute, freeze_time.hour, freeze_time.day
            ))),
        },
        ApplicationBody::Error { error_bits } => leaf(
            "异常应答",
            vec![],
            format!("错误代码={:02X}H", error_bits),
        ),
        ApplicationBody::Raw(data) => leaf(
            "原始数据",
            data.clone(),
            format!("{} 字节（未解析）", data.len()),
        ),
        ApplicationBody::Empty => leaf("空数据域", vec![], "无数据".to_string()),
    }
}

/// 数据项节点：展示数据标识、原始数据和 spec-engine 解析结果
fn data_item_node(index: usize, item: &DataItem) -> Value {
    let di_str = format!("DI={:08X}H", item.identifier.to_u32());
    let raw_len = item.raw_data.len();

    let mut children = vec![
        leaf("数据标识", vec![], di_str.clone()),
        leaf("数据长度", vec![], format!("{} 字节", raw_len)),
        leaf(
            "原始数据",
            item.raw_data.clone(),
            format!("{:02X?}", item.raw_data),
        ),
    ];

    // 如果有 spec-engine 解析结果，直接嵌入到树中
    if let Some(ref parsed) = item.parsed_value {
        children.push(Value::Node {
            name: "解析结果".to_string(),
            raw: vec![],
            value: Box::new(parsed.clone()),
        });
    } else {
        children.push(leaf(
            "解析结果",
            vec![],
            "未解析（无对应字典定义）".to_string(),
        ));
    }

    Value::Node {
        name: format!("数据项 {}", index),
        raw: vec![],
        value: Box::new(Value::List(children)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_read_request_as_value_tree() {
        // 使用 parse_real_frame 示例中的真实报文
        let frame = vec![
            0x68, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x68, 
            0x91, 0x12, 0x32, 0x38, 0x33, 0x37, 0x33, 0x34,
            0x33, 0x35, 0x33, 0x36, 0x33, 0x37, 0x33, 0x38, 
            0x33, 0x39, 0x33, 0x3A, 0x2E, 0x16
        ];
        let (tree, consumed) = decode_message_as_value(&frame, "dlt645-2007", "南网").unwrap();
        assert_eq!(consumed, 30);

        if let Value::Node { name, raw, value } = tree {
            assert_eq!(name, "报文");
            assert_eq!(raw.len(), 30);
            if let Value::List(rows) = *value {
                assert!(!rows.is_empty());
                // 至少应该有：起始符、地址、起始符、控制码、数据长度、数据域、校验码、结束符
                assert!(rows.len() >= 8);
            } else {
                panic!("Expected List of rows");
            }
        } else {
            panic!("Expected Node as root");
        }
    }
}

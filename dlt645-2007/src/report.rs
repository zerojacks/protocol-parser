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
use std::sync::OnceLock;

fn spec_engine() -> &'static spec_engine::Engine {
    static ENGINE: OnceLock<spec_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(spec_engine::Engine::new_default)
}

/// 查询 DI 的名称
///
/// 使用 spec_engine 的 lookup_di 方法查询数据标识的名称
fn lookup_di_name(protocol: &str, region: &str, di_hex: &str, dir: Option<&str>) -> Result<String> {
    // 将十六进制字符串转换为 u32
    let di_u32 = u32::from_str_radix(di_hex, 16)
        .map_err(|_| crate::error::Error::InvalidDataFormat {
            reason: format!("无效的 DI 十六进制字符串: {}", di_hex),
        })?;
    // 使用 spec_engine 查询 DI 定义
    match spec_engine().lookup(protocol, di_u32, region, dir) {
        Some(filed) => {
            Ok(filed.name)
        }
        _ => {
            // 查询失败，返回错误
            Err(crate::error::Error::InvalidDataFormat {
                reason: format!("DI {} 在 {} {} 中未定义", di_hex, protocol, region),
            })
        }
    }
}

impl Message {
    /// 生成报文的一句话摘要，形如 "读数据 · (当前)组合有功总电能 等3项"。
    ///
    /// 从结构化的 [`Message`]（功能码枚举 + 应用层数据体）直接提取，
    /// 而不是从渲染用的 FieldValue 树里做字符串匹配——摘要结构稳定，
    /// 不受展示树节点命名调整的影响。供日志列表等只需要一行概览的
    /// 场景使用。
    pub fn summary(&self, protocol: &str, region: &str) -> String {
        let function = self.application.function.description();
        let identifiers: Vec<crate::data_identifier::DataIdentifier> = match &self.application.body {
            ApplicationBody::ReadRequest { identifiers } => identifiers.clone(),
            ApplicationBody::ReadResponse { items } => {
                items.iter().map(|item| item.identifier).collect()
            }
            ApplicationBody::WriteRequest { identifier, .. } => vec![*identifier],
            _ => Vec::new(),
        };
        match summarize_identifiers(&identifiers, protocol, region) {
            Some(item) => format!("{function} · {item}"),
            None => function.to_string(),
        }
    }
}

/// 摘要里的数据项部分：第一个 DI 的名称，多个 DI 时追加 "等N项"。
/// 名称查不到（规范未定义）时退回 "DI=XXXXXXXXH"。
fn summarize_identifiers(
    identifiers: &[crate::data_identifier::DataIdentifier],
    protocol: &str,
    region: &str,
) -> Option<String> {
    let first = identifiers.first()?;
    let di_hex = format!("{:08X}", first.to_u32());
    let name =
        lookup_di_name(protocol, region, &di_hex, Some("0")).unwrap_or_else(|_| format!("DI={di_hex}H"));
    Some(if identifiers.len() > 1 {
        format!("{name} 等{}项", identifiers.len())
    } else {
        name
    })
}

/// 解析一条完整报文并渲染成一棵 `Value` 树，返回渲染结果和消耗的字节数。
///
/// 根节点 `name` 固定为 `"DL/T 645-2007 报文"`，`raw` 是整条报文的原始字节，`value` 是按标准顺序
/// 排列的字段列表（`Value::List`）：起始符/地址域/起始符/控制码/数据长度/数据域/校验码/结束符。
pub fn decode_message_as_value(
    buf: &[u8],
    protocol: &str,
    region: &str,
) -> Result<(Value, usize)> {
    let (msg, consumed) = decode_message(buf, protocol, region)?;
    let tree = render_message_as_value(&msg, protocol, region)?;
    Ok((tree, consumed))
}

/// 将已解析的 Message 渲染成一棵 `Value` 树
///
/// 这是 `Message::to_value_tree()` 的内部实现函数。
pub fn render_message_as_value(msg: &Message, protocol: &str, region: &str) -> Result<Value> {
    // 从 Message 对象重新编码生成原始字节
    let raw_bytes = msg.frame.encode(false); // false = 不包含前导字节
    
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
    rows.push(application_node(&msg.application, &raw_bytes[10..10 + l], protocol, region));

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
        name: "DL/T 645-2007 报文".to_string(),
        raw: raw_bytes.clone(),
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

/// 控制码节点：展开显示位域信息
fn control_code_node(ctrl: &ControlCode, raw: u8) -> Value {
    // D7: 方向位
    let direction_desc = match ctrl.direction {
        Direction::MasterToSlave => "方向=主站→从站",
        Direction::SlaveToMaster => "方向=从站→主站",
    };
    let d7_bit = bit_node("D7方向位", 7, 7, raw, direction_desc.to_string());

    // D6: 从站异常标志
    let status_desc = if ctrl.direction == Direction::SlaveToMaster {
        match ctrl.slave_status {
            crate::control::SlaveStatus::Normal => "从站正常",
            crate::control::SlaveStatus::Abnormal => "从站异常",
        }
    } else {
        "保留位(主站发起)"
    };
    let d6_bit = bit_node("D6从站异常标志", 6, 6, raw, status_desc.to_string());

    // D5: 后续数据帧标志
    let followup_desc = if ctrl.has_follow_up {
        "有后续数据帧"
    } else {
        "无后续数据帧"
    };
    let d5_bit = bit_node("D5后续帧标志", 5, 5, raw, followup_desc.to_string());

    // D4-D0: 功能码
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
    let d4_0_bit = bit_node("D4~D0功能码", 4, 0, raw, function_desc.to_string());

    Value::Node {
        name: "控制码".to_string(),
        raw: vec![raw],
        value: Box::new(Value::List(vec![d7_bit, d6_bit, d5_bit, d4_0_bit])),
    }
}

/// 位字段节点：跟随 CSG1209022 的约定，`Node.raw` 留空，原始字节放在 `Bit.bit_byte`
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

/// 应用层节点：直接展示数据内容
fn application_node(app: &ApplicationLayer, raw: &[u8], protocol: &str, region: &str) -> Value {
    let body_node = application_body_node(&app.body, raw, protocol, region);
    
    Value::Node {
        name: "数据域".to_string(),
        raw: raw.to_vec(),
        value: Box::new(body_node),
    }
}

/// 应用层数据体节点：根据不同的数据类型展示不同的内容
fn application_body_node(body: &ApplicationBody, _raw: &[u8], protocol: &str, region: &str) -> Value {
    match body {
        ApplicationBody::ReadRequest { identifiers } => {
            let mut items = Vec::new();
            for (i, di) in identifiers.iter().enumerate() {
                let di_hex = format!("{:08X}", di.to_u32());
                
                // 尝试使用 spec-engine 查询 DI 名称
                let desc = if let Ok(di_info) = lookup_di_name(protocol, region, &di_hex, Some("0")) {
                    format!("DI={:08X}H ({})", di.to_u32(), di_info)
                } else {
                    format!("DI={:08X}H", di.to_u32())
                };
                
                items.push(leaf(
                    &format!("数据标识 {}", i + 1),
                    di.as_bytes().to_vec(),
                    desc,
                ));
            }
            Value::List(items)
        }
        ApplicationBody::ReadResponse { items } => {
            let mut rows = Vec::new();
            for (i, item) in items.iter().enumerate() {
                rows.push(data_item_node(i + 1, item, protocol, region));
            }
            Value::List(rows)
        }
        ApplicationBody::WriteRequest { identifier, password, operator_code, data } => {
            let di_hex = format!("{:08X}", identifier.to_u32());
            // WriteRequest 也是主站发起的，方向为下行（0）
            let di_desc = if let Ok(di_info) = lookup_di_name(protocol, region, &di_hex, Some("0")) {
                format!("DI={:08X}H ({})", identifier.to_u32(), di_info)
            } else {
                format!("DI={:08X}H", identifier.to_u32())
            };
            
            let items = vec![
                leaf(
                    "数据标识",
                    identifier.as_bytes().to_vec(),
                    di_desc,
                ),
                leaf(
                    "密码",
                    vec![password.permission_level, password.value[0], password.value[1], password.value[2]],
                    format!("权限={:X}H, 值={:02X}{:02X}{:02X}H", 
                           password.permission_level, 
                           password.value[0], password.value[1], password.value[2]),
                ),
                leaf(
                    "操作者代码",
                    operator_code.code.to_vec(),
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
            Value::List(items)
        }
        ApplicationBody::WriteResponse => Value::Str("写数据成功".to_string()),
        ApplicationBody::BroadcastTime { time } => Value::Str(format!(
            "时间={:02X}-{:02X}-{:02X} {:02X}:{:02X}:{:02X}",
            time.year, time.month, time.day, time.hour, time.minute, time.second
        )),
        ApplicationBody::Freeze { freeze_time } => Value::Str(format!(
            "冻结时间={:02X}:{:02X} {:02X}日",
            freeze_time.minute, freeze_time.hour, freeze_time.day
        )),
        ApplicationBody::Error { error_bits } => Value::Str(format!("错误代码={:02X}H", error_bits)),
        ApplicationBody::Raw(data) => Value::Str(format!("{} 字节（未解析）", data.len())),
        ApplicationBody::Empty => Value::Str("无数据".to_string()),
    }
}

/// 数据项节点：展示数据标识编码及其对应的数据内容
fn data_item_node(index: usize, item: &DataItem, protocol: &str, region: &str) -> Value {
    let identifier_raw = item.identifier.as_bytes().to_vec();
    let mut raw = identifier_raw.clone();
    raw.extend_from_slice(&item.raw_data);

    let di_hex = format!("{:08X}", item.identifier.to_u32());
    let identifier_desc = if let Ok(di_name) = lookup_di_name(protocol, region, &di_hex, Some("1")) {
        format!("DI={:08X}H ({})", item.identifier.to_u32(), di_name)
    } else {
        format!("DI={:08X}H", item.identifier.to_u32())
    };
    let content = if let Some(ref parsed) = item.parsed_value {
        parsed.clone()
    } else {
        Value::Str(format!("未解析 ({} 字节)", item.raw_data.len()))
    };

    Value::Node {
        name: format!("数据项 {}", index),
        raw,
        value: Box::new(Value::List(vec![
            leaf("数据标识", identifier_raw, identifier_desc),
            Value::Node {
                name: "数据内容".to_string(),
                raw: item.raw_data.clone(),
                value: Box::new(content),
            },
        ])),
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
            assert_eq!(name, "DL/T 645-2007 报文");
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

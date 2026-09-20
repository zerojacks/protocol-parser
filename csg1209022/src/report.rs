//! 把一整条报文（链路层 + 应用层 + DI 内容）渲染成一棵 [`crate::field_value::FieldValue`] 树。
//!
//! 目的：上层 UI 只需要认识 `FieldValue::Node{name, raw, value}` 这一种"三列表格"形状
//! （帧域名称 / 原始字节 / 解析值），就能把链路层字段和 DI 内容用同一套渲染逻辑
//! 展示成一张表——不需要为"帧层字段"和"DI内容"分别写两套展示代码。
//!
//! `FieldValue` 是本 crate 自己定义、和 `spec_engine::Value` 结构镜像但类型独立的报文展示值
//! （见 `crate::field_value`）：DI 内容在 `app::body::parse_data_units` 里解析完
//! 就立刻从 `spec_engine::Value` 转换成 `FieldValue`，本文件（以及上层调用方）之后
//! 就再也看不到 `spec_engine::Value`——这样 spec-engine 自己的类型演进不会变成本 crate
//! 的破坏性变更。
//!
//! 位字段（控制域C、命令序号SEQ）沿用 spec-engine 自己对 `Value::Bit` 的约定：
//! 外层 `Node.raw` 留空、原始字节放在 `Bit.bit_byte` 里、语义描述放在 `Bit.value` 里
//! （参见 spec-engine `src/parser.rs` 里构造位域的写法），这样和 DI 内容里本来就有的
//! 位字段渲染方式完全一致，不需要 UI 区分"这是帧层的位字段还是内容层的位字段"。
//!
//! ## 已知和标准文本对不上、需要你确认的地方
//!
//! - **主站地址 A3**：标准 5.1.4.4 明确写的是"A3的D0~D7组成0~255个主站地址"，即整个字节
//!   就是主站地址，没有再拆分。如果你的参考图例里 A3 被拆成"D7~D4帧序号"+"D3~D0主站地址"
//!   两个子位，这里没有照抄，因为我在标准文本里找不到依据——如果你能找到对应章节/勘误，
//!   我们再补上这个子位拆分。
//! - **D5(ACD)/D4(FCV) 在上行报文里的描述**：标准 5.1.3.4/5.1.3.5 里 ACD 的语义和功能码无关、
//!   D4 在上行方向明确写的是"保留"，这里就按这两条给出通用描述；如果你的参考图例里
//!   给出了"无效"/"FCB位无效"这类和功能码相关的特例描述，那是实现方自己的补充规则，
//!   标准文本里没有依据，需要你确认后我们再对应加上。
//! - **E0000000 的内容文字**：`spec_engine` 当前给这个 DI 的解析结果是 `Value::Int(0/1)`，
//!   还没有 `enum_map` 能直接产出"01-表示全部否定"这样的文字；这属于 spec-engine 那边
//!   `schema/csg13` 的补充，不在本 crate 里做。

use proto_common::FieldValue as Value;
use std::sync::OnceLock;

use crate::app::body::{ack, WriteParamError};
use crate::app::{ApplicationBody, ApplicationLayer, DataIdentifier, DataUnit, HistoryDataQuery, PointSelector, Tp};
use crate::engine::Message;
use crate::error::Result;
use crate::link::control::{ControlField, Direction};
use crate::link::{AddressField, Frame};

fn spec_engine() -> &'static spec_engine::Engine {
    static ENGINE: OnceLock<spec_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(spec_engine::Engine::new_default)
}

/// 查询 DI 的名称
///
/// 使用 spec_engine 的 lookup 方法查询数据标识的名称
fn lookup_di_name(protocol: &str, region: &str, di_hex: &str, dir: Option<&str>) -> Result<String> {
    // 将十六进制字符串转换为 u32
    let di_u32 = u32::from_str_radix(di_hex, 16)
        .map_err(|_| crate::error::ProtoError::UnknownDi(di_hex.to_string()))?;
    
    // 使用 spec_engine 查询 DI 定义
    match spec_engine().lookup(protocol, di_u32, region, dir) {
        Some(field) => Ok(field.name),
        _ => {
            // 查询失败，返回错误
            Err(crate::error::ProtoError::UnknownDi(di_hex.to_string()))
        }
    }
}

/// 解析一条完整报文并渲染成一棵 `Value` 树，返回渲染结果和消耗的字节数。
///
/// 根节点 `name` 固定为 `"Q/CSG1209022-2019 报文"`，`raw` 是整条报文的原始字节，`value` 是按标准顺序
/// 排列的字段列表（`Value::List`）：起始符/长度/起始符/控制域/地址域/AFN/SEQ/信息体/
/// [时间标签]/校验码/结束符。
pub fn decode_message_as_value(
    buf: &[u8],
    protocol: &str,
    region: &str,
) -> Result<(Value, usize)> {
    let (frame, consumed) = Frame::decode(buf)?;
    let application =
        ApplicationLayer::decode(&frame.payload, frame.control.direction, protocol, region)?;
    
    let msg = Message { frame, application };
    let tree = render_message_as_value(&msg, protocol, region)?;
    Ok((tree, consumed))
}

/// 将已解析的 Message 渲染成一棵 `Value` 树
///
/// 这是 `Message::to_value_tree()` 的内部实现函数。
///
/// # 参数
///
/// - `msg`: 已解析的报文
/// - `protocol`: 协议标识（如 "csg13"）
/// - `region`: 区域标识（如 "南网"）
pub fn render_message_as_value(msg: &Message, protocol: &str, region: &str) -> Result<Value> {
    // 从 Message 对象重新编码生成原始字节
    let raw_bytes = msg.frame.encode()?;
    
    let mut rows = Vec::new();

    rows.push(leaf("起始符", vec![raw_bytes[0]], "起始符".to_string()));

    // 长度L：BIN编码2字节，两次重复出现，这里合并展示成一行，raw 取两次出现的完整4字节
    let l = u16::from_le_bytes([raw_bytes[1], raw_bytes[2]]);
    rows.push(leaf(
        "长度",
        raw_bytes[1..5].to_vec(),
        format!("长度={}，总长度={}(总长度=长度+8)", l, l as u32 + 8),
    ));

    rows.push(leaf("起始符", vec![raw_bytes[5]], "起始符".to_string()));

    rows.push(control_field_node(&msg.frame.control));
    rows.push(address_field_node(&msg.frame.address)?);

    rows.push(leaf(
        "应用层功能码AFN",
        vec![msg.application.afn.to_u8()],
        msg.application.afn.description(),
    ));
    rows.push(seq_field_node(&msg.application.seq));
    rows.push(application_body_node(&msg.application.body, protocol, region, &msg.frame.control.direction));

    if let Some(tp) = &msg.application.tp {
        rows.push(tp_node(tp)?);
    }

    let cs_index = raw_bytes.len() - 2;
    rows.push(leaf(
        "校验码CS",
        vec![raw_bytes[cs_index]],
        "校验正确".to_string(),
    ));
    rows.push(leaf("结束符", vec![raw_bytes[raw_bytes.len() - 1]], "结束符".to_string()));

    Ok(Value::Node {
        name: "Q/CSG1209022-2019 报文".to_string(),
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

/// 位字段节点：跟随 spec-engine 自己的约定，`Node.raw` 留空，原始字节放在 `Bit.bit_byte`。
/// `bit_start`/`bit_end` 是闭区间、`bit_start >= bit_end`（比如 D7 单独一位是 (7,7)，
/// D3~D0 四位是 (3,0)）。
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

fn control_field_node(control: &ControlField) -> Value {
    let byte = control.encode();

    let dir_desc = match control.direction {
        Direction::Up => "终端发出的上行报文",
        Direction::Down => "主站发出的下行报文",
    }
    .to_string();

    let prm_desc = if control.prm {
        "来自启动站"
    } else {
        "来自从动站"
    }
    .to_string();

    let (d5_name, d5_desc) = match control.direction {
        Direction::Down => (
            "D5帧计数位FCB(下行)/要求访问位ACD(上行)",
            if control.fcb() {
                "新的发送/确认或请求/响应服务（帧计数取反）".to_string()
            } else {
                "沿用原帧计数".to_string()
            },
        ),
        Direction::Up => (
            "D5帧计数位FCB(下行)/要求访问位ACD(上行)",
            if control.acd() {
                "终端有告警数据等待访问".to_string()
            } else {
                "终端无告警数据等待访问".to_string()
            },
        ),
    };

    let (d4_name, d4_desc) = match control.direction {
        Direction::Down => (
            "D4帧计数有效位FCV(下行)/保留(上行)",
            if control.fcv {
                "FCB位有效".to_string()
            } else {
                "FCB位无效".to_string()
            },
        ),
        Direction::Up => ("D4帧计数有效位FCV(下行)/保留(上行)", "保留位".to_string()),
    };

    let fc_desc = link_function_code_desc(control.prm, control.function_code);

    let bits = vec![
        bit_node("D7传输方向位DIR", 7, 7, byte, dir_desc),
        bit_node("D6启动标志位PRM", 6, 6, byte, prm_desc),
        bit_node(d5_name, 5, 5, byte, d5_desc),
        bit_node(d4_name, 4, 4, byte, d4_desc),
        bit_node("D3~D0功能码", 3, 0, byte, fc_desc),
    ];

    Value::Node {
        name: "控制域C".to_string(),
        raw: vec![byte],
        value: Box::new(Value::List(bits)),
    }
}

/// 链路层功能码语义（标准 5.1.3.6 表 5-1/表 5-2）。
fn link_function_code_desc(prm: bool, code: u8) -> String {
    let prefix = if prm { "来自启动站" } else { "来自从动站" };
    let meaning = if prm {
        match code {
            1 => "复位命令（发送/确认）",
            4 => "用户数据（发送/无回答）",
            9 => "链路测试（请求/响应）",
            10 => "请求1级数据（请求/响应）",
            11 => "请求2级数据（请求/响应）",
            _ => "备用",
        }
    } else {
        match code {
            0 => "确认：认可",
            8 => "用户数据（响应帧）",
            9 => "否定：无所召唤数据",
            11 => "链路状态（响应帧）",
            _ => "备用",
        }
    };
    format!("{prefix}:{meaning}")
}

fn address_field_node(address: &AddressField) -> Result<Value> {
    let bytes = address.encode()?;
    let region = &address.region;

    let region_desc = format!(
        "省地市区县码={:02}{:02}{:02}省{:02},地市{:02},区县{:02}",
        region.province, region.city, region.county, region.province, region.city, region.county
    );
    let terminal_desc = format!("终端地址={:06X}", address.terminal_addr);
    let master_desc = format!("主站地址={}", address.master_addr);

    let children = vec![
        leaf("省地市区县码A1", bytes[0..3].to_vec(), region_desc),
        leaf("终端地址A2", bytes[3..6].to_vec(), terminal_desc),
        leaf("主站地址A3", vec![bytes[6]], master_desc),
    ];

    Ok(Value::Node {
        name: "地址域A".to_string(),
        raw: bytes.to_vec(),
        value: Box::new(Value::List(children)),
    })
}

fn seq_field_node(seq: &crate::app::SeqField) -> Value {
    let byte = seq.encode();

    let tpv_desc = if seq.tpv {
        "帧末尾带有时间标签Tp"
    } else {
        "帧末尾不带时间标签"
    }
    .to_string();

    let fir_desc = if seq.fir && seq.fin {
        "当前帧为单帧：第一帧".to_string()
    } else if seq.fir {
        "当前帧为多帧中的第一帧".to_string()
    } else {
        "当前帧不是首帧".to_string()
    };

    let fin_desc = if seq.fir && seq.fin {
        "当前帧为单帧：最后一帧".to_string()
    } else if seq.fin {
        "当前帧为多帧中的最后一帧".to_string()
    } else {
        "当前帧不是末帧，后续还有分帧".to_string()
    };

    let con_desc = if seq.con {
        "需要对该帧报文进行确认"
    } else {
        "不需要对该帧报文进行确认"
    }
    .to_string();

    let seq_desc = format!("帧内序号={}", seq.seq);

    let bits = vec![
        bit_node("D7帧时间标签有效位TpV", 7, 7, byte, tpv_desc),
        bit_node("D6首帧标志FIR", 6, 6, byte, fir_desc),
        bit_node("D5末帧标志FIN", 5, 5, byte, fin_desc),
        bit_node("D4请求确认标志CON", 4, 4, byte, con_desc),
        bit_node("D3~D0帧内序号", 3, 0, byte, seq_desc),
    ];

    Value::Node {
        name: "命令序号SEQ".to_string(),
        raw: vec![byte],
        value: Box::new(Value::List(bits)),
    }
}

fn application_body_node(body: &ApplicationBody, protocol: &str, region: &str, direction: &Direction) -> Value {
    // 根据方向确定 dir 参数
    let dir_str = match direction {
        Direction::Down => Some("0"),
        Direction::Up => Some("1"),
    };
    
    match body {
        // 上行响应：带DA+DI+内容的数据单元
        ApplicationBody::Ack(units)
        | ApplicationBody::LinkTest(units)
        | ApplicationBody::Auth(units)
        | ApplicationBody::ReadParamResponse(units)
        | ApplicationBody::ReadCurrentDataResponse(units)
        | ApplicationBody::ReadHistoryDataResponse(units)
        | ApplicationBody::ReadEventResponse(units)
        | ApplicationBody::ReadTaskDataResponse(units)
        | ApplicationBody::ReadAlarmDataResponse(units)
        | ApplicationBody::FileTransferRequest(units)
        | ApplicationBody::FileTransferResponse(units)
        | ApplicationBody::RelayRequest(units)
        | ApplicationBody::RelayResponse(units) => {
            let mut raw = Vec::new();
            let groups = units
                .iter()
                .enumerate()
                .map(|(i, unit)| {
                    raw.extend_from_slice(&unit.raw);
                    data_unit_group_node(i + 1, unit, protocol, region, dir_str)
                })
                .collect();
            Value::Node {
                name: "信息体".to_string(),
                raw,
                value: Box::new(Value::List(groups)),
            }
        }
        // 下行请求：只有DA+DI的标识列表
        ApplicationBody::ReadParamRequest(ids)
        | ApplicationBody::ReadCurrentDataRequest(ids)
        | ApplicationBody::ReadEventRequest(ids)
        | ApplicationBody::ReadTaskDataRequest(ids)
        | ApplicationBody::ReadAlarmDataRequest(ids)
        | ApplicationBody::FileTransferQueryRequest(ids) => {
            let mut raw = Vec::new();
            let groups = ids
                .iter()
                .enumerate()
                .map(|(i, id)| {
                    raw.extend_from_slice(&id.encode());
                    data_identifier_group_node(i + 1, id, protocol, region, dir_str)
                })
                .collect();
            Value::Node {
                name: "信息体".to_string(),
                raw,
                value: Box::new(Value::List(groups)),
            }
        }
        // 写参数下行请求：带内容
        ApplicationBody::WriteParamRequest(units) => {
            let mut raw = Vec::new();
            let groups = units
                .iter()
                .enumerate()
                .map(|(i, unit)| {
                    raw.extend_from_slice(&unit.raw);
                    data_unit_group_node(i + 1, unit, protocol, region, dir_str)
                })
                .collect();
            Value::Node {
                name: "信息体".to_string(),
                raw,
                value: Box::new(Value::List(groups)),
            }
        }
        // 写参数上行响应：DA+DI+ERR
        ApplicationBody::WriteParamResponse(errors) => {
            let mut raw = Vec::new();
            let groups = errors
                .iter()
                .enumerate()
                .map(|(i, err)| {
                    let err_raw = {
                        let mut v = Vec::new();
                        v.extend_from_slice(&err.da.encode());
                        v.extend_from_slice(&err.di.to_le_bytes());
                        v.push(err.err);
                        v
                    };
                    raw.extend_from_slice(&err_raw);
                    write_param_error_node(i + 1, err, &err_raw, protocol, region, dir_str)
                })
                .collect();
            Value::Node {
                name: "信息体".to_string(),
                raw,
                value: Box::new(Value::List(groups)),
            }
        }
        // 读历史数据下行请求：DA+DI+时间范围+密度
        ApplicationBody::ReadHistoryDataRequest(queries) => {
            let mut raw = Vec::new();
            let groups = queries
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    let query_raw = {
                        let mut v = Vec::new();
                        v.extend_from_slice(&q.da.encode());
                        v.extend_from_slice(&q.di.to_le_bytes());
                        v.extend_from_slice(&q.start_time.encode().unwrap_or([0; 6]));
                        v.extend_from_slice(&q.end_time.encode().unwrap_or([0; 6]));
                        v.push(q.density.encode());
                        v
                    };
                    raw.extend_from_slice(&query_raw);
                    history_data_query_node(i + 1, q, &query_raw, protocol, region, dir_str)
                })
                .collect();
            Value::Node {
                name: "信息体".to_string(),
                raw,
                value: Box::new(Value::List(groups)),
            }
        }
        ApplicationBody::Raw(bytes) => Value::Node {
            name: "信息体".to_string(),
            raw: bytes.clone(),
            value: Box::new(Value::Bytes(bytes.clone())),
        },
    }
}

fn point_selector_desc(selector: &PointSelector) -> String {
    match selector {
        PointSelector::Terminal => "Pn=测量点：0(终端)".to_string(),
        PointSelector::All => "Pn=全部测量点(除终端外)".to_string(),
        PointSelector::Points(points) => {
            let list = points
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!("Pn=测量点：{list}")
        }
    }
}

fn content_name(di: u32, value: &Value) -> String {
    match value {
        Value::Node { name, .. } => {
            // spec-engine 的 Node.name 形如 "E0000000_全部进行确定/否定"，
            // 已经自带 DI 前缀，这里去重，避免展示成 "[E0000000]-E0000000_xxx"。
            let prefix = format!("{di:08X}_");
            name.strip_prefix(&prefix).unwrap_or(name).to_string()
        }
        _ => "未知".to_string(),
    }
}

fn data_unit_group_node(index: usize, unit: &DataUnit, protocol: &str, region: &str, dir: Option<&str>) -> Value {
    let da_desc = point_selector_desc(&unit.da.point_selector());
    
    // 尝试查询 DI 名称
    let di_hex = format!("{:08X}", unit.di);
    let di_desc = if let Ok(name) = lookup_di_name(protocol, region, &di_hex, dir) {
        format!("数据标识编码：[{:08X}]-({})", unit.di, name)
    } else {
        format!(
            "数据标识编码：[{:08X}]-{}",
            unit.di,
            content_name(unit.di, &unit.value)
        )
    };

    let mut children = vec![
        leaf("信息点标识DA", unit.da.encode().to_vec(), da_desc),
        leaf("数据标识编码DI", unit.di.to_le_bytes().to_vec(), di_desc),
        Value::Node {
            name: "数据标识内容".to_string(),
            // 用 DataUnit.content_raw（解析时按 content_consumed 显式切片得到），
            // 不再事后匹配 unit.value 是不是 Value::Node 去猜 raw ——
            // 后者只在恰好是 Node 变体时凑巧对，遇到 Value::Int/Bit/List 等
            // 顶层就会静默拿到空数组。
            raw: unit.content_raw.clone(),
            value: Box::new(unit.value.clone()),
        },
    ];

    if let Some(time) = &unit.time {
        let raw = time.encode().map(|bytes| bytes.to_vec()).unwrap_or_default();
        children.push(leaf(
            "数据时间",
            raw,
            format!(
                "{:04}年{:02}月{:02}日{:02}时{:02}分",
                time.year, time.month, time.day, time.hour, time.minute
            ),
        ));
    }

    // 补上一段 ack 语义提示：仅当这条信息体正好是"全部确认/否定" DI 时才附加
    let _ = ack::DI_ACK_ALL; // 保留引用，提醒调用方 ack 模块提供 is_positive() 辅助函数

    Value::Node {
        name: format!("<第{index}组>"),
        // 直接用解析时切好的整组 raw，而不是留空 —— 它已经包含了 DA+DI+内容[+时间]
        // 全部原始字节，与 spec_engine 实际消耗的内容字节数保持一致。
        raw: unit.raw.clone(),
        value: Box::new(Value::List(children)),
    }
}

fn data_identifier_group_node(index: usize, id: &DataIdentifier, protocol: &str, region: &str, dir: Option<&str>) -> Value {
    let da_desc = point_selector_desc(&id.da.point_selector());
    
    // 尝试查询 DI 名称
    let di_hex = format!("{:08X}", id.di);
    let di_desc = if let Ok(name) = lookup_di_name(protocol, region, &di_hex, dir) {
        format!("数据标识编码：[{:08X}]-({})", id.di, name)
    } else {
        format!("数据标识编码：[{:08X}]", id.di)
    };

    let children = vec![
        leaf("信息点标识DA", id.da.encode().to_vec(), da_desc),
        leaf("数据标识编码DI", id.di.to_le_bytes().to_vec(), di_desc),
    ];

    Value::Node {
        name: format!("<第{index}组>"),
        // 这一变体没有内容/时间域，组的 raw 就是 DA+DI 本身，长度固定已知
        raw: id.encode().to_vec(),
        value: Box::new(Value::List(children)),
    }
}

fn write_param_error_node(index: usize, err: &WriteParamError, raw: &[u8], protocol: &str, region: &str, dir: Option<&str>) -> Value {
    let da_desc = point_selector_desc(&err.da.point_selector());
    
    // 尝试查询 DI 名称
    let di_hex = format!("{:08X}", err.di);
    let di_desc = if let Ok(name) = lookup_di_name(protocol, region, &di_hex, dir) {
        format!("数据标识编码：[{:08X}]-({})", err.di, name)
    } else {
        format!("数据标识编码：[{:08X}]", err.di)
    };
    let err_desc = format!("错误码：0x{:02X}", err.err);

    let children = vec![
        leaf("信息点标识DA", err.da.encode().to_vec(), da_desc),
        leaf("数据标识编码DI", err.di.to_le_bytes().to_vec(), di_desc),
        leaf("错误码ERR", vec![err.err], err_desc),
    ];

    Value::Node {
        name: format!("<第{index}组>"),
        raw: raw.to_vec(),
        value: Box::new(Value::List(children)),
    }
}

fn history_data_query_node(index: usize, q: &HistoryDataQuery, raw: &[u8], protocol: &str, region: &str, dir: Option<&str>) -> Value {
    let da_desc = point_selector_desc(&q.da.point_selector());
    
    // 尝试查询 DI 名称
    let di_hex = format!("{:08X}", q.di);
    let di_desc = if let Ok(name) = lookup_di_name(protocol, region, &di_hex, dir) {
        format!("数据标识编码：[{:08X}]-({})", q.di, name)
    } else {
        format!("数据标识编码：[{:08X}]", q.di)
    };
    let start_desc = format!(
        "起始时间：{:04}年{:02}月{:02}日{:02}时{:02}分",
        q.start_time.year, q.start_time.month, q.start_time.day, q.start_time.hour, q.start_time.minute
    );
    let end_desc = format!(
        "结束时间：{:04}年{:02}月{:02}日{:02}时{:02}分",
        q.end_time.year, q.end_time.month, q.end_time.day, q.end_time.hour, q.end_time.minute
    );
    let density_desc = format!("数据密度：{}", q.density.description());

    let children = vec![
        leaf("信息点标识DA", q.da.encode().to_vec(), da_desc),
        leaf("数据标识编码DI", q.di.to_le_bytes().to_vec(), di_desc),
        leaf(
            "数据起始时间",
            q.start_time.encode().unwrap_or([0; 6]).to_vec(),
            start_desc,
        ),
        leaf(
            "数据结束时间",
            q.end_time.encode().unwrap_or([0; 6]).to_vec(),
            end_desc,
        ),
        leaf("数据密度", vec![q.density.encode()], density_desc),
    ];

    Value::Node {
        name: format!("<第{index}组>"),
        raw: raw.to_vec(),
        value: Box::new(Value::List(children)),
    }
}

fn tp_node(tp: &Tp) -> Result<Value> {
    let bytes = tp.encode()?;
    let desc = format!(
        "启动帧发送时标：{}日{}时{}分{}秒。允许发送传输延迟时间：{}分",
        tp.day, tp.hour, tp.minute, tp.second, tp.allowed_delay_minutes
    );
    Ok(leaf("时间标签Tp", bytes.to_vec(), desc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Afn, DataAddress, SeqField};
    use crate::link::address::RegionCode;
    use crate::link::ControlField;

    #[test]
    fn renders_ack_all_frame_as_value_tree() {
        let mut payload = vec![
            Afn::Ack.to_u8(),
            SeqField::single_frame(false, 0).unwrap().encode(),
        ];
        payload.extend_from_slice(&DataAddress::terminal().encode());
        payload.extend_from_slice(&ack::DI_ACK_ALL.to_le_bytes());
        payload.push(0x01); // 内容：1=否定

        let frame = Frame {
            control: ControlField::new_up(false, false, false, 9).unwrap(),
            address: AddressField {
                region: RegionCode {
                    province: 44,
                    city: 0,
                    county: 0,
                },
                terminal_addr: 11,
                master_addr: 1,
            },
            payload,
        };
        let bytes = frame.encode().unwrap();

        let (tree, consumed) = decode_message_as_value(&bytes, "csg13", "南网").unwrap();
        assert_eq!(consumed, bytes.len());

        let Value::Node { name, value, .. } = &tree else {
            panic!("根节点必须是 Node");
        };
        assert_eq!(name, "报文");
        let Value::List(rows) = value.as_ref() else {
            panic!("根节点的值必须是 List");
        };
        // 起始符/长度/起始符/控制域/地址域/AFN/SEQ/信息体/CS/结束符 共10行（本例没有Tp）
        assert_eq!(rows.len(), 10);

        // 校验控制域展开成5个位字段
        let control_row = &rows[3];
        if let Value::Node {
            name,
            value: control_value,
            ..
        } = control_row
        {
            assert_eq!(name, "控制域C");
            if let Value::List(bits) = control_value.as_ref() {
                assert_eq!(bits.len(), 5);
            } else {
                panic!("控制域的值必须是 List");
            }
        } else {
            panic!("期望 Node");
        }

        // 信息体 -> <第1组> -> 数据标识内容 应该能追溯到 spec-engine 给出的 E0000000 节点
        let body_row = &rows[7];
        if let Value::Node {
            name,
            raw: body_raw,
            value: body_value,
        } = body_row
        {
            assert_eq!(name, "信息体");
            // 回归点：信息体的 raw 不应再是空数组，必须等于 DA+DI+内容 的原始字节
            // （本例没有数据时间域）：2字节DA + 4字节DI + 1字节内容 = 7字节。
            assert_eq!(body_raw, &[0u8, 0, 0, 0, 0, 224, 1]);
            if let Value::List(groups) = body_value.as_ref() {
                assert_eq!(groups.len(), 1);
                if let Value::Node {
                    raw: group_raw,
                    value: group_value,
                    ..
                } = &groups[0]
                {
                    // 组节点的 raw 同样不应为空，且要和信息体的 raw 一致（只有一组）
                    assert_eq!(group_raw, body_raw);
                    if let Value::List(fields) = group_value.as_ref() {
                        // 信息点标识DA / 数据标识编码DI / 数据标识内容
                        assert_eq!(fields.len(), 3);
                        if let Value::Node {
                            name: content_field_name,
                            raw: content_field_raw,
                            value: content_value,
                        } = &fields[2]
                        {
                            assert_eq!(content_field_name, "数据标识内容");
                            // 数据标识内容的 raw 必须是 spec_engine 实际消耗的那1字节内容，
                            // 不能因为顶层 Value 恰好/不恰好是 Node 变体而变化
                            assert_eq!(content_field_raw, &[1u8]);
                            // 内层应该就是 spec_engine::parse_di 给出的原始 Node
                            if let Value::Node {
                                name: inner_name, ..
                            } = content_value.as_ref()
                            {
                                assert!(inner_name.contains("全部进行确定"));
                            } else {
                                panic!("期望内层是 spec-engine 的 Node");
                            }
                        } else {
                            panic!("期望第3个字段是数据标识内容");
                        }
                    }
                } else {
                    panic!("期望 <第1组> 是 Node");
                }
            }
        } else {
            panic!("期望信息体是 Node");
        }
    }

    /// 回归测试：多组信息体（多个 DA+DI+内容）时，信息体的 raw 必须是每组各自消耗的
    /// 原始字节依次累加，而不是只取第一组或者留空。
    #[test]
    fn multi_group_body_raw_accumulates_each_units_consumed_bytes() {
        let mut payload = vec![
            Afn::Ack.to_u8(),
            SeqField::single_frame(false, 0).unwrap().encode(),
        ];

        // 第1组：终端点位 + E0000000（确认）
        payload.extend_from_slice(&DataAddress::terminal().encode());
        payload.extend_from_slice(&ack::DI_ACK_ALL.to_le_bytes());
        payload.push(0x00);

        // 第2组：测量点p1 + E0000000（否定），验证累加而不是只取第一组
        payload.extend_from_slice(&DataAddress::from_point_number(1).unwrap().encode());
        payload.extend_from_slice(&ack::DI_ACK_ALL.to_le_bytes());
        payload.push(0x01);

        let frame = Frame {
            control: ControlField::new_up(false, false, false, 9).unwrap(),
            address: AddressField {
                region: RegionCode {
                    province: 44,
                    city: 0,
                    county: 0,
                },
                terminal_addr: 11,
                master_addr: 1,
            },
            payload,
        };
        let bytes = frame.encode().unwrap();

        let (tree, _consumed) = decode_message_as_value(&bytes, "csg13", "南网").unwrap();
        let Value::Node { value, .. } = &tree else {
            panic!("根节点必须是 Node");
        };
        let Value::List(rows) = value.as_ref() else {
            panic!("根节点的值必须是 List");
        };
        let Value::Node {
            raw: body_raw,
            value: body_value,
            ..
        } = &rows[7]
        else {
            panic!("期望信息体是 Node");
        };

        // 两组各 7 字节（2DA+4DI+1内容），累计 14 字节
        assert_eq!(body_raw.len(), 14);
        assert_eq!(
            body_raw,
            &[0u8, 0, 0, 0, 0, 224, 0, 1, 1, 0, 0, 0, 224, 1]
        );

        let Value::List(groups) = body_value.as_ref() else {
            panic!("信息体的值必须是 List");
        };
        assert_eq!(groups.len(), 2);
        // 每组各自的 raw 拼起来应该正好等于信息体整体的 raw
        let mut rebuilt = Vec::new();
        for g in groups {
            let Value::Node { raw, .. } = g else {
                panic!("每组必须是 Node");
            };
            rebuilt.extend_from_slice(raw);
        }
        assert_eq!(&rebuilt, body_raw);
    }
}

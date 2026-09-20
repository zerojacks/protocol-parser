//! 应用层"信息体"：DA + DI + 数据标识内容 + [数据时间]（标准 6.1.1 图6-1、6.1.4、6.1.5）。
//!
//! 这是帧层和 `spec-engine`（内容层）真正对接的地方：DA/DI/时间域由本 crate 自己解析，
//! DI 内容本身完全委托给 `spec_engine::parse_di`，本 crate 不重复实现任何字段级解码逻辑。
//!
//! 注意：目前只支持**解码**方向的内容解析（对应 `spec_engine::parse_di`）。
//! `spec-engine` 目前没有对外暴露内容编码接口，所以下行"写参数"等需要构造 DI 内容字节的场景，
//! 调用方需要自己准备好原始字节传入 `DataUnit::new_raw`；等 spec-engine 补上编码能力后再打通。

pub mod ack;

use crate::app::da::{DataAddress, DA_LEN};
use crate::app::datetime::{DataTime, Density, DATA_TIME_LEN};
use crate::error::{ProtoError, Result};
use proto_common::{field_value, FieldValue};
use std::sync::OnceLock;

/// DI 占4字节，传输顺序 DI0,DI1,DI2,DI3（小端），与 spec-engine 的 DI 数值约定一致。
pub const DI_LEN: usize = 4;

fn spec_engine() -> &'static spec_engine::Engine {
    static ENGINE: OnceLock<spec_engine::Engine> = OnceLock::new();
    ENGINE.get_or_init(spec_engine::Engine::new_default)
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataUnit {
    pub da: DataAddress,
    pub di: u32,
    pub value: FieldValue,
    /// 仅当所属报文的 SEQ.TpV 位为1，或该 AFN 语义上要求携带数据时间域时才存在
    pub time: Option<DataTime>,
    /// 本组（DA+DI+内容[+数据时间]）在原始报文里实际占用的原始字节。
    ///
    /// 显式在解析时按位置切片得到（`group_start..offset`），不依赖事后从
    /// 解析结果的形状反推——`FieldValue`/`spec_engine::Value` 并不是每种 DI
    /// 内容都会是 `Node`，反推方式在别的形状下会静默拿到空数组。
    pub raw: Vec<u8>,
    /// 单独的"数据标识内容"部分（即 `spec_engine::parse_di` 消费掉的那一段）原始字节，
    /// 同样按 `content_consumed` 显式切片得到，理由同上。
    pub content_raw: Vec<u8>,
}

/// 解析一段"DA+DI+内容[+时间]"反复出现的应用层数据区（标准6.1.5"数据组织的顺序规则"）。
///
/// - `protocol`/`region`/`dir` 透传给 `spec_engine::parse_di`，分别对应协议名（如 `"csg13"`）、
///   省份/局方（如 `"南网"`，查不到会自动回退到 `spec_engine::DEFAULT_REGION`）、报文方向。
/// - `with_time` 对应 SEQ 域的 TpV 位：为 true 时每个信息体后面还跟着一个 6 字节的数据时间域。
/// 
/// 注意：循环解析会在遇到以下情况时停止：
/// - 缓冲区结束
/// - 剩余恰好16字节且无法解析为有效的DA（可能是PW密码域）
pub fn parse_data_units(
    buf: &[u8],
    protocol: &str,
    region: &str,
    dir: Option<&str>,
    with_time: bool,
) -> Result<(Vec<DataUnit>, usize)> {
    let mut offset = 0usize;
    let mut units = Vec::new();
    
    const PW_LEN: usize = 16; // 密码域长度

    while offset < buf.len() {
        let remaining = buf.len() - offset;
        
        // 如果剩余字节恰好是16字节，可能是PW（密码域）
        // PW出现在除AFN=00H外的所有报文中
        // 判断方法：
        // 1. 尝试解析DA，如果失败则认为是PW
        // 2. 如果DA解析成功，尝试解析DI和内容，如果这16字节无法构成有效的数据单元，则认为是PW
        if remaining == PW_LEN {
            let mut is_pw = false;
            
            // 尝试完整解析一个数据单元来判断是否是PW
            if let Ok((_da, da_consumed)) = DataAddress::decode(&buf[offset..]) {
                let test_offset = offset + da_consumed;
                
                // 检查是否有足够的字节解析DI
                if remaining >= da_consumed + DI_LEN {
                    let di = u32::from_le_bytes([
                        buf[test_offset],
                        buf[test_offset + 1],
                        buf[test_offset + 2],
                        buf[test_offset + 3],
                    ]);
                    let content_offset = test_offset + DI_LEN;
                    
                    // 尝试解析DI内容
                    if let Ok((_, content_consumed)) = spec_engine().parse_di(
                        protocol,
                        di,
                        region,
                        dir,
                        &buf[content_offset..],
                    ) {
                        let total_consumed = da_consumed + DI_LEN + content_consumed;
                        
                        // 如果with_time=true，还需要加上时间字段
                        let expected_consumed = if with_time {
                            total_consumed + DATA_TIME_LEN
                        } else {
                            total_consumed
                        };
                        
                        // 如果16字节无法完整容纳一个数据单元，或者解析后还有剩余（不等于16），
                        // 则认为这不是有效的数据单元，而是PW
                        if expected_consumed > PW_LEN || (expected_consumed < PW_LEN && total_consumed != PW_LEN) {
                            is_pw = true;
                        }
                    } else {
                        // DI内容解析失败，可能是PW
                        is_pw = true;
                    }
                } else {
                    // 没有足够字节解析DI，可能是PW
                    is_pw = true;
                }
            } else {
                // DA解析失败，认为是PW
                is_pw = true;
            }
            
            if is_pw {
                // 确认是PW，停止解析数据单元
                break;
            }
        }
        
        // 至少需要 DA + DI 的长度
        if remaining < DA_LEN + DI_LEN {
            break;
        }

        // 本组（DA+DI+内容[+时间]）的起点，用来在解析完成后一次性切出整组的 raw，
        // 而不是事后从已经解析出来的 spec_engine::Value 里反推。
        let group_start = offset;

        let (da, da_consumed) = match DataAddress::decode(&buf[offset..]) {
            Ok(result) => result,
            Err(_) => {
                // DA解析失败，可能遇到了PW或其他非数据区域，停止解析
                break;
            }
        };
        offset += da_consumed;

        if buf.len() < offset + DI_LEN {
            // 回退offset，保留这部分未解析的字节（可能是PW）
            offset = group_start;
            break;
        }
        
        let di = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += DI_LEN;

        let content_start = offset;
        let (raw_value, content_consumed) = match spec_engine().parse_di(protocol, di, region, dir, &buf[offset..]) {
            Ok(result) => result,
            Err(e) => {
                // DI内容解析失败，可能是无效的DI或遇到了PW
                // 如果之前解析成功了一些单元，这里停止；否则返回错误
                if units.is_empty() {
                    return Err(ProtoError::Dict(e));
                }
                offset = group_start;
                break;
            }
        };
        offset += content_consumed;
        // 直接按 spec_engine 报告的 content_consumed 切片，这段字节数才是"真相"来源；
        // 不遍历/反推解析结果的内部结构去猜有多少字节被消耗掉了。
        let content_raw = buf[content_start..offset].to_vec();
        // 在这里、且只在这里，把 spec_engine::Value 转换成本 crate 自己的 FieldValue——
        // 越过这一行之后，本 crate 内部（包括 report 渲染）就不再直接依赖 spec_engine 的类型。
        let value = field_value::from_spec_engine(&raw_value);

        let time = if with_time {
            if buf.len() < offset + DATA_TIME_LEN {
                // 时间字段不完整，停止解析
                offset = group_start;
                break;
            }
            let (t, t_consumed) = DataTime::decode(&buf[offset..])?;
            offset += t_consumed;
            Some(t)
        } else {
            None
        };

        let raw = buf[group_start..offset].to_vec();

        units.push(DataUnit {
            da,
            di,
            value,
            time,
            raw,
            content_raw,
        });
    }

    Ok((units, offset))
}

/// 只有 DA+DI、没有内容的场景（下行请求报文，如"读当前数据"请求里只召唤 DA+DI，不带内容）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataIdentifier {
    pub da: DataAddress,
    pub di: u32,
}

/// 写参数响应中的错误信息（DA+DI+ERR码），标准6.2.3.2.4和附录F
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteParamError {
    pub da: DataAddress,
    pub di: u32,
    /// 错误码（1字节），定义见附录F
    pub err: u8,
}

pub const DATA_IDENTIFIER_LEN: usize = DA_LEN + DI_LEN;

impl DataIdentifier {
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        let (da, da_consumed) = DataAddress::decode(buf)?;
        if buf.len() < da_consumed + DI_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: da_consumed + DI_LEN,
                actual: buf.len(),
            });
        }
        let base = da_consumed;
        let di = u32::from_le_bytes([buf[base], buf[base + 1], buf[base + 2], buf[base + 3]]);
        Ok((Self { da, di }, DATA_IDENTIFIER_LEN))
    }

    pub fn encode(&self) -> [u8; DATA_IDENTIFIER_LEN] {
        let mut out = [0u8; DATA_IDENTIFIER_LEN];
        out[..DA_LEN].copy_from_slice(&self.da.encode());
        out[DA_LEN..].copy_from_slice(&self.di.to_le_bytes());
        out
    }
}

/// 解析读任务数据响应中的自描述格式任务数据（表A.10.4）或任务定义格式（表A.10.5）。
///
/// AFN=12H 上行报文有两种数据格式：
/// - 数据结构方式=0：自描述方式（表A.10.4），每个数据单元包含DA+DI+内容+时间
/// - 数据结构方式=1：按任务定义（表A.10.5），只有数据内容，需要预先知道任务参数配置
///
/// 参数说明：
/// - `buf`: 包含数据结构方式字节及后续所有任务数据内容的字节切片
/// - `protocol`/`region`/`dir`: 透传给 spec_engine 解析 DI 内容
pub fn parse_task_data(
    buf: &[u8],
    protocol: &str,
    region: &str,
    dir: Option<&str>,
) -> Result<(Vec<DataUnit>, usize)> {
    if buf.is_empty() {
        return Err(ProtoError::UnexpectedEof {
            needed: 1,
            actual: 0,
        });
    }

    let data_structure = buf[0];
    let mut offset = 1;

    match data_structure {
        0 => {
            // 自描述方式（表A.10.4）
            // 格式：数据组数(n, m) + [DA(2) + DI(4) + 内容(变长) + 时间(5)] × (n×m)
            
            if buf.len() < 3 {
                return Err(ProtoError::UnexpectedEof {
                    needed: 3,
                    actual: buf.len(),
                });
            }
            
            let n = buf[1] as usize; // 信息点标识组数
            let m = buf[2] as usize; // 数据标识编码组数
            offset = 3;

            let total_units = n * m;
            let mut units = Vec::new();
            const PW_LEN: usize = 16; // 密码域长度

            for i in 0..total_units {
                if offset >= buf.len() {
                    break; // 数据可能不完整，返回已解析的部分
                }
                
                let remaining = buf.len() - offset;
                
                // PW检测：如果剩余字节恰好是16字节且还有未解析的单元，可能是PW
                if remaining == PW_LEN && i < total_units - 1 {
                    // 尝试解析DA来判断是否是有效的数据单元
                    let mut is_pw = false;
                    
                    if let Ok((_da, da_consumed)) = DataAddress::decode(&buf[offset..]) {
                        let test_offset = offset + da_consumed;
                        
                        if remaining >= da_consumed + DI_LEN {
                            let di = u32::from_le_bytes([
                                buf[test_offset],
                                buf[test_offset + 1],
                                buf[test_offset + 2],
                                buf[test_offset + 3],
                            ]);
                            let content_offset = test_offset + DI_LEN;
                            
                            if let Ok((_, content_consumed)) = spec_engine().parse_di(
                                protocol,
                                di,
                                region,
                                dir,
                                &buf[content_offset..],
                            ) {
                                // 自描述格式：DA(2) + DI(4) + 内容 + 时间(5)
                                let expected_consumed = da_consumed + DI_LEN + content_consumed + 5;
                                
                                if expected_consumed > PW_LEN || (expected_consumed < PW_LEN && da_consumed + DI_LEN + content_consumed != PW_LEN) {
                                    is_pw = true;
                                }
                            } else {
                                is_pw = true;
                            }
                        } else {
                            is_pw = true;
                        }
                    } else {
                        is_pw = true;
                    }
                    
                    if is_pw {
                        // 确认是PW，停止解析
                        break;
                    }
                }

                let group_start = offset;

                // 解析 DA (2字节)
                let (da, da_consumed) = DataAddress::decode(&buf[offset..])?;
                offset += da_consumed;

                // 解析 DI (4字节)
                if buf.len() < offset + DI_LEN {
                    break;
                }
                let di = u32::from_le_bytes([
                    buf[offset],
                    buf[offset + 1],
                    buf[offset + 2],
                    buf[offset + 3],
                ]);
                offset += DI_LEN;

                // 解析数据标识内容（变长，由 spec_engine 确定）
                let content_start = offset;
                let (raw_value, content_consumed) =
                    spec_engine().parse_di(protocol, di, region, dir, &buf[offset..])?;
                offset += content_consumed;
                let content_raw = buf[content_start..offset].to_vec();
                let value = field_value::from_spec_engine(&raw_value);

                // 解析数据时间（5字节 BCD）
                let time = if buf.len() >= offset + 5 {
                    // 自描述格式中数据时间是5字节 BCD: YYMMDDhhmm
                    let time_bytes = &buf[offset..offset + 5];
                    
                    // 检查是否为填充字节（全FF表示无效数据）
                    let is_invalid = time_bytes.iter().all(|&b| b == 0xFF);
                    
                    if !is_invalid {
                        // 解析BCD时间
                        let year = bcd_to_decimal(time_bytes[0]);
                        let month = bcd_to_decimal(time_bytes[1]);
                        let day = bcd_to_decimal(time_bytes[2]);
                        let hour = bcd_to_decimal(time_bytes[3]);
                        let minute = bcd_to_decimal(time_bytes[4]);
                        
                        offset += 5;
                        Some(DataTime {
                            year: 2000 + year as u16, // YY -> YYYY
                            month,
                            day,
                            hour,
                            minute,
                        })
                    } else {
                        offset += 5;
                        None
                    }
                } else {
                    None
                };

                let raw = buf[group_start..offset].to_vec();

                units.push(DataUnit {
                    da,
                    di,
                    value,
                    time,
                    raw,
                    content_raw,
                });
            }

            Ok((units, offset))
        }
        1 => {
            // 按任务定义方式（表A.10.5）
            // 这种格式需要预先知道任务参数配置才能正确解析
            // 格式：[DA(2) + 内容1 + ... + 内容m] × n
            // 由于不知道每个内容项的长度，这里暂时使用通用解析
            // TODO: 需要读取任务参数配置来确定数据项列表
            
            // 暂时回退到通用解析，不带时间
            let (units, consumed) = parse_data_units(&buf[offset..], protocol, region, dir, false)?;
            Ok((units, offset + consumed))
        }
        _ => {
            // 未知的数据结构方式，返回错误
            Err(ProtoError::OutOfRange(data_structure as u32))
        }
    }
}

/// 解析读历史数据响应中的多时间点数据（AFN=0DH上行）。
///
/// AFN=0DH 上行报文格式：循环 [DA + DI + (内容 + 时间(6字节)) × N]
/// 
/// 协议描述（6.2.7.2.1）：
/// "每个DA、DI的值唯一表示一个信息点和一个数据标识，后面紧跟数据标识的内容和数据时间。
/// 按时间的先后顺序组织回复报文。"
/// 
/// 注意：
/// - 同一DA+DI可以重复出现多次，每次对应不同时间点的数据
/// - 同一 DA+DI 的后续时间点不重复携带 DA+DI，数量需要通过内容和时间判断
pub fn parse_history_data_response(
    buf: &[u8],
    protocol: &str,
    region: &str,
    dir: Option<&str>,
) -> Result<(Vec<DataUnit>, usize)> {
    let mut offset = 0usize;
    let mut units = Vec::new();
    const PW_LEN: usize = 16; // 密码域长度

    // 循环解析直到缓冲区结束
    while offset < buf.len() {
        let remaining = buf.len() - offset;
        
        // PW检测：如果剩余字节恰好是16字节，可能是PW
        if remaining == PW_LEN {
            let mut is_pw = false;
            
            if let Ok((_da, da_consumed)) = DataAddress::decode(&buf[offset..]) {
                let test_offset = offset + da_consumed;
                
                if remaining >= da_consumed + DI_LEN {
                    let di = u32::from_le_bytes([
                        buf[test_offset],
                        buf[test_offset + 1],
                        buf[test_offset + 2],
                        buf[test_offset + 3],
                    ]);
                    let content_offset = test_offset + DI_LEN;
                    
                    if let Ok((_, content_consumed)) = spec_engine().parse_di(
                        protocol,
                        di,
                        region,
                        dir,
                        &buf[content_offset..],
                    ) {
                        // 16字节剩余区只有在恰好构成一个完整数据点时才不是PW。
                        let expected_consumed =
                            da_consumed + DI_LEN + content_consumed + DATA_TIME_LEN;

                        if expected_consumed != PW_LEN {
                            is_pw = true;
                        }
                    } else {
                        is_pw = true;
                    }
                } else {
                    is_pw = true;
                }
            } else {
                is_pw = true;
            }
            
            if is_pw {
                // 确认是PW，停止解析
                break;
            }
        }
        
        // 至少需要 DA(2) + DI(4) + 时间(6) = 12字节
        if remaining < DA_LEN + DI_LEN + DATA_TIME_LEN {
            // 剩余字节不足以构成一个完整的数据单元，停止解析
            break;
        }

        let group_start = offset;

        // 解析 DA
        let (da, da_consumed) = DataAddress::decode(&buf[offset..])?;
        offset += da_consumed;

        // 解析 DI
        if buf.len() < offset + DI_LEN {
            break;
        }
        let di = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += DI_LEN;

        let mut values = Vec::new();
        let mut content_raw = Vec::new();
        let mut previous_time = None;
        loop {
            let unit_start = offset;
            let (raw_value, content_consumed) = match spec_engine().parse_di(
                protocol,
                di,
                region,
                dir,
                &buf[offset..],
            ) {
                Ok(result) => result,
                Err(error) if !values.is_empty() => {
                    offset = unit_start;
                    let _ = error;
                    break;
                }
                Err(error) => return Err(ProtoError::Dict(error)),
            };
            let content_start = offset;
            offset += content_consumed;
            content_raw.extend_from_slice(&buf[content_start..offset]);

            if buf.len() < offset + DATA_TIME_LEN {
                offset = group_start;
                break;
            }
            let (time, time_consumed) = match DataTime::decode(&buf[offset..]) {
                Ok(result) => result,
                Err(error) if !values.is_empty() => {
                    offset = unit_start;
                    let _ = error;
                    break;
                }
                Err(error) => return Err(error),
            };
            offset += time_consumed;

            if !is_valid_history_time(&time)
                || previous_time
                    .as_ref()
                    .is_some_and(|previous| !is_history_time_after(&time, previous))
            {
                offset = unit_start;
                break;
            }

            let time_value = FieldValue::Node {
                name: "数据时间".to_string(),
                raw: time.encode().map(|bytes| bytes.to_vec()).unwrap_or_default(),
                value: Box::new(FieldValue::Str(format!(
                    "{:04}年{:02}月{:02}日{:02}时{:02}分",
                    time.year, time.month, time.day, time.hour, time.minute
                ))),
            };
            values.push(FieldValue::Map(vec![
                (
                    "数据内容".to_string(),
                    field_value::from_spec_engine(&raw_value),
                ),
                ("数据时间".to_string(), time_value),
            ]));
            previous_time = Some(time);
        }

        if !values.is_empty() {
            let raw = buf[group_start..offset].to_vec();
            units.push(DataUnit {
                da,
                di,
                value: FieldValue::List(values),
                time: None,
                raw,
                content_raw,
            });
        }
    }

    Ok((units, offset))
}

fn is_valid_history_time(time: &DataTime) -> bool {
    if !(1..=12).contains(&time.month)
        || !(0..=23).contains(&time.hour)
        || !(0..=59).contains(&time.minute)
    {
        return false;
    }

    let days_in_month: u8 = match time.month {
        2 => {
            if time.year % 4 == 0 {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&time.day)
}

fn is_history_time_after(current: &DataTime, previous: &DataTime) -> bool {
    (
        current.year,
        current.month,
        current.day,
        current.hour,
        current.minute,
    ) > (
        previous.year,
        previous.month,
        previous.day,
        previous.hour,
        previous.minute,
    )
}

/// BCD码转十进制
fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

/// 解析连续排列、只有 DA+DI 没有内容的召唤列表（下行请求常见结构）。
pub fn parse_data_identifiers(buf: &[u8]) -> Result<Vec<DataIdentifier>> {
    let mut offset = 0;
    let mut out = Vec::new();
    while offset < buf.len() {
        let (id, consumed) = DataIdentifier::decode(&buf[offset..])?;
        out.push(id);
        offset += consumed;
    }
    Ok(out)
}

/// 解析写参数响应中的错误列表（DA+DI+ERR，标准6.2.3.2）。
pub fn parse_write_param_errors(buf: &[u8]) -> Result<Vec<WriteParamError>> {
    let mut offset = 0;
    let mut out = Vec::new();
    while offset < buf.len() {
        let (da, da_consumed) = DataAddress::decode(&buf[offset..])?;
        offset += da_consumed;

        if buf.len() < offset + DI_LEN + 1 {
            return Err(ProtoError::UnexpectedEof {
                needed: offset + DI_LEN + 1,
                actual: buf.len(),
            });
        }
        let di = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += DI_LEN;

        let err = buf[offset];
        offset += 1;

        out.push(WriteParamError { da, di, err });
    }
    Ok(out)
}

/// 解析读历史数据请求中的查询列表（DA+DI+起始时间+结束时间+密度，标准6.2.7.1）。
pub fn parse_history_data_queries(buf: &[u8]) -> Result<Vec<crate::app::HistoryDataQuery>> {
    use crate::app::datetime::DATA_TIME_LEN;
    use crate::app::HistoryDataQuery;

    let mut offset = 0;
    let mut out = Vec::new();
    while offset < buf.len() {
        let (da, da_consumed) = DataAddress::decode(&buf[offset..])?;
        offset += da_consumed;

        if buf.len() < offset + DI_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: offset + DI_LEN,
                actual: buf.len(),
            });
        }
        let di = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += DI_LEN;

        // 解析起始时间（7字节）
        if buf.len() < offset + DATA_TIME_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: offset + DATA_TIME_LEN,
                actual: buf.len(),
            });
        }
        let (start_time, _) = DataTime::decode(&buf[offset..])?;
        offset += DATA_TIME_LEN;

        // 解析结束时间（7字节）
        if buf.len() < offset + DATA_TIME_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: offset + DATA_TIME_LEN,
                actual: buf.len(),
            });
        }
        let (end_time, _) = DataTime::decode(&buf[offset..])?;
        offset += DATA_TIME_LEN;

        // 解析密度（1字节）
        if offset >= buf.len() {
            return Err(ProtoError::UnexpectedEof {
                needed: offset + 1,
                actual: buf.len(),
            });
        }
        let density = Density::decode(buf[offset]);
        offset += 1;

        out.push(HistoryDataQuery {
            da,
            di,
            start_time,
            end_time,
            density,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_identifier_roundtrip() {
        let id = DataIdentifier {
            da: DataAddress::from_point_number(1).unwrap(),
            di: 0x00010000,
        };
        let bytes = id.encode();
        let (decoded, consumed) = DataIdentifier::decode(&bytes).unwrap();
        assert_eq!(consumed, DATA_IDENTIFIER_LEN);
        assert_eq!(decoded, id);
    }

    #[test]
    fn parse_multiple_identifiers() {
        let id1 = DataIdentifier {
            da: DataAddress::from_point_number(1).unwrap(),
            di: 0x00010000,
        };
        let id2 = DataIdentifier {
            da: DataAddress::from_point_number(2).unwrap(),
            di: 0x00020000,
        };
        let mut buf = Vec::new();
        buf.extend_from_slice(&id1.encode());
        buf.extend_from_slice(&id2.encode());
        let parsed = parse_data_identifiers(&buf).unwrap();
        assert_eq!(parsed, vec![id1, id2]);
    }

    #[test]
    fn parse_history_data_with_multiple_time_points() {
        let da1 = DataAddress::from_point_number(1).unwrap();
        let da2 = DataAddress::from_point_number(2).unwrap();
        let di: u32 = 0x0001_0000;
        let mut buf = Vec::new();

        buf.extend_from_slice(&da1.encode());
        buf.extend_from_slice(&di.to_le_bytes());
        buf.extend_from_slice(&[0x23, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&[0x20, 0x26, 0x09, 0x20, 0x10, 0x00]);
        buf.extend_from_slice(&[0x24, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&[0x20, 0x26, 0x09, 0x20, 0x10, 0x01]);

        buf.extend_from_slice(&da2.encode());
        buf.extend_from_slice(&di.to_le_bytes());
        buf.extend_from_slice(&[0x25, 0x01, 0x00, 0x00]);
        buf.extend_from_slice(&[0x20, 0x26, 0x09, 0x20, 0x10, 0x02]);

        let (units, consumed) =
            parse_history_data_response(&buf, "csg13", "南网", Some("1")).unwrap();

        assert_eq!(consumed, buf.len());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].da, da1);
        assert_eq!(units[1].da, da2);
        match &units[0].value {
            FieldValue::List(values) => {
                assert_eq!(values.len(), 2);
                assert!(matches!(
                    &values[0],
                    FieldValue::Map(entries) if entries.len() == 2
                ));
                assert!(matches!(
                    &values[1],
                    FieldValue::Map(entries) if entries.len() == 2
                ));
            }
            other => panic!("期望历史内容列表，实际得到 {other:?}"),
        }
        assert!(matches!(&units[1].value, FieldValue::List(values) if values.len() == 1));
    }

    // 端到端：真实 DI 内容解析走 spec-engine，覆盖在 tests/ 下的集成测试里，
    // 这里只覆盖帧层自己负责的 DA/DI 定位逻辑，避免和 spec-engine 的内部实现细节耦合。
}

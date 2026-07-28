//! 应用层：AFN + SEQ + 信息体 + [PW] + [Tp]（标准第6章）。

pub mod afn;
pub mod body;
pub mod da;
pub mod datetime;
pub mod seq;

pub use afn::Afn;
pub use body::{ack, DataIdentifier, DataUnit, WriteParamError};
pub use da::{DataAddress, PointSelector, DA_LEN};
pub use datetime::{DataTime, Density, Tp};
pub use seq::SeqField;

use crate::error::{ProtoError, Result};
use crate::link::Direction;

/// 读历史数据请求中的查询条件（DA+DI+起始时间+结束时间+密度，标准6.2.7.1）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryDataQuery {
    pub da: DataAddress,
    pub di: u32,
    /// 数据起始时间（标准6.2.7.1.4，7字节：分秒时日月年星期）
    pub start_time: DataTime,
    /// 数据结束时间（7字节）
    pub end_time: DataTime,
    /// 数据密度（标准6.2.7.1.6，1字节）
    pub density: Density,
}

/// 已按 AFN + 方向具体解析出的报文体。
///
/// 目前对 `AFN=0x00`（确认/否定）和 `AFN=0x0C`（读当前数据）做了完全类型化的解析，
/// 覆盖了从帧层到 spec-engine 内容层的完整打通路径；其余 AFN 先落到 `Raw`，
/// 后续按 03a/03b 参考文件逐个补齐，不在一次性改动里囫囵吞枣地猜。
///
/// `Ack` 和 `ReadCurrentDataResponse` 结构上其实一样（都是 DA+DI+内容 的循环体，
/// 内容都交给 spec_engine::parse_di 解析——`E0000000` 这个"全部确认/否定"DI 本身就在
/// csg13 字典里），只是分开命名方便调用方按语义匹配。
#[derive(Debug, Clone, PartialEq)]
pub enum ApplicationBody {
    /// 确认/否定 AFN=00H
    Ack(Vec<DataUnit>),
    
    /// 链路接口检测 AFN=02H（上行：DA+DI+内容，下行用AFN=00H确认）
    LinkTest(Vec<DataUnit>),
    
    /// 安全认证 AFN=06H（标准6.2.4，DA=0，单组DI+内容）
    Auth(Vec<DataUnit>),
    
    /// 读参数 AFN=0AH
    ReadParamRequest(Vec<DataIdentifier>),
    ReadParamResponse(Vec<DataUnit>),
    
    /// 写参数 AFN=04H
    WriteParamRequest(Vec<DataUnit>),
    WriteParamResponse(Vec<WriteParamError>),
    
    /// 读当前数据 AFN=0CH
    ReadCurrentDataRequest(Vec<DataIdentifier>),
    ReadCurrentDataResponse(Vec<DataUnit>),
    
    /// 读历史数据 AFN=0DH（下行请求带时间范围和密度，标准6.2.7）
    ReadHistoryDataRequest(Vec<HistoryDataQuery>),
    ReadHistoryDataResponse(Vec<DataUnit>),
    
    /// 读事件记录 AFN=0EH
    ReadEventRequest(Vec<DataIdentifier>),
    ReadEventResponse(Vec<DataUnit>),
    
    /// 读任务数据 AFN=12H
    ReadTaskDataRequest(Vec<DataIdentifier>),
    ReadTaskDataResponse(Vec<DataUnit>),
    
    /// 读告警数据 AFN=13H
    ReadAlarmDataRequest(Vec<DataIdentifier>),
    ReadAlarmDataResponse(Vec<DataUnit>),
    
    /// 文件传输 AFN=0FH
    FileTransferRequest(Vec<DataUnit>),          // 下行：文件传输启动/传输文件内容
    FileTransferQueryRequest(Vec<DataIdentifier>), // 下行：查询文件信息（无内容）
    FileTransferResponse(Vec<DataUnit>),         // 上行：响应（查询文件信息、错误码等）
    
    /// 中继转发 AFN=10H
    RelayRequest(Vec<DataUnit>),     // 下行：主站->终端->设备
    RelayResponse(Vec<DataUnit>),    // 上行：设备->终端->主站
    
    /// 尚未类型化的 AFN，保留原始字节，调用方可以自行按标准对应章节解析
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationLayer {
    pub afn: Afn,
    pub seq: SeqField,
    pub body: ApplicationBody,
    /// 时间标签，仅当 `seq.tpv == true` 时存在（标准 6.1.8）
    pub tp: Option<Tp>,
}

impl ApplicationLayer {
    /// 解析应用层原始字节（即 `Frame.payload`）。
    ///
    /// - `direction` 来自链路层控制域 `ControlField.direction`，同一个 AFN 下行/上行报文体结构不同；
    /// - `protocol`/`region` 透传给 spec-engine 做 DI 内容解析（如 `"csg13"` / `"南网"`）。
    pub fn decode(
        payload: &[u8],
        direction: Direction,
        protocol: &str,
        region: &str,
    ) -> Result<Self> {
        if payload.len() < 2 {
            return Err(ProtoError::UnexpectedEof {
                needed: 2,
                actual: payload.len(),
            });
        }
        let afn = Afn::from_u8(payload[0]);
        let seq = SeqField::decode(payload[1]);
        let mut rest = &payload[2..];

        let tp = if seq.tpv {
            if rest.len() < datetime::TP_LEN {
                return Err(ProtoError::UnexpectedEof {
                    needed: datetime::TP_LEN,
                    actual: rest.len(),
                });
            }
            let split_at = rest.len() - datetime::TP_LEN;
            let (body_part, tp_part) = rest.split_at(split_at);
            let (tp, _) = Tp::decode(tp_part)?;
            rest = body_part;
            Some(tp)
        } else {
            None
        };

        // dir 传递给 spec-engine：0=下行(主站→终端)，1=上行(终端→主站)
        let dir: Option<&str> = match direction {
            Direction::Down => Some("0"),
            Direction::Up => Some("1"),
        };

        let body = match (afn, direction) {
            (Afn::Ack, _) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::Ack(units)
            }
            // 链路接口检测 AFN=02H（上行带内容，标准6.2.2）
            (Afn::LinkTest, Direction::Up) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::LinkTest(units)
            }
            // 安全认证 AFN=06H（标准6.2.4，上下行都是DA=0+DI+内容）
            (Afn::Auth, _) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::Auth(units)
            }
            // 读参数 AFN=0AH
            (Afn::ReadParam, Direction::Down) => {
                ApplicationBody::ReadParamRequest(body::parse_data_identifiers(rest)?)
            }
            (Afn::ReadParam, Direction::Up) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::ReadParamResponse(units)
            }
            // 写参数 AFN=04H
            (Afn::WriteParam, Direction::Down) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::WriteParamRequest(units)
            }
            (Afn::WriteParam, Direction::Up) => {
                ApplicationBody::WriteParamResponse(body::parse_write_param_errors(rest)?)
            }
            // 读当前数据 AFN=0CH
            (Afn::ReadCurrentData, Direction::Down) => {
                ApplicationBody::ReadCurrentDataRequest(body::parse_data_identifiers(rest)?)
            }
            (Afn::ReadCurrentData, Direction::Up) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::ReadCurrentDataResponse(units)
            }
            // 读历史数据 AFN=0DH
            (Afn::ReadHistoryData, Direction::Down) => {
                ApplicationBody::ReadHistoryDataRequest(body::parse_history_data_queries(rest)?)
            }
            (Afn::ReadHistoryData, Direction::Up) => {
                // AFN=0DH 上行报文格式：DA + DI + N(内容数) + [内容 + 时间] × N
                // 同一个DA+DI下可以有多个时间点的历史数据
                let (units, _) = body::parse_history_data_response(rest, protocol, region, dir)?;
                ApplicationBody::ReadHistoryDataResponse(units)
            }
            // 读事件记录 AFN=0EH
            (Afn::ReadEvent, Direction::Down) => {
                ApplicationBody::ReadEventRequest(body::parse_data_identifiers(rest)?)
            }
            (Afn::ReadEvent, Direction::Up) => {
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::ReadEventResponse(units)
            }
            // 读任务数据 AFN=12H
            (Afn::ReadTask, Direction::Down) => {
                ApplicationBody::ReadTaskDataRequest(body::parse_data_identifiers(rest)?)
            }
            (Afn::ReadTask, Direction::Up) => {
                // AFN=12H 上行报文格式：DA(2) + DI(4) + 任务数据内容
                // 任务数据内容根据数据结构方式字节决定格式
                
                // 先解析 DA
                let (_da, da_consumed) = DataAddress::decode(rest)?;
                let mut offset = da_consumed;
                
                // 解析 DI（任务号）
                if rest.len() < offset + body::DI_LEN {
                    return Err(ProtoError::UnexpectedEof {
                        needed: offset + body::DI_LEN,
                        actual: rest.len(),
                    });
                }
                let _task_di = u32::from_le_bytes([
                    rest[offset],
                    rest[offset + 1],
                    rest[offset + 2],
                    rest[offset + 3],
                ]);
                offset += body::DI_LEN;
                
                // 解析任务数据内容（自描述方式或按任务定义方式）
                let (units, _) = body::parse_task_data(&rest[offset..], protocol, region, dir)?;
                ApplicationBody::ReadTaskDataResponse(units)
            }
            // 读告警数据 AFN=13H
            (Afn::ReadAlarm, Direction::Down) => {
                ApplicationBody::ReadAlarmDataRequest(body::parse_data_identifiers(rest)?)
            }
            (Afn::ReadAlarm, Direction::Up) => {
                // AFN=13H 上行报文格式：标准的 [DA + DI + 内容] 重复
                // 协议说"按时间先后顺序组织"是指多个DA+DI组按时间排序
                // 而不是同一DA+DI下有多条记录
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::ReadAlarmDataResponse(units)
            }
            // 文件传输 AFN=0FH
            (Afn::FileTransfer, Direction::Down) => {
                // 下行报文：DA=0，单个 DA+DI[+内容]
                // DI=E3010001H: 文件传输启动（有内容）
                // DI=E3010002H: 传输文件内容（有内容）
                // DI=E3010003H: 查询文件信息（无内容）
                // 判断是否有内容：尝试解析，如果只有DA+DI则为查询请求
                if rest.len() > DA_LEN + body::DI_LEN {
                    // 有内容：文件传输启动/传输文件内容
                    let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                    ApplicationBody::FileTransferRequest(units)
                } else {
                    // 无内容：查询文件信息
                    ApplicationBody::FileTransferQueryRequest(body::parse_data_identifiers(rest)?)
                }
            }
            (Afn::FileTransfer, Direction::Up) => {
                // 上行报文：
                // - 文件传输启动/传输内容应答：DA+DI+ERR(1字节)
                // - 查询文件信息应答：DA+DI+内容
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::FileTransferResponse(units)
            }
            // 中继转发 AFN=10H
            (Afn::Relay, Direction::Down) => {
                // 下行报文：标准格式 [DA+DI+内容] 循环
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::RelayRequest(units)
            }
            (Afn::Relay, Direction::Up) => {
                // 上行报文：标准格式 [DA+DI+内容] 循环
                let (units, _) = body::parse_data_units(rest, protocol, region, dir, false)?;
                ApplicationBody::RelayResponse(units)
            }
            _ => ApplicationBody::Raw(rest.to_vec()),
        };

        Ok(Self {
            afn,
            seq,
            body,
            tp,
        })
    }

    /// 编码回应用层字节序列（不含链路层帧头帧尾/校验和，那是 `Frame::encode` 的职责）。
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        out.push(self.afn.to_u8());
        out.push(self.seq.encode());
        match &self.body {
            // 带内容的应答报文暂不支持从类型化结构反向编码（spec-engine只提供解码）
            ApplicationBody::Ack(_)
            | ApplicationBody::LinkTest(_)
            | ApplicationBody::Auth(_)
            | ApplicationBody::ReadParamResponse(_)
            | ApplicationBody::WriteParamRequest(_)
            | ApplicationBody::ReadCurrentDataResponse(_)
            | ApplicationBody::ReadHistoryDataResponse(_)
            | ApplicationBody::ReadEventResponse(_)
            | ApplicationBody::ReadTaskDataResponse(_)
            | ApplicationBody::ReadAlarmDataResponse(_)
            | ApplicationBody::FileTransferRequest(_)
            | ApplicationBody::FileTransferResponse(_)
            | ApplicationBody::RelayRequest(_)
            | ApplicationBody::RelayResponse(_) => {
                return Err(ProtoError::OutOfRange(0));
            }
            // 只有DA+DI的请求可以编码
            ApplicationBody::ReadParamRequest(ids)
            | ApplicationBody::ReadCurrentDataRequest(ids)
            | ApplicationBody::ReadEventRequest(ids)
            | ApplicationBody::ReadTaskDataRequest(ids)
            | ApplicationBody::ReadAlarmDataRequest(ids)
            | ApplicationBody::FileTransferQueryRequest(ids) => {
                for id in ids {
                    out.extend_from_slice(&id.encode());
                }
            }
            // 写参数错误响应
            ApplicationBody::WriteParamResponse(errors) => {
                for err in errors {
                    out.extend_from_slice(&err.da.encode());
                    out.extend_from_slice(&err.di.to_le_bytes());
                    out.push(err.err);
                }
            }
            // 读历史数据请求
            ApplicationBody::ReadHistoryDataRequest(queries) => {
                for q in queries {
                    out.extend_from_slice(&q.da.encode());
                    out.extend_from_slice(&q.di.to_le_bytes());
                    out.extend_from_slice(&q.start_time.encode()?);
                    out.extend_from_slice(&q.end_time.encode()?);
                    out.push(q.density.encode());
                }
            }
            ApplicationBody::Raw(bytes) => out.extend_from_slice(bytes),
        }
        if let Some(tp) = self.tp {
            out.extend_from_slice(&tp.encode()?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::body::ack::DI_ACK_ALL;

    #[test]
    fn decode_ack_all_frame() {
        let mut payload = vec![0x00, 0b0110_0000]; // AFN=00H, SEQ: FIR=1,FIN=1,CON=0,PSEQ=0
        let da = DataAddress::terminal();
        payload.extend_from_slice(&da.encode());
        payload.extend_from_slice(&DI_ACK_ALL.to_le_bytes());
        payload.push(0x00); // 内容：0=确认

        let app = ApplicationLayer::decode(&payload, Direction::Up, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::Ack);
        assert!(app.seq.is_single_frame());
        let units = match &app.body {
            ApplicationBody::Ack(units) => units,
            other => panic!("期望 Ack，实际 {other:?}"),
        };
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].di, DI_ACK_ALL);
        assert_eq!(ack::is_positive(&units[0].value), Some(true));
    }

    #[test]
    fn decode_read_current_data_request() {
        let mut payload = vec![0x0C, 0b0110_0000];
        let id = DataIdentifier {
            da: DataAddress::from_point_number(1).unwrap(),
            di: 0x00010000,
        };
        payload.extend_from_slice(&id.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(
            app.body,
            ApplicationBody::ReadCurrentDataRequest(vec![id])
        );
        assert_eq!(app.encode().unwrap(), payload);
    }

    #[test]
    fn decode_read_param_request() {
        let mut payload = vec![0x0A, 0b0110_0000]; // AFN=0AH
        let id = DataIdentifier {
            da: DataAddress::terminal(),
            di: 0x04000001, // 示例DI
        };
        payload.extend_from_slice(&id.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::ReadParam);
        match &app.body {
            ApplicationBody::ReadParamRequest(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], id);
            }
            other => panic!("期望 ReadParamRequest，实际 {other:?}"),
        }
    }

    #[test]
    fn decode_write_param_response() {
        let mut payload = vec![0x04, 0b0110_0000]; // AFN=04H
        let err = WriteParamError {
            da: DataAddress::terminal(),
            di: 0x04000001,
            err: 0x01, // 错误码1
        };
        payload.extend_from_slice(&err.da.encode());
        payload.extend_from_slice(&err.di.to_le_bytes());
        payload.push(err.err);

        let app = ApplicationLayer::decode(&payload, Direction::Up, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::WriteParam);
        match &app.body {
            ApplicationBody::WriteParamResponse(errors) => {
                assert_eq!(errors.len(), 1);
                assert_eq!(errors[0], err);
            }
            other => panic!("期望 WriteParamResponse，实际 {other:?}"),
        }
    }

    #[test]
    fn decode_read_history_data_request() {
        let mut payload = vec![0x0D, 0b0110_0000]; // AFN=0DH
        let query = HistoryDataQuery {
            da: DataAddress::from_point_number(1).unwrap(),
            di: 0x05000101,
            start_time: DataTime {
                year: 2024,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
            },
            end_time: DataTime {
                year: 2024,
                month: 1,
                day: 2,
                hour: 0,
                minute: 0,
            },
            density: Density::Daily,
        };
        payload.extend_from_slice(&query.da.encode());
        payload.extend_from_slice(&query.di.to_le_bytes());
        payload.extend_from_slice(&query.start_time.encode().unwrap());
        payload.extend_from_slice(&query.end_time.encode().unwrap());
        payload.push(query.density.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::ReadHistoryData);
        match &app.body {
            ApplicationBody::ReadHistoryDataRequest(queries) => {
                assert_eq!(queries.len(), 1);
                assert_eq!(queries[0], query);
            }
            other => panic!("期望 ReadHistoryDataRequest，实际 {other:?}"),
        }
    }

    #[test]
    fn decode_read_event_request() {
        let mut payload = vec![0x0E, 0b0110_0000]; // AFN=0EH
        let id = DataIdentifier {
            da: DataAddress::terminal(),
            di: 0x02000001,
        };
        payload.extend_from_slice(&id.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::ReadEvent);
        match &app.body {
            ApplicationBody::ReadEventRequest(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], id);
            }
            other => panic!("期望 ReadEventRequest，实际 {other:?}"),
        }
    }

    #[test]
    fn decode_read_task_data_request() {
        let mut payload = vec![0x12, 0b0110_0000]; // AFN=12H
        let id = DataIdentifier {
            da: DataAddress::terminal(),
            di: 0x03000001,
        };
        payload.extend_from_slice(&id.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::ReadTask);
        match &app.body {
            ApplicationBody::ReadTaskDataRequest(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], id);
            }
            other => panic!("期望 ReadTaskDataRequest，实际 {other:?}"),
        }
    }

    #[test]
    fn decode_read_alarm_data_request() {
        let mut payload = vec![0x13, 0b0110_0000]; // AFN=13H
        let id = DataIdentifier {
            da: DataAddress::terminal(),
            di: 0x02010001,
        };
        payload.extend_from_slice(&id.encode());

        let app = ApplicationLayer::decode(&payload, Direction::Down, "csg13", "南网").unwrap();
        assert_eq!(app.afn, Afn::ReadAlarm);
        match &app.body {
            ApplicationBody::ReadAlarmDataRequest(ids) => {
                assert_eq!(ids.len(), 1);
                assert_eq!(ids[0], id);
            }
            other => panic!("期望 ReadAlarmDataRequest，实际 {other:?}"),
        }
    }
}

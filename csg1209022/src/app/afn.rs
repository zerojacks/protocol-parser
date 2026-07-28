//! 应用层功能码 AFN（标准 6.1.1，表 6-1；及 5.1.3.6 链路层/应用层功能码对应表）。

use crate::error::{ProtoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Afn {
    /// 00H 确认/否定
    Ack,
    /// 01H 复位（链路层功能码=1 对应的应用层码，标准 5.1.3.6 对应表）
    Reset,
    /// 02H 链路接口检测
    LinkTest,
    /// 04H 写参数
    WriteParam,
    /// 06H 安全认证
    Auth,
    /// 0AH 读参数
    ReadParam,
    /// 0CH 读当前数据
    ReadCurrentData,
    /// 0DH 读历史数据
    ReadHistoryData,
    /// 0EH 读事件记录
    ReadEvent,
    /// 0FH 文件传输
    FileTransfer,
    /// 10H 中继转发
    Relay,
    /// 12H 读任务数据
    ReadTask,
    /// 13H 读告警数据
    ReadAlarm,
    /// 15H 用户自定义数据
    CustomData,
    /// 20H 终端请求主站数据
    TerminalRequest,
    /// 标准之外/尚未收录的功能码，保留原始字节以便上层自行处理
    Other(u8),
}

impl Afn {
    pub fn to_u8(self) -> u8 {
        match self {
            Afn::Ack => 0x00,
            Afn::Reset => 0x01,
            Afn::LinkTest => 0x02,
            Afn::WriteParam => 0x04,
            Afn::Auth => 0x06,
            Afn::ReadParam => 0x0A,
            Afn::ReadCurrentData => 0x0C,
            Afn::ReadHistoryData => 0x0D,
            Afn::ReadEvent => 0x0E,
            Afn::FileTransfer => 0x0F,
            Afn::Relay => 0x10,
            Afn::ReadTask => 0x12,
            Afn::ReadAlarm => 0x13,
            Afn::CustomData => 0x15,
            Afn::TerminalRequest => 0x20,
            Afn::Other(b) => b,
        }
    }

    pub fn from_u8(byte: u8) -> Self {
        match byte {
            0x00 => Afn::Ack,
            0x01 => Afn::Reset,
            0x02 => Afn::LinkTest,
            0x04 => Afn::WriteParam,
            0x06 => Afn::Auth,
            0x0A => Afn::ReadParam,
            0x0C => Afn::ReadCurrentData,
            0x0D => Afn::ReadHistoryData,
            0x0E => Afn::ReadEvent,
            0x0F => Afn::FileTransfer,
            0x10 => Afn::Relay,
            0x12 => Afn::ReadTask,
            0x13 => Afn::ReadAlarm,
            0x15 => Afn::CustomData,
            0x20 => Afn::TerminalRequest,
            other => Afn::Other(other),
        }
    }

    /// 严格版本：遇到标准未定义的功能码时返回错误，而不是 `Other`。
    /// 供需要"拒绝未知功能码"的调用方使用。
    pub fn try_from_u8_strict(byte: u8) -> Result<Self> {
        let afn = Self::from_u8(byte);
        if matches!(afn, Afn::Other(_)) {
            return Err(ProtoError::UnknownAfn(byte));
        }
        Ok(afn)
    }

    /// 人类可读的中文功能描述，供报文可视化/日志使用（标准 6.1.1 表 6-1 的"应用功能定义"列）。
    pub fn description(&self) -> String {
        match self {
            Afn::Ack => "确认/否定".to_string(),
            Afn::Reset => "复位".to_string(),
            Afn::LinkTest => "链路接口检测".to_string(),
            Afn::WriteParam => "写参数".to_string(),
            Afn::Auth => "安全认证".to_string(),
            Afn::ReadParam => "读参数".to_string(),
            Afn::ReadCurrentData => "读当前数据".to_string(),
            Afn::ReadHistoryData => "读历史数据".to_string(),
            Afn::ReadEvent => "读事件记录".to_string(),
            Afn::FileTransfer => "文件传输".to_string(),
            Afn::Relay => "中继转发".to_string(),
            Afn::ReadTask => "读任务数据".to_string(),
            Afn::ReadAlarm => "读告警数据".to_string(),
            Afn::CustomData => "用户自定义数据".to_string(),
            Afn::TerminalRequest => "终端请求主站数据".to_string(),
            Afn::Other(b) => format!("未知功能码(0x{b:02X})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_known_codes() {
        for &code in &[
            0x00u8, 0x01, 0x02, 0x04, 0x06, 0x0A, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x12, 0x13, 0x15,
            0x20,
        ] {
            let afn = Afn::from_u8(code);
            assert_eq!(afn.to_u8(), code);
            assert!(!matches!(afn, Afn::Other(_)));
        }
    }

    #[test]
    fn unknown_code_is_other_but_roundtrips() {
        let afn = Afn::from_u8(0x99);
        assert_eq!(afn, Afn::Other(0x99));
        assert_eq!(afn.to_u8(), 0x99);
        assert!(Afn::try_from_u8_strict(0x99).is_err());
    }

    #[test]
    fn description_is_non_empty_for_all_known_codes() {
        for &code in &[0x00u8, 0x01, 0x0A, 0x0C, 0x20] {
            assert!(!Afn::from_u8(code).description().is_empty());
        }
    }
}

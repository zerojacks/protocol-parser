//! 数据时间域、时间标签 Tp、数据密度（标准 6.1.6、6.1.8）。

use crate::error::{ProtoError, Result};

/// 单个 BCD 字节解码为 0~99 的数值，解码失败转换成 `ProtoError::Bcd`。
fn bcd_byte(byte: u8) -> Result<u8> {
    Ok(proto_common::bcd::decode(&[byte])? as u8)
}

fn bcd_byte_encode(value: u8) -> Result<u8> {
    proto_common::bcd::encode(value as u64, 1)
        .map(|v| v[0])
        .ok_or(ProtoError::OutOfRange(value as u32))
}

pub const DATA_TIME_LEN: usize = 6;

/// 数据时间（标准 6.1.6.1/6.1.6.2），BCD 编码，按年月日时分顺序传输，
/// 年份用2字节 BCD（如 2013 年传输为 `20 13`，按书写顺序、非低字节在前）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl DataTime {
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < DATA_TIME_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: DATA_TIME_LEN,
                actual: buf.len(),
            });
        }
        // 年份两字节按书写顺序拼接：如 "20 13" -> 2013，不做低字节在前的翻转
        let year = bcd_byte(buf[0])? as u16 * 100 + bcd_byte(buf[1])? as u16;
        let month = bcd_byte(buf[2])?;
        let day = bcd_byte(buf[3])?;
        let hour = bcd_byte(buf[4])?;
        let minute = bcd_byte(buf[5])?;
        Ok((
            Self {
                year,
                month,
                day,
                hour,
                minute,
            },
            DATA_TIME_LEN,
        ))
    }

    pub fn encode(&self) -> Result<[u8; DATA_TIME_LEN]> {
        let century_part = (self.year / 100) as u8;
        let year_part = (self.year % 100) as u8;
        Ok([
            bcd_byte_encode(century_part)?,
            bcd_byte_encode(year_part)?,
            bcd_byte_encode(self.month)?,
            bcd_byte_encode(self.day)?,
            bcd_byte_encode(self.hour)?,
            bcd_byte_encode(self.minute)?,
        ])
    }
}

/// 数据密度 m（标准 6.1.6.3，表 6-5）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// 终端历史数据存储密度（按实际存储间隔）
    Native,
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    SixtyMinutes,
    Daily,
    Monthly,
    SettlementDay,
    Reserved(u8),
}

impl Density {
    pub fn description(self) -> String {
        match self {
            Density::Native => "终端存储密度".to_string(),
            Density::OneMinute => "1分钟".to_string(),
            Density::FiveMinutes => "5分钟".to_string(),
            Density::FifteenMinutes => "15分钟".to_string(),
            Density::ThirtyMinutes => "30分钟".to_string(),
            Density::SixtyMinutes => "60分钟".to_string(),
            Density::Daily => "日".to_string(),
            Density::Monthly => "月".to_string(),
            Density::SettlementDay => "结算日".to_string(),
            Density::Reserved(value) => format!("保留值({value})"),
        }
    }

    pub fn decode(byte: u8) -> Self {
        match byte {
            0 => Density::Native,
            1 => Density::OneMinute,
            2 => Density::FiveMinutes,
            3 => Density::FifteenMinutes,
            4 => Density::ThirtyMinutes,
            5 => Density::SixtyMinutes,
            6 => Density::Daily,
            7 => Density::Monthly,
            8 => Density::SettlementDay,
            other => Density::Reserved(other),
        }
    }

    pub fn encode(self) -> u8 {
        match self {
            Density::Native => 0,
            Density::OneMinute => 1,
            Density::FiveMinutes => 2,
            Density::FifteenMinutes => 3,
            Density::ThirtyMinutes => 4,
            Density::SixtyMinutes => 5,
            Density::Daily => 6,
            Density::Monthly => 7,
            Density::SettlementDay => 8,
            Density::Reserved(b) => b,
        }
    }
}

pub const TP_LEN: usize = 5;

/// 时间标签 Tp（标准 6.1.8，表 6-6），用于交换网络通道下辅助判断报文时序/时效性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tp {
    /// 启动帧发送时标：日
    pub day: u8,
    /// 启动帧发送时标：时
    pub hour: u8,
    /// 启动帧发送时标：分
    pub minute: u8,
    /// 启动帧发送时标：秒
    pub second: u8,
    /// 允许发送传输延时时间（分钟，BIN编码）；为0表示从动站不做时效性判断
    pub allowed_delay_minutes: u8,
}

impl Tp {
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < TP_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: TP_LEN,
                actual: buf.len(),
            });
        }
        Ok((
            Self {
                day: bcd_byte(buf[0])?,
                hour: bcd_byte(buf[1])?,
                minute: bcd_byte(buf[2])?,
                second: bcd_byte(buf[3])?,
                allowed_delay_minutes: buf[4],
            },
            TP_LEN,
        ))
    }

    pub fn encode(&self) -> Result<[u8; TP_LEN]> {
        Ok([
            bcd_byte_encode(self.day)?,
            bcd_byte_encode(self.hour)?,
            bcd_byte_encode(self.minute)?,
            bcd_byte_encode(self.second)?,
            self.allowed_delay_minutes,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_matches_standard_example() {
        // 标准示例：2013年5月9日13时14分 -> 20 13 05 09 13 14
        let bytes = [0x20, 0x13, 0x05, 0x09, 0x13, 0x14];
        let (dt, consumed) = DataTime::decode(&bytes).unwrap();
        assert_eq!(consumed, DATA_TIME_LEN);
        assert_eq!(
            dt,
            DataTime {
                year: 2013,
                month: 5,
                day: 9,
                hour: 13,
                minute: 14,
            }
        );
        assert_eq!(dt.encode().unwrap(), bytes);
    }

    #[test]
    fn density_roundtrip() {
        for &b in &[0u8, 3, 6, 7, 8, 99] {
            let d = Density::decode(b);
            assert_eq!(d.encode(), b);
        }
        assert_eq!(Density::decode(0).description(), "终端存储密度");
        assert_eq!(Density::decode(1).description(), "1分钟");
        assert_eq!(Density::decode(99).description(), "保留值(99)");
    }

    #[test]
    fn tp_roundtrip() {
        let bytes = [0x09, 0x13, 0x14, 0x30, 0x05];
        let (tp, consumed) = Tp::decode(&bytes).unwrap();
        assert_eq!(consumed, TP_LEN);
        assert_eq!(tp.encode().unwrap(), bytes);
    }
}

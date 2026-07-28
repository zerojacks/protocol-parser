//! 控制码解析（1字节，包含方向、状态、后续标志和功能码）

use crate::error::{Error, Result};

/// 控制码（1字节）
///
/// 位布局：
/// ```text
///  D7        D6              D5              D4~D0
/// 方向      从站应答标志    后续数据帧标志    功能码
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlCode {
    /// 传输方向
    pub direction: Direction,
    /// 从站应答状态（仅在上行时有效）
    pub slave_status: SlaveStatus,
    /// 是否有后续数据帧
    pub has_follow_up: bool,
    /// 功能码
    pub function: FunctionCode,
}

impl ControlCode {
    /// 从字节解析控制码
    pub fn from_byte(byte: u8) -> Result<Self> {
        let direction = if byte & 0x80 != 0 {
            Direction::SlaveToMaster
        } else {
            Direction::MasterToSlave
        };

        let slave_status = if byte & 0x40 != 0 {
            SlaveStatus::Abnormal
        } else {
            SlaveStatus::Normal
        };

        let has_follow_up = byte & 0x20 != 0;

        let function_bits = byte & 0x1F;
        let function = FunctionCode::from_bits(function_bits)?;

        Ok(Self {
            direction,
            slave_status,
            has_follow_up,
            function,
        })
    }

    /// 转换为字节
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;

        if self.direction == Direction::SlaveToMaster {
            byte |= 0x80;
        }

        if self.slave_status == SlaveStatus::Abnormal {
            byte |= 0x40;
        }

        if self.has_follow_up {
            byte |= 0x20;
        }

        byte |= self.function.to_bits();

        byte
    }

    /// 创建主站请求控制码
    pub fn master_request(function: FunctionCode) -> Self {
        Self {
            direction: Direction::MasterToSlave,
            slave_status: SlaveStatus::Normal,
            has_follow_up: false,
            function,
        }
    }

    /// 创建从站正常应答控制码
    pub fn slave_response(function: FunctionCode, has_follow_up: bool) -> Self {
        Self {
            direction: Direction::SlaveToMaster,
            slave_status: SlaveStatus::Normal,
            has_follow_up,
            function,
        }
    }

    /// 创建从站异常应答控制码
    pub fn slave_error_response(function: FunctionCode) -> Self {
        Self {
            direction: Direction::SlaveToMaster,
            slave_status: SlaveStatus::Abnormal,
            has_follow_up: false,
            function,
        }
    }

    /// 判断是否为错误响应
    pub fn is_error(&self) -> bool {
        self.direction == Direction::SlaveToMaster && self.slave_status == SlaveStatus::Abnormal
    }
}

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 主站->从站（下行）
    MasterToSlave,
    /// 从站->主站（上行）
    SlaveToMaster,
}

/// 从站应答状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlaveStatus {
    /// 正常应答
    Normal,
    /// 异常应答
    Abnormal,
}

/// 功能码（D4~D0，5位）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionCode {
    /// 保留
    Reserved,
    /// 08H - 广播校时
    BroadcastTime,
    /// 11H - 读数据
    ReadData,
    /// 12H - 读后续数据
    ReadFollowUp,
    /// 13H - 读通信地址
    ReadAddress,
    /// 14H - 写数据
    WriteData,
    /// 15H - 写通信地址
    WriteAddress,
    /// 16H - 冻结命令
    Freeze,
    /// 17H - 更改通信速率
    ChangeBaudRate,
    /// 18H - 修改密码
    ChangePassword,
    /// 19H - 最大需量清零
    ClearMaxDemand,
    /// 1AH - 电表清零
    ClearMeter,
    /// 1BH - 事件清零
    ClearEvent,
}

impl FunctionCode {
    /// 从5位功能码解析
    fn from_bits(bits: u8) -> Result<Self> {
        match bits {
            0x00 => Ok(FunctionCode::Reserved),
            0x08 => Ok(FunctionCode::BroadcastTime),
            0x11 => Ok(FunctionCode::ReadData),
            0x12 => Ok(FunctionCode::ReadFollowUp),
            0x13 => Ok(FunctionCode::ReadAddress),
            0x14 => Ok(FunctionCode::WriteData),
            0x15 => Ok(FunctionCode::WriteAddress),
            0x16 => Ok(FunctionCode::Freeze),
            0x17 => Ok(FunctionCode::ChangeBaudRate),
            0x18 => Ok(FunctionCode::ChangePassword),
            0x19 => Ok(FunctionCode::ClearMaxDemand),
            0x1A => Ok(FunctionCode::ClearMeter),
            0x1B => Ok(FunctionCode::ClearEvent),
            _ => Err(Error::InvalidFunctionCode { code: bits }),
        }
    }

    /// 转换为5位功能码
    pub fn to_bits(&self) -> u8 {
        match self {
            FunctionCode::Reserved => 0x00,
            FunctionCode::BroadcastTime => 0x08,
            FunctionCode::ReadData => 0x11,
            FunctionCode::ReadFollowUp => 0x12,
            FunctionCode::ReadAddress => 0x13,
            FunctionCode::WriteData => 0x14,
            FunctionCode::WriteAddress => 0x15,
            FunctionCode::Freeze => 0x16,
            FunctionCode::ChangeBaudRate => 0x17,
            FunctionCode::ChangePassword => 0x18,
            FunctionCode::ClearMaxDemand => 0x19,
            FunctionCode::ClearMeter => 0x1A,
            FunctionCode::ClearEvent => 0x1B,
        }
    }

    /// 获取主站请求控制码字节
    pub fn master_request_byte(&self) -> u8 {
        self.to_bits()
    }

    /// 获取从站正常应答控制码字节（无后续数据）
    pub fn slave_normal_response_byte(&self) -> u8 {
        0x80 | self.to_bits()
    }

    /// 获取从站正常应答控制码字节（有后续数据）
    pub fn slave_normal_response_with_follow_up_byte(&self) -> u8 {
        0xA0 | self.to_bits()
    }

    /// 获取从站异常应答控制码字节
    pub fn slave_error_response_byte(&self) -> u8 {
        0xC0 | self.to_bits()
    }

    /// 获取功能码描述
    pub fn description(&self) -> &'static str {
        match self {
            FunctionCode::Reserved => "保留",
            FunctionCode::BroadcastTime => "广播校时",
            FunctionCode::ReadData => "读数据",
            FunctionCode::ReadFollowUp => "读后续数据",
            FunctionCode::ReadAddress => "读通信地址",
            FunctionCode::WriteData => "写数据",
            FunctionCode::WriteAddress => "写通信地址",
            FunctionCode::Freeze => "冻结命令",
            FunctionCode::ChangeBaudRate => "更改通信速率",
            FunctionCode::ChangePassword => "修改密码",
            FunctionCode::ClearMaxDemand => "最大需量清零",
            FunctionCode::ClearMeter => "电表清零",
            FunctionCode::ClearEvent => "事件清零",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_code_read_data_request() {
        // 主站读数据请求：C = 11H
        let ctrl = ControlCode::from_byte(0x11).unwrap();
        assert_eq!(ctrl.direction, Direction::MasterToSlave);
        assert_eq!(ctrl.slave_status, SlaveStatus::Normal);
        assert!(!ctrl.has_follow_up);
        assert_eq!(ctrl.function, FunctionCode::ReadData);
        assert_eq!(ctrl.to_byte(), 0x11);
    }

    #[test]
    fn test_control_code_slave_normal_response() {
        // 从站正常应答（无后续）：C = 91H
        let ctrl = ControlCode::from_byte(0x91).unwrap();
        assert_eq!(ctrl.direction, Direction::SlaveToMaster);
        assert_eq!(ctrl.slave_status, SlaveStatus::Normal);
        assert!(!ctrl.has_follow_up);
        assert_eq!(ctrl.function, FunctionCode::ReadData);
        assert!(!ctrl.is_error());
    }

    #[test]
    fn test_control_code_slave_with_follow_up() {
        // 从站应答有后续数据：C = B1H
        let ctrl = ControlCode::from_byte(0xB1).unwrap();
        assert_eq!(ctrl.direction, Direction::SlaveToMaster);
        assert_eq!(ctrl.slave_status, SlaveStatus::Normal);
        assert!(ctrl.has_follow_up);
        assert_eq!(ctrl.function, FunctionCode::ReadData);
    }

    #[test]
    fn test_control_code_slave_error_response() {
        // 从站异常应答：C = D1H
        let ctrl = ControlCode::from_byte(0xD1).unwrap();
        assert_eq!(ctrl.direction, Direction::SlaveToMaster);
        assert_eq!(ctrl.slave_status, SlaveStatus::Abnormal);
        assert!(!ctrl.has_follow_up);
        assert_eq!(ctrl.function, FunctionCode::ReadData);
        assert!(ctrl.is_error());
    }

    #[test]
    fn test_function_code_bytes() {
        assert_eq!(FunctionCode::ReadData.master_request_byte(), 0x11);
        assert_eq!(FunctionCode::ReadData.slave_normal_response_byte(), 0x91);
        assert_eq!(FunctionCode::ReadData.slave_normal_response_with_follow_up_byte(), 0xB1);
        assert_eq!(FunctionCode::ReadData.slave_error_response_byte(), 0xD1);
    }

    #[test]
    fn test_all_function_codes_roundtrip() {
        let codes = [
            0x00, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18, 0x19, 0x1A, 0x1B,
        ];

        for &code in &codes {
            let func = FunctionCode::from_bits(code).unwrap();
            assert_eq!(func.to_bits(), code);
        }
    }
}

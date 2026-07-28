//! 链路层控制域 C（标准 5.1.3）。
//!
//! ```text
//!          D7        D6        D5          D4          D3~D0
//! 下行  传输方向DIR  启动标志PRM  帧计数FCB    帧计数有效FCV   功能码
//! 上行                          要求访问ACD   保留
//! ```
//!
//! D5 在下行报文里是 FCB（帧计数位），在上行报文里是 ACD（要求访问位）——
//! 同一个比特位含义随 `Direction` 变化，因此用一个 `fcb_or_acd` 字段承载，
//! 具体语义由调用方结合 `direction` 自己解释（对应 `fcb()`/`acd()` 两个访问器）。

use crate::error::{ProtoError, Result};

/// 传输方向位 DIR。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 主站发出的下行报文
    Down,
    /// 终端发出的上行报文
    Up,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlField {
    pub direction: Direction,
    /// 启动标志位 PRM：true 表示此帧来自启动站，false 表示来自从动站
    pub prm: bool,
    /// D5 位的原始值：下行时是 FCB（帧计数位），上行时是 ACD（要求访问位）
    fcb_or_acd: bool,
    /// 帧计数有效位 FCV：true 表示 FCB 位有效
    pub fcv: bool,
    /// D3~D0：链路层功能码（0~15）
    pub function_code: u8,
}

impl ControlField {
    pub fn decode(byte: u8) -> Self {
        let direction = if byte & 0b1000_0000 != 0 {
            Direction::Up
        } else {
            Direction::Down
        };
        let prm = byte & 0b0100_0000 != 0;
        let fcb_or_acd = byte & 0b0010_0000 != 0;
        let fcv = byte & 0b0001_0000 != 0;
        let function_code = byte & 0b0000_1111;
        Self {
            direction,
            prm,
            fcb_or_acd,
            fcv,
            function_code,
        }
    }

    pub fn encode(self) -> u8 {
        let mut byte = 0u8;
        if self.direction == Direction::Up {
            byte |= 0b1000_0000;
        }
        if self.prm {
            byte |= 0b0100_0000;
        }
        if self.fcb_or_acd {
            byte |= 0b0010_0000;
        }
        if self.fcv {
            byte |= 0b0001_0000;
        }
        byte | (self.function_code & 0b0000_1111)
    }

    /// 帧计数位 FCB（仅下行报文里有意义，需 `fcv == true` 才有效）
    pub fn fcb(&self) -> bool {
        self.fcb_or_acd
    }

    /// 要求访问位 ACD（仅上行响应报文里有意义）：
    /// true 表示终端有告警数据等待访问。
    pub fn acd(&self) -> bool {
        self.fcb_or_acd
    }

    pub fn new_down(prm: bool, fcb: bool, fcv: bool, function_code: u8) -> Result<Self> {
        if function_code > 0x0F {
            return Err(ProtoError::OutOfRange(function_code as u32));
        }
        Ok(Self {
            direction: Direction::Down,
            prm,
            fcb_or_acd: fcb,
            fcv,
            function_code,
        })
    }

    pub fn new_up(prm: bool, acd: bool, fcv: bool, function_code: u8) -> Result<Self> {
        if function_code > 0x0F {
            return Err(ProtoError::OutOfRange(function_code as u32));
        }
        Ok(Self {
            direction: Direction::Up,
            prm,
            fcb_or_acd: acd,
            fcv,
            function_code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_down_frame() {
        // 请求2级数据（功能码=11）、PRM=1、FCV=1、FCB=0
        let c = ControlField::new_down(true, false, true, 11).unwrap();
        let byte = c.encode();
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded, c);
        assert_eq!(decoded.direction, Direction::Down);
        assert_eq!(decoded.function_code, 11);
        assert!(!decoded.fcb());
    }

    #[test]
    fn roundtrip_up_response_with_acd() {
        // 响应帧：用户数据（功能码=8），ACD=1 表示有告警数据等待访问
        let c = ControlField::new_up(false, true, false, 8).unwrap();
        let byte = c.encode();
        let decoded = ControlField::decode(byte);
        assert_eq!(decoded.direction, Direction::Up);
        assert!(decoded.acd());
        assert_eq!(decoded.function_code, 8);
    }

    #[test]
    fn rejects_function_code_out_of_range() {
        assert!(ControlField::new_down(true, false, true, 0x10).is_err());
    }
}

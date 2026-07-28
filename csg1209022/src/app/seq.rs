//! 帧序列域 SEQ（标准 6.1.2）。
//!
//! ```text
//! D7   D6   D5   D4   D3~D0
//! TpV  FIR  FIN  CON  PSEQ/RSEQ
//! ```

use crate::error::{ProtoError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeqField {
    /// 时间标签有效位：true 表示报文带有时间标签 Tp
    pub tpv: bool,
    /// 首帧标志：本报文的第一帧
    pub fir: bool,
    /// 末帧标志：本报文的最后一帧
    pub fin: bool,
    /// 请求确认标志位：true 表示需要对该帧进行确认
    pub con: bool,
    /// 启动帧序号 PSEQ（下行）/ 响应帧序号 RSEQ（上行），取值 0~15
    pub seq: u8,
}

impl SeqField {
    pub fn decode(byte: u8) -> Self {
        Self {
            tpv: byte & 0b1000_0000 != 0,
            fir: byte & 0b0100_0000 != 0,
            fin: byte & 0b0010_0000 != 0,
            con: byte & 0b0001_0000 != 0,
            seq: byte & 0b0000_1111,
        }
    }

    pub fn encode(self) -> u8 {
        let mut byte = 0u8;
        if self.tpv {
            byte |= 0b1000_0000;
        }
        if self.fir {
            byte |= 0b0100_0000;
        }
        if self.fin {
            byte |= 0b0010_0000;
        }
        if self.con {
            byte |= 0b0001_0000;
        }
        byte | (self.seq & 0b0000_1111)
    }

    pub fn new(tpv: bool, fir: bool, fin: bool, con: bool, seq: u8) -> Result<Self> {
        if seq > 0x0F {
            return Err(ProtoError::OutOfRange(seq as u32));
        }
        Ok(Self {
            tpv,
            fir,
            fin,
            con,
            seq,
        })
    }

    /// 单帧报文（既是首帧又是末帧）的便捷构造函数
    pub fn single_frame(con: bool, seq: u8) -> Result<Self> {
        Self::new(false, true, true, con, seq)
    }

    pub fn is_single_frame(&self) -> bool {
        self.fir && self.fin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let seq = SeqField::new(true, true, false, true, 7).unwrap();
        let byte = seq.encode();
        assert_eq!(SeqField::decode(byte), seq);
    }

    #[test]
    fn single_frame_helper() {
        let seq = SeqField::single_frame(false, 15).unwrap();
        assert!(seq.is_single_frame());
        assert_eq!(seq.seq, 15);
    }

    #[test]
    fn rejects_seq_out_of_range() {
        assert!(SeqField::new(false, true, true, false, 16).is_err());
    }
}

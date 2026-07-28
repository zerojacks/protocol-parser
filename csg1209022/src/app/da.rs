//! 信息点标识 DA（标准 6.1.3）。
//!
//! DA 由信息点元 DA1（1字节，位图，标识组内8个测量点）和信息点组 DA2（1字节，二进制组号）构成。
//! 传输顺序低字节在前，即先传 DA1 后传 DA2。
//!
//! 测量点编号 pn（n = 1~2032）与 (DA1,DA2) 的换算关系：
//! `pn = (DA2 - 1) * 8 + k`，其中 k（1~8）是 DA1 中置位的比特（bit0 -> p(8*(DA2-1)+1) ...）。
//!
//! 两个特殊值：
//! - DA1=0x00 且 DA2=0x00：终端本身的测量点，记为 p0；
//! - DA1=0xFF 且 DA2=0xFF：除终端信息点外的所有测量点。

use crate::error::{ProtoError, Result};

pub const DA_LEN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataAddress {
    /// 信息点元：组内8个测量点的位图
    pub da1: u8,
    /// 信息点组：二进制组号（1~254），配合 DA1 定位 pn
    pub da2: u8,
}

/// DA 所表达的测量点集合，展开成具体点号可能非常大（最多2032个），
/// 所以用一个小枚举描述，而不是无条件展开成 `Vec`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointSelector {
    /// 终端本身，p0
    Terminal,
    /// 除终端外的所有测量点
    All,
    /// 具体点号列表（已按从小到大展开）
    Points(Vec<u32>),
}

impl DataAddress {
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < DA_LEN {
            return Err(ProtoError::UnexpectedEof {
                needed: DA_LEN,
                actual: buf.len(),
            });
        }
        // 传输顺序：DA1 在前，DA2 在后
        Ok((
            Self {
                da1: buf[0],
                da2: buf[1],
            },
            DA_LEN,
        ))
    }

    pub fn encode(&self) -> [u8; DA_LEN] {
        [self.da1, self.da2]
    }

    pub fn terminal() -> Self {
        Self { da1: 0x00, da2: 0x00 }
    }

    pub fn all_points() -> Self {
        Self { da1: 0xFF, da2: 0xFF }
    }

    /// 单点便捷构造：给定 pn（1~2032），计算出对应的 DA1/DA2
    pub fn from_point_number(pn: u32) -> Result<Self> {
        if pn == 0 || pn > 2032 {
            return Err(ProtoError::OutOfRange(pn));
        }
        let zero_based = pn - 1;
        let group = (zero_based / 8) as u8; // 0-based group
        let bit = (zero_based % 8) as u8;
        Ok(Self {
            da1: 1u8 << bit,
            da2: group + 1,
        })
    }

    pub fn point_selector(&self) -> PointSelector {
        if self.da1 == 0x00 && self.da2 == 0x00 {
            return PointSelector::Terminal;
        }
        if self.da1 == 0xFF && self.da2 == 0xFF {
            return PointSelector::All;
        }
        let mut points = Vec::new();
        for bit in 0..8u32 {
            if self.da1 & (1 << bit) != 0 {
                let pn = (self.da2 as u32 - 1) * 8 + bit + 1;
                points.push(pn);
            }
        }
        PointSelector::Points(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_point() {
        let da = DataAddress::terminal();
        assert_eq!(da.point_selector(), PointSelector::Terminal);
    }

    #[test]
    fn all_points_marker() {
        let da = DataAddress::all_points();
        assert_eq!(da.point_selector(), PointSelector::All);
    }

    #[test]
    fn example_from_standard_p1_p2() {
        // 标准示例：DA2=01H, DA1=03H 表示测量点 p1、p2
        let da = DataAddress { da1: 0x03, da2: 0x01 };
        assert_eq!(da.point_selector(), PointSelector::Points(vec![1, 2]));
    }

    #[test]
    fn single_point_roundtrip() {
        for pn in [1u32, 8, 9, 2032] {
            let da = DataAddress::from_point_number(pn).unwrap();
            assert_eq!(da.point_selector(), PointSelector::Points(vec![pn]));
        }
    }

    #[test]
    fn rejects_point_number_out_of_range() {
        assert!(DataAddress::from_point_number(0).is_err());
        assert!(DataAddress::from_point_number(2033).is_err());
    }

    #[test]
    fn wire_order_is_da1_then_da2() {
        let bytes = [0x03, 0x01];
        let (da, consumed) = DataAddress::decode(&bytes).unwrap();
        assert_eq!(consumed, 2);
        assert_eq!(da.da1, 0x03);
        assert_eq!(da.da2, 0x01);
        assert_eq!(da.encode(), bytes);
    }
}

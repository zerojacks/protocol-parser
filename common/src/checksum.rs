//! 帧校验和计算。
//!
//! Q/CSG1209022-2019 5.1.5：帧校验和是用户数据区所有字节的八位位组算术和，不考虑溢出位，
//! 用户数据区包括控制域、地址域、链路用户数据（应用层）三部分。
//! DLT/645-2007 的帧校验和算法与此一致（同为逐字节算术和、截断到 8 位），
//! 因此下沉到这里共享；两个协议对"哪些字节参与校验"的界定不同，仍由各自帧层自行切片。

/// 对给定字节序列计算八位位组算术和（逐字节相加，只保留低 8 位）。
pub fn arithmetic_sum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

/// 校验给定数据的校验和是否与期望值一致。
pub fn verify(data: &[u8], expected: u8) -> bool {
    arithmetic_sum(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_wraps_on_overflow() {
        assert_eq!(arithmetic_sum(&[0xFF, 0x02]), 0x01);
    }

    #[test]
    fn empty_input_sums_to_zero() {
        assert_eq!(arithmetic_sum(&[]), 0);
    }

    #[test]
    fn verify_matches_expected() {
        let data = [0x68, 0x01, 0x02, 0x03];
        let sum = arithmetic_sum(&data);
        assert!(verify(&data, sum));
        assert!(!verify(&data, sum.wrapping_add(1)));
    }
}

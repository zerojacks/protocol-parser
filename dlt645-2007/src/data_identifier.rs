//! 数据标识（DI）解析（4字节：DI3 DI2 DI1 DI0）

use crate::error::{Error, Result};

/// 数据标识（4字节，低字节先传：DI0 DI1 DI2 DI3）
///
/// 数据标识用于指定要读取或写入的数据项。
///
/// # 字节布局
///
/// - DI3: 数据类型（00=电能量, 01=最大需量, 02=变量, 03=事件, 04=参数, 05=冻结, 06=负荷记录）
/// - DI2, DI1, DI0: 具体数据项编码
///
/// # 通配符规则
///
/// - 如果 DI2/DI1/DI0 = FFH，表示该维度的"所有值"（数据块读取）
/// - DI3 不能为 FFH
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataIdentifier {
    /// 原始4字节（DI0, DI1, DI2, DI3）
    raw: [u8; 4],
}

impl DataIdentifier {
    /// 数据标识长度
    pub const LEN: usize = 4;
    
    /// 数据块通配符
    pub const WILDCARD: u8 = 0xFF;

    /// 从4字节创建数据标识
    ///
    /// # 参数
    ///
    /// - `bytes`: [DI0, DI1, DI2, DI3]（传输顺序）
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self { raw: bytes }
    }

    /// 从 DI3-DI2-DI1-DI0 顺序创建（人类可读顺序）
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// // 00 01 00 00 - 当前正向有功总电能
    /// let di = DataIdentifier::from_di(0x00, 0x01, 0x00, 0x00);
    /// ```
    pub fn from_di(di3: u8, di2: u8, di1: u8, di0: u8) -> Self {
        Self::from_bytes([di0, di1, di2, di3])
    }

    /// 从 u32 创建（按 DI3DI2DI1DI0 顺序）
    pub fn from_u32(value: u32) -> Self {
        Self::from_bytes([
            (value & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 24) & 0xFF) as u8,
        ])
    }

    /// 转换为 u32（DI3 在最高字节）
    pub fn to_u32(&self) -> u32 {
        u32::from_le_bytes(self.raw)
    }

    /// 获取 DI0（最低字节）
    pub fn di0(&self) -> u8 {
        self.raw[0]
    }

    /// 获取 DI1
    pub fn di1(&self) -> u8 {
        self.raw[1]
    }

    /// 获取 DI2
    pub fn di2(&self) -> u8 {
        self.raw[2]
    }

    /// 获取 DI3（数据类型）
    pub fn di3(&self) -> u8 {
        self.raw[3]
    }

    /// 获取数据类型分类
    pub fn category(&self) -> DataCategory {
        DataCategory::from_di3(self.di3())
    }

    /// 判断是否为数据块（包含通配符）
    pub fn is_block(&self) -> bool {
        self.raw[0] == Self::WILDCARD
            || self.raw[1] == Self::WILDCARD
            || self.raw[2] == Self::WILDCARD
    }

    /// 获取原始字节（传输顺序）
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.raw
    }

    /// 编码为字节数组
    pub fn encode(&self) -> [u8; 4] {
        self.raw
    }

    /// 从字节切片解码
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: buf.len(),
            });
        }

        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&buf[0..4]);
        Ok((Self::from_bytes(bytes), Self::LEN))
    }

    /// 格式化为十六进制字符串（DI3-DI2-DI1-DI0 顺序）
    pub fn to_hex_string(&self) -> String {
        format!(
            "{:02X} {:02X} {:02X} {:02X}",
            self.di3(),
            self.di2(),
            self.di1(),
            self.di0()
        )
    }
}

impl std::fmt::Display for DataIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex_string())
    }
}

/// 数据类型分类（根据 DI3 划分）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCategory {
    /// 00H - 电能量（kWh/kvarh/kVAh）
    Energy,
    /// 01H - 最大需量及发生时间
    MaxDemand,
    /// 02H - 变量（电压、电流、功率等）
    Variables,
    /// 03H - 事件记录
    Events,
    /// 04H - 参变量（日期时间、费率等）
    Parameters,
    /// 05H - 冻结量
    Freeze,
    /// 06H - 负荷记录
    LoadProfile,
    /// 其他/未知
    Unknown(u8),
}

impl DataCategory {
    /// 从 DI3 字节获取分类
    pub fn from_di3(di3: u8) -> Self {
        match di3 {
            0x00 => DataCategory::Energy,
            0x01 => DataCategory::MaxDemand,
            0x02 => DataCategory::Variables,
            0x03 => DataCategory::Events,
            0x04 => DataCategory::Parameters,
            0x05 => DataCategory::Freeze,
            0x06 => DataCategory::LoadProfile,
            other => DataCategory::Unknown(other),
        }
    }

    /// 获取分类描述
    pub fn description(&self) -> &'static str {
        match self {
            DataCategory::Energy => "电能量",
            DataCategory::MaxDemand => "最大需量",
            DataCategory::Variables => "变量",
            DataCategory::Events => "事件记录",
            DataCategory::Parameters => "参变量",
            DataCategory::Freeze => "冻结量",
            DataCategory::LoadProfile => "负荷记录",
            DataCategory::Unknown(_) => "未知类型",
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_identifier_creation() {
        let di = DataIdentifier::from_di(0x00, 0x01, 0x00, 0x00);
        assert_eq!(di.di3(), 0x00);
        assert_eq!(di.di2(), 0x01);
        assert_eq!(di.di1(), 0x00);
        assert_eq!(di.di0(), 0x00);
    }

    #[test]
    fn test_data_identifier_roundtrip() {
        let original = DataIdentifier::from_di(0x02, 0x01, 0x01, 0x00);
        let bytes = original.encode();
        let (decoded, consumed) = DataIdentifier::decode(&bytes).unwrap();
        assert_eq!(consumed, 4);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_data_identifier_to_u32() {
        // DI = 00 01 00 00 → u32 = 0x00000100
        let di = DataIdentifier::from_di(0x00, 0x01, 0x00, 0x00);
        assert_eq!(di.to_u32(), 0x00010000);
    }

    #[test]
    fn test_data_category() {
        let di_energy = DataIdentifier::from_di(0x00, 0x01, 0x00, 0x00);
        assert_eq!(di_energy.category(), DataCategory::Energy);
        
        let di_voltage = DataIdentifier::from_di(0x02, 0x01, 0x01, 0x00);
        assert_eq!(di_voltage.category(), DataCategory::Variables);
    }

    #[test]
    fn test_data_block_wildcard() {
        let di_block = DataIdentifier::from_di(0x02, 0x01, 0xFF, 0x00);
        assert!(di_block.is_block());
        
        let di_single = DataIdentifier::from_di(0x02, 0x01, 0x01, 0x00);
        assert!(!di_single.is_block());
    }
}

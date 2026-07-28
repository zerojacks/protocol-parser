//! 地址域解析（6字节BCD编码，表示12位十进制地址）

use crate::error::{Error, Result};
use crate::{BROADCAST_ADDRESS, WILDCARD_BYTE};

/// 地址域（6字节BCD编码，低字节先传）
///
/// 地址域用BCD码表示，每个字节表示两位十进制数。
/// 传输顺序：A0 A1 A2 A3 A4 A5（低字节在前）
/// 表示地址：A5A4A3A2A1A0（高位在前）
///
/// # 特殊地址
///
/// - `999999999999`：广播地址
/// - 包含 `AA`：通配符（"不关心"匹配）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// 原始6字节（传输顺序：A0..A5）
    raw: [u8; 6],
}

impl Address {
    /// 地址域字节长度
    pub const LEN: usize = 6;

    /// 从6字节解析地址
    ///
    /// # 参数
    ///
    /// - `bytes`: 6字节数组，顺序为 A0 A1 A2 A3 A4 A5（传输顺序）
    pub fn from_bytes(bytes: [u8; 6]) -> Result<Self> {
        // 验证每个字节是否为有效的BCD（0x00-0x99 或 0xAA通配符）
        for &byte in &bytes {
            if byte != WILDCARD_BYTE && !is_valid_bcd(byte) {
                return Err(Error::InvalidBcd { byte });
            }
        }
        Ok(Self { raw: bytes })
    }

    /// 从12位十进制字符串创建地址
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let addr = Address::from_decimal_str("123456789012")?;
    /// ```
    pub fn from_decimal_str(s: &str) -> Result<Self> {
        if s.len() != 12 {
            return Err(Error::InvalidAddress {
                reason: format!("地址长度必须为12位，实际 {} 位", s.len()),
            });
        }

        let mut bytes = [0u8; 6];
        // 从高位到低位，每两位转换为一个BCD字节
        for i in 0..6 {
            let start = (5 - i) * 2; // 从字符串高位开始
            let two_digits = &s[start..start + 2];
            let value = two_digits.parse::<u8>().map_err(|_| Error::InvalidAddress {
                reason: format!("无效的十进制数字: {}", two_digits),
            })?;
            
            if value > 99 {
                return Err(Error::InvalidAddress {
                    reason: format!("数字超出范围: {}", value),
                });
            }
            
            // 转换为BCD（十位在高4位，个位在低4位）
            bytes[i] = ((value / 10) << 4) | (value % 10);
        }

        Ok(Self { raw: bytes })
    }

    /// 创建广播地址
    pub fn broadcast() -> Self {
        Self::from_decimal_str(BROADCAST_ADDRESS).unwrap()
    }

    /// 判断是否为广播地址
    pub fn is_broadcast(&self) -> bool {
        self.raw.iter().all(|&b| b == 0x99)
    }

    /// 判断是否包含通配符
    pub fn has_wildcard(&self) -> bool {
        self.raw.iter().any(|&b| b == WILDCARD_BYTE)
    }

    /// 转换为12位十进制字符串
    pub fn to_decimal_string(&self) -> String {
        let mut result = String::with_capacity(12);
        // 从A5到A0（高位到低位）
        for &byte in self.raw.iter().rev() {
            if byte == WILDCARD_BYTE {
                result.push_str("AA");
            } else {
                let high = (byte >> 4) & 0x0F;
                let low = byte & 0x0F;
                result.push(char::from_digit(high as u32, 10).unwrap_or('?'));
                result.push(char::from_digit(low as u32, 10).unwrap_or('?'));
            }
        }
        result
    }

    /// 获取原始字节（传输顺序）
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.raw
    }

    /// 编码为字节数组
    pub fn encode(&self) -> [u8; 6] {
        self.raw
    }

    /// 解码（从字节切片）
    pub fn decode(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < Self::LEN {
            return Err(Error::UnexpectedEof {
                needed: Self::LEN,
                actual: buf.len(),
            });
        }

        let mut bytes = [0u8; 6];
        bytes.copy_from_slice(&buf[0..6]);
        let addr = Self::from_bytes(bytes)?;
        Ok((addr, Self::LEN))
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_decimal_string())
    }
}

/// 检查字节是否为有效的BCD编码（0x00-0x99）
fn is_valid_bcd(byte: u8) -> bool {
    let high = (byte >> 4) & 0x0F;
    let low = byte & 0x0F;
    high <= 9 && low <= 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_from_decimal_str() {
        let addr = Address::from_decimal_str("123456789012").unwrap();
        assert_eq!(addr.to_decimal_string(), "123456789012");
    }

    #[test]
    fn test_address_roundtrip() {
        let original = "000000000001";
        let addr = Address::from_decimal_str(original).unwrap();
        let bytes = addr.encode();
        let (decoded, consumed) = Address::decode(&bytes).unwrap();
        assert_eq!(consumed, 6);
        assert_eq!(decoded.to_decimal_string(), original);
    }

    #[test]
    fn test_broadcast_address() {
        let addr = Address::broadcast();
        assert!(addr.is_broadcast());
        assert_eq!(addr.to_decimal_string(), "999999999999");
    }

    #[test]
    fn test_invalid_bcd() {
        let bytes = [0x12, 0x9A, 0x78, 0x56, 0x34, 0x12]; // 0x9A 无效
        let result = Address::from_bytes(bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_address_byte_order() {
        // 地址 123456789012
        // 分解：12-34-56-78-90-12（从高到低）
        // BCD: 0x12, 0x34, 0x56, 0x78, 0x90, 0x12（从高到低）
        // 传输顺序（低字节先传）：0x12, 0x90, 0x78, 0x56, 0x34, 0x12
        let addr = Address::from_decimal_str("123456789012").unwrap();
        let bytes = addr.as_bytes();
        assert_eq!(bytes[0], 0x12); // A0 (最低位12)
        assert_eq!(bytes[1], 0x90); // A1 (90)
        assert_eq!(bytes[2], 0x78); // A2 (78)
        assert_eq!(bytes[3], 0x56); // A3 (56)
        assert_eq!(bytes[4], 0x34); // A4 (34)
        assert_eq!(bytes[5], 0x12); // A5 (最高位12)
    }
}

//! 确认/否定报文体（AFN=00H，标准 6.2.1）辅助类型。
//!
//! 结构上和普通信息体一样是 DA+DI+内容 循环（参见 `super::parse_data_units`），
//! **也走 `spec_engine::parse_di` 解析内容**——因为 DI=`0xE0000000`（"全部进行确定/否定"）
//! 已经收录在 csg13 字典里（1字节，BCD）。这里不再单独实现一套 00/01 字节解析，
//! 只提供一个从已解析的 `DataUnit` 判断"是确认还是否定"的便捷 helper。
//!
//! 注：当前字典里 `E0000000` 还没有配 `enum_map`，所以 `spec_engine::parse_di` 目前给出的是
//! `Value::Int(0)`/`Value::Int(1)`，还不会直接产出"01-表示全部否定"这样的文字描述；
//! 如果需要这一层文字，需要在 `spec-engine` 的 schema 里给这个 DI 补充 `enum_map`
//! （那是 spec-engine 自己的事，不在本 crate 里做）。

use proto_common::FieldValue;

/// 全部确认/否认场景使用的特殊数据标识编码
pub const DI_ACK_ALL: u32 = 0xE000_0000;

/// 从已解析的内容值里判断"确认(true)还是否定(false)"，取值规则是 0=确认、非0=否定。
/// 解析不出数值（比如遇到尚未识别的 DI）时返回 `None`，调用方自行决定如何兜底。
pub fn is_positive(value: &FieldValue) -> Option<bool> {
    value.as_int().map(|i| i == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_when_zero() {
        let v = FieldValue::Node {
            name: "E0000000_全部进行确定/否定".into(),
            raw: vec![0],
            value: Box::new(FieldValue::Int(0)),
        };
        assert_eq!(is_positive(&v), Some(true));
    }

    #[test]
    fn negative_when_nonzero() {
        let v = FieldValue::Node {
            name: "E0000000_全部进行确定/否定".into(),
            raw: vec![1],
            value: Box::new(FieldValue::Int(1)),
        };
        assert_eq!(is_positive(&v), Some(false));
    }
}

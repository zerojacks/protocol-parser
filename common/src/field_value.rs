//! 报文展示树用的值类型（跨协议共享）
//!
//! 之所以不直接把 `spec_engine::Value` 到处传（哪怕形状长得一模一样），是因为
//! `spec_engine` 是独立发布、独立演进的外部依赖：它的 `Value` 枚举以后加字段、
//! 加变体、调整语义，我们完全不受控制。如果本 crate 的公开 API（报文展示树）
//! 直接就是 `spec_engine::Value`，那么 spec-engine 每次改动都可能是本 crate的
//! 破坏性变更（breaking change），版本号也被迫跟着它的节奏走。
//!
//! 所以这里定义一份**结构上镜像、但完全独立**的 `FieldValue`，只在 DI 内容解析
//! 结果要嵌入展示树的那一个边界点（[`from_spec_engine`]）做一次转换。以后 spec-engine
//! 的 `Value` 变了，只需要改这一个函数；本 crate 的公开类型和依赖它的下游代码都不受影响。
//!
//! ## 跨协议共享
//!
//! 本模块在 `proto-common` 中定义，被 `csg1209022` 和 `dlt645-2007` 共同使用。
//! 这确保了两个协议库对外暴露的数据结构完全一致，方便调用者统一处理。

#[cfg(feature = "spec-engine-support")]
use spec_engine::Value as SpecValue;

/// 报文展示树里的一个值，语义上和 `spec_engine::Value` 一一对应，但类型独立。
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<FieldValue>),
    Map(Vec<(String, FieldValue)>),
    WithUnit {
        value: Box<FieldValue>,
        unit: String,
    },
    /// 解析节点：字段名称、原始字节、解析值
    Node {
        name: String,
        raw: Vec<u8>,
        value: Box<FieldValue>,
    },
    /// 位字段：区间 + 提取出的数值 + 所在原始字节 + 可选的语义描述
    Bit {
        bit_start: usize,
        bit_end: usize,
        bit_value: u64,
        bit_byte: Vec<u8>,
        value: Option<Box<FieldValue>>,
    },
    Skip,
    Invalid {
        reason: String,
    },
    Pn(i64),
}

impl FieldValue {
    /// 尝试将值提取为整数
    pub fn as_int(&self) -> Option<i64> {
        match self {
            FieldValue::Int(i) => Some(*i),
            FieldValue::Pn(i) => Some(*i),
            FieldValue::WithUnit { value, .. } => value.as_int(),
            FieldValue::Node { value, .. } => value.as_int(),
            FieldValue::Bit { value, .. } => value.as_deref().and_then(|v| v.as_int()),
            _ => None,
        }
    }

    /// 尝试将值提取为字符串引用
    pub fn as_str(&self) -> Option<&str> {
        match self {
            FieldValue::Str(s) => Some(s.as_str()),
            FieldValue::WithUnit { value, .. } => value.as_str(),
            FieldValue::Node { value, .. } => value.as_str(),
            _ => None,
        }
    }

    /// 尝试将值提取为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            FieldValue::Float(f) => Some(*f),
            FieldValue::WithUnit { value, .. } => value.as_float(),
            FieldValue::Node { value, .. } => value.as_float(),
            _ => None,
        }
    }

    /// 取节点自身的 `raw` 字节（非 `Node` 返回空切片）
    pub fn raw(&self) -> &[u8] {
        match self {
            FieldValue::Node { raw, .. } => raw,
            _ => &[],
        }
    }
}

/// 把 `spec_engine::Value` 转换成 [`FieldValue`]。
///
/// 这是**唯一**一处直接匹配 `spec_engine::Value` 全部变体的地方——
/// 特意写成穷尽匹配（没有 `_ =>` 兜底分支），这样 spec-engine 以后新增变体时，
/// 这里会编译失败，逼着我们显式处理新变体，而不是悄悄丢数据。
///
/// # Feature Gate
///
/// 此函数需要 `spec-engine-support` feature 启用。
#[cfg(feature = "spec-engine-support")]
pub fn from_spec_engine(value: &SpecValue) -> FieldValue {
    match value {
        SpecValue::Int(i) => FieldValue::Int(*i),
        SpecValue::Float(f) => FieldValue::Float(*f),
        SpecValue::Str(s) => FieldValue::Str(s.clone()),
        SpecValue::Bytes(b) => FieldValue::Bytes(b.clone()),
        SpecValue::List(items) => FieldValue::List(items.iter().map(from_spec_engine).collect()),
        SpecValue::Map(items) => FieldValue::Map(
            items
                .iter()
                .map(|(k, v)| (k.clone(), from_spec_engine(v)))
                .collect(),
        ),
        SpecValue::WithUnit { value, unit } => FieldValue::WithUnit {
            value: Box::new(from_spec_engine(value)),
            unit: unit.clone(),
        },
        SpecValue::Node { name, raw, value } => FieldValue::Node {
            name: name.clone(),
            raw: raw.clone(),
            value: Box::new(from_spec_engine(value)),
        },
        SpecValue::Bit {
            bit_start,
            bit_end,
            bit_value,
            bit_byte,
            value,
        } => FieldValue::Bit {
            bit_start: *bit_start,
            bit_end: *bit_end,
            bit_value: *bit_value,
            bit_byte: bit_byte.clone(),
            value: value.as_ref().map(|v| Box::new(from_spec_engine(v))),
        },
        SpecValue::Skip => FieldValue::Skip,
        SpecValue::Invalid { reason } => FieldValue::Invalid {
            reason: reason.clone(),
        },
        SpecValue::Pn(i) => FieldValue::Pn(*i),
    }
}

#[cfg(all(test, feature = "spec-engine-support"))]
mod tests {
    use super::*;

    #[test]
    fn converts_node_recursively() {
        let sv = SpecValue::Node {
            name: "foo".into(),
            raw: vec![1, 2],
            value: Box::new(SpecValue::WithUnit {
                value: Box::new(SpecValue::Float(1.23)),
                unit: "kWh".into(),
            }),
        };
        let fv = from_spec_engine(&sv);
        match fv {
            FieldValue::Node { name, raw, value } => {
                assert_eq!(name, "foo");
                assert_eq!(raw, vec![1, 2]);
                match *value {
                    FieldValue::WithUnit { value, unit } => {
                        assert_eq!(unit, "kWh");
                        assert_eq!(*value, FieldValue::Float(1.23));
                    }
                    other => panic!("期望 WithUnit，实际 {other:?}"),
                }
            }
            other => panic!("期望 Node，实际 {other:?}"),
        }
    }

    #[test]
    fn converts_bit_with_nested_description() {
        let sv = SpecValue::Bit {
            bit_start: 7,
            bit_end: 7,
            bit_value: 1,
            bit_byte: vec![0x80],
            value: Some(Box::new(SpecValue::Str("上行".into()))),
        };
        let fv = from_spec_engine(&sv);
        assert_eq!(
            fv,
            FieldValue::Bit {
                bit_start: 7,
                bit_end: 7,
                bit_value: 1,
                bit_byte: vec![0x80],
                value: Some(Box::new(FieldValue::Str("上行".into()))),
            }
        );
    }

    #[test]
    fn test_as_int() {
        let val = FieldValue::Int(42);
        assert_eq!(val.as_int(), Some(42));

        let val_with_unit = FieldValue::WithUnit {
            value: Box::new(FieldValue::Int(100)),
            unit: "A".to_string(),
        };
        assert_eq!(val_with_unit.as_int(), Some(100));
    }

    #[test]
    fn test_as_str() {
        let val = FieldValue::Str("test".to_string());
        assert_eq!(val.as_str(), Some("test"));
    }

    #[test]
    fn test_as_float() {
        let val = FieldValue::Float(3.14);
        assert_eq!(val.as_float(), Some(3.14));
    }
}

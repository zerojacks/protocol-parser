//! 多个计量通信协议解析器共享的极薄工具函数。
//!
//! 这个 crate 故意保持很薄：只放校验和、BCD 编解码这类**纯函数级别**的工具，
//! 不定义任何协议相关的结构体或 trait。每个协议（`csg1209022`、`dlt645-2007` ...）
//! 的帧结构、寻址方式、状态机差异很大，强行抽象成"通用协议层"只会增加维护负担。
//!
//! ## spec-engine 集成支持
//!
//! 当启用 `spec-engine-support` feature 时，本 crate 还提供 `FieldValue` 类型，
//! 用于封装 spec-engine 的解析结果。这确保了所有协议库对外暴露的数据结构一致。

pub mod bcd;
pub mod checksum;

#[cfg(feature = "spec-engine-support")]
pub mod field_value;

pub use bcd::BcdError;

#[cfg(feature = "spec-engine-support")]
pub use field_value::{from_spec_engine, FieldValue};

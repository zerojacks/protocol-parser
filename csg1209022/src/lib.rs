//! 中国南方电网《计量自动化终端上行通信规约》（Q/CSG1209022-2019）解析器。
//!
//! 分层对应标准的三层参考模型：
//!
//! - [`link`]：链路层——FT1.2 帧格式、控制域 C、地址域 A（标准第5章）
//! - [`app`]：应用层——AFN、SEQ、信息点标识 DA、数据标识 DI、数据时间域、时间标签 Tp（标准第6章）
//! - [`engine`]：顶层入口，把链路层和应用层串起来
//!
//! DI 数据标识**内容**本身的解析完全委托给独立发布的 [`spec_engine`](https://github.com/zerojacks/spec-engine)
//! （只是普通依赖引用，不在本 workspace 内），本 crate 只负责帧结构、寻址、AFN分发。
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use csg1209022::decode_message;
//!
//! let (msg, consumed) = decode_message(&raw_bytes, "csg13", "南网")?;
//! println!("{:?}", msg.application.body);
//! ```

pub mod app;
pub mod engine;
pub mod error;
pub mod link;
pub mod report;

pub use engine::{decode_message, encode_message, Message};
pub use error::{ProtoError, Result};
pub use proto_common::FieldValue;
pub use report::decode_message_as_value;

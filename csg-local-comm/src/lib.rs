//! Q/CSG1209021-2019 计量自动化终端本地通信模块接口协议解析器
//!
//! 本库实现了 **Q/CSG1209021-2019《计量自动化终端本地通信模块接口协议》**的完整解析功能。
//! 该协议定义了集中器/采集器与其本地通信模块（载波/微功率无线/以太网）之间的通信规范。
//!
//! ## 协议特点
//!
//! - 帧格式：`68H | L(2B) | C(1B) | [A(12B)] | AFN(1B) | SEQ(1B) | DI(4B) | 数据内容 | CS(1B) | 16H`
//! - 两种节点角色：集中器（DI3=E8H）、采集器（DI3=EAH）
//! - 支持广播地址：`99 99 99 99 99 99H`
//! - 三种传输服务类：发送/无回答、发送/确认、请求/响应
//!
//! ## 架构设计
//!
//! 本库采用三层架构，与 DLT645-2007 和 CSG1209022 保持一致：
//!
//! - **链路层 (link)**：负责帧格式、地址域、控制字节的编解码
//! - **应用层 (app)**：负责 AFN、DI、数据内容的解析
//! - **引擎层 (engine)**：串联链路层和应用层，提供统一的编解码接口，集成 spec-engine
//!
//! ## 使用示例
//!
//! ```rust,ignore
//! use csg1209021::decode_message;
//!
//! let bytes = vec![0x68, 0x25, 0x00, ...]; // 完整帧
//! let (msg, consumed) = decode_message(&bytes, "csg-local-comm", "南网")?;
//!
//! println!("控制字节: {:?}", msg.frame.control);
//! println!("AFN: {:?}", msg.application.afn);
//! println!("DI: {:08X}H", msg.application.di.to_u32());
//!
//! // 渲染成树形结构供前端展示
//! let tree = msg.to_value_tree(&bytes[..consumed])?;
//! ```

pub mod app;
pub mod control;
pub mod engine;
pub mod error;
pub mod link;
pub mod report;

pub use app::{Afn, ApplicationBody, ApplicationLayer, DataIdentifier};
pub use control::{ControlByte, Direction};
pub use engine::{decode_message, encode_message, Message};
pub use error::{Error, Result};
pub use link::{Address, Frame};
pub use proto_common::FieldValue;

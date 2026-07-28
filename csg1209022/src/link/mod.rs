//! 链路层：FT1.2 帧格式、控制域、地址域（标准第5章）。

pub mod address;
pub mod control;
pub mod frame;

pub use address::AddressField;
pub use control::{ControlField, Direction};
pub use frame::Frame;

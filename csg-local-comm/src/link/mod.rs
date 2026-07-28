//! 链路层模块
//!
//! 负责帧格式、地址域、控制字节的编解码

pub mod address;
pub mod frame;

pub use address::{Address, AddressDomain};
pub use frame::Frame;

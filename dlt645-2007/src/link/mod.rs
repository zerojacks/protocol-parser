//! DL/T 645-2007 链路层
//!
//! 链路层负责帧格式、地址和控制码的编解码，
//! 数据域的具体内容由应用层处理。

pub mod frame;

pub use frame::{Frame, find_frame_start};

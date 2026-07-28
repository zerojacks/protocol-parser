//! 统一的多协议解析器
//!
//! 自动检测并解析多种电力通信协议：
//! - DL/T 645-2007: 多功能电能表通信协议
//! - Q/CSG1209022-2019: 计量自动化终端上行通信规约
//! - Q/CSG1209021-2019: 计量自动化终端本地通信模块接口协议
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use protocol_parser::auto_parse;
//!
//! let bytes = vec![0x68, 0x49, 0x00, 0x40, ...];
//! match auto_parse(&bytes) {
//!     Ok(result) => {
//!         println!("协议: {}", result.protocol_name());
//!         println!("解析结果: {:?}", result);
//!     }
//!     Err(e) => eprintln!("解析失败: {}", e),
//! }
//! ```

use proto_common::FieldValue;
use thiserror::Error;

/// 协议类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    /// DL/T 645-2007 多功能电能表通信协议
    Dlt645_2007,
    /// Q/CSG1209022-2019 计量自动化终端上行通信规约
    Csg1209022,
    /// Q/CSG1209021-2019 本地通信模块接口协议
    CsgLocalComm,
}

impl ProtocolType {
    /// 获取协议名称
    pub fn name(&self) -> &'static str {
        match self {
            ProtocolType::Dlt645_2007 => "DL/T 645-2007",
            ProtocolType::Csg1209022 => "Q/CSG1209022-2019",
            ProtocolType::CsgLocalComm => "Q/CSG1209021-2019",
        }
    }

    /// 获取协议标识（用于 spec-engine）
    pub fn identifier(&self) -> &'static str {
        match self {
            ProtocolType::Dlt645_2007 => "dlt645-2007",
            ProtocolType::Csg1209022 => "csg13",
            ProtocolType::CsgLocalComm => "csg-local-comm",
        }
    }
}

/// 统一的解析结果
#[derive(Debug, Clone)]
pub enum ParsedMessage {
    /// DLT645-2007 解析结果
    Dlt645(dlt645_2007::Message),
    /// CSG1209022 解析结果
    Csg1209022(csg1209022::Message),
    /// CSG 本地通信解析结果
    CsgLocalComm(csg_local_comm::Message),
}

impl ParsedMessage {
    /// 获取协议类型
    pub fn protocol_type(&self) -> ProtocolType {
        match self {
            ParsedMessage::Dlt645(_) => ProtocolType::Dlt645_2007,
            ParsedMessage::Csg1209022(_) => ProtocolType::Csg1209022,
            ParsedMessage::CsgLocalComm(_) => ProtocolType::CsgLocalComm,
        }
    }

    /// 获取协议名称
    pub fn protocol_name(&self) -> &'static str {
        self.protocol_type().name()
    }

    /// 转换为树形结构（用于前端展示）
    pub fn to_value_tree(&self) -> Result<FieldValue, ParseError> {
        match self {
            ParsedMessage::Dlt645(_msg) => {
                // 需要原始字节来渲染，这里暂时返回基本信息
                // 实际使用时可能需要保存原始字节
                Err(ParseError::NotImplemented("DLT645 to_value_tree 需要原始字节".to_string()))
            }
            ParsedMessage::Csg1209022(_msg) => {
                Err(ParseError::NotImplemented("CSG1209022 to_value_tree 需要原始字节".to_string()))
            }
            ParsedMessage::CsgLocalComm(msg) => {
                csg_local_comm::report::render_message_as_value(msg)
                    .map_err(|e| ParseError::RenderError(format!("{:?}", e)))
            }
        }
    }
}

/// 解析错误
#[derive(Error, Debug)]
pub enum ParseError {
    /// 无法识别协议类型
    #[error("无法识别协议类型：数据不符合任何已知协议格式")]
    UnknownProtocol,

    /// DLT645 解析错误
    #[error("DLT645-2007 解析错误: {0}")]
    Dlt645Error(String),

    /// CSG1209022 解析错误
    #[error("CSG1209022 解析错误: {0}")]
    Csg1209022Error(String),

    /// CSG 本地通信解析错误
    #[error("CSG 本地通信解析错误: {0}")]
    CsgLocalCommError(String),

    /// 渲染错误
    #[error("渲染错误: {0}")]
    RenderError(String),

    /// 功能未实现
    #[error("功能未实现: {0}")]
    NotImplemented(String),
}

/// 自动检测协议类型
///
/// 根据帧格式特征判断协议类型，检测优先级：
/// 1. Q/CSG1209021 (本地通信)
/// 2. DL/T 645-2007
/// 3. Q/CSG1209022
pub fn detect_protocol(buf: &[u8]) -> Option<ProtocolType> {
    // 优先检测 CSG 本地通信（最特殊）
    if csg_local_comm::is_csg_local_comm_frame(buf) {
        return Some(ProtocolType::CsgLocalComm);
    }

    // 检测 DLT645（有明显的双 68H 特征）
    if dlt645_2007::is_dlt645_frame(buf) {
        return Some(ProtocolType::Dlt645_2007);
    }

    // 检测 CSG1209022
    if csg1209022::is_csg1209022_frame(buf) {
        return Some(ProtocolType::Csg1209022);
    }

    None
}

/// 自动解析报文
///
/// 自动检测协议类型并调用相应的解析器。
///
/// # 参数
///
/// - `buf`: 完整的帧字节
/// - `region`: 区域标识（用于 spec-engine），默认 "南网"
///
/// # 返回
///
/// 返回解析结果和消耗的字节数
pub fn auto_parse(buf: &[u8], region: Option<&str>) -> Result<(ParsedMessage, usize), ParseError> {
    let region = region.unwrap_or("南网");

    // 检测协议类型
    let protocol_type = detect_protocol(buf).ok_or(ParseError::UnknownProtocol)?;

    // 根据协议类型调用相应的解析器
    match protocol_type {
        ProtocolType::Dlt645_2007 => {
            let (msg, consumed) = dlt645_2007::decode_message(buf, "dlt645-2007", region)
                .map_err(|e| ParseError::Dlt645Error(format!("{:?}", e)))?;
            Ok((ParsedMessage::Dlt645(msg), consumed))
        }
        ProtocolType::Csg1209022 => {
            let (msg, consumed) = csg1209022::decode_message(buf, "csg13", region)
                .map_err(|e| ParseError::Csg1209022Error(format!("{:?}", e)))?;
            Ok((ParsedMessage::Csg1209022(msg), consumed))
        }
        ProtocolType::CsgLocalComm => {
            let (msg, consumed) = csg_local_comm::decode_message(buf)
                .map_err(|e| ParseError::CsgLocalCommError(format!("{:?}", e)))?;
            Ok((ParsedMessage::CsgLocalComm(msg), consumed))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_csg_local_comm() {
        // CSG 本地通信帧
        let frame = vec![
            0x68, 0x0C, 0x00, 0x80, 0x00, 0x01, 0x01, 0x00, 0x01, 0xE8, 0x6B, 0x16,
        ];
        assert_eq!(detect_protocol(&frame), Some(ProtocolType::CsgLocalComm));
    }

    #[test]
    fn test_detect_dlt645() {
        // DLT645 帧
        let frame = vec![
            0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x68, 0x11, 0x04, 0x33, 0x33, 0x34, 0x33,
            0xEB, 0x16,
        ];
        assert_eq!(detect_protocol(&frame), Some(ProtocolType::Dlt645_2007));
    }

    #[test]
    fn test_detect_unknown() {
        let frame = vec![0x00, 0x01, 0x02];
        assert_eq!(detect_protocol(&frame), None);
    }

    #[test]
    fn test_auto_parse_csg_local_comm() {
        let frame = vec![
            0x68, 0x0C, 0x00, 0x80, 0x00, 0x01, 0x01, 0x00, 0x01, 0xE8, 0x6B, 0x16,
        ];
        let result = auto_parse(&frame, None);
        assert!(result.is_ok());
        if let Ok((msg, consumed)) = result {
            assert_eq!(msg.protocol_type(), ProtocolType::CsgLocalComm);
            assert_eq!(consumed, 12);
        }
    }
}

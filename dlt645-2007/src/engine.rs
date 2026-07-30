//! DL/T 645-2007 顶层入口
//!
//! 把链路层 `Frame` 和应用层 `ApplicationLayer` 串联起来，
//! 提供完整的报文编解码功能。

use crate::app::ApplicationLayer;
use crate::error::Result;
use crate::link::Frame;
use proto_common::FieldValue;

/// 一条完整解析出来的报文：链路层帧 + 已解析的应用层内容
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    /// 链路层帧
    pub frame: Frame,
    /// 应用层内容
    pub application: ApplicationLayer,
}

impl Message {
    /// 将报文渲染成树形 FieldValue 结构，适合前端 UI 展示
    ///
    /// # 返回
    ///
    /// 根节点为 "DL/T 645-2007 报文" 的 FieldValue 树，包含所有字段的 name/raw/value 信息
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "南网")?;
    /// let tree = msg.to_value_tree()?;
    /// // 现在可以序列化 tree 发送给前端，或递归遍历展示
    /// ```
    pub fn to_value_tree(&self) -> Result<FieldValue> {
        crate::report::render_message_as_value(self)
    }
}

/// 从字节流解码一条完整报文（链路层 + 应用层），返回解析结果和消耗的字节数
///
/// # 参数
///
/// - `buf`: 包含完整帧的字节切片
/// - `protocol`: 协议标识，传给 spec-engine 做 DI 内容解析（如 "dlt645-2007"）
/// - `region`: 区域标识，传给 spec-engine（如 "国网"、"南网"）
///
/// # 返回
///
/// - `Ok((Message, consumed))`: 成功解析的报文和消耗的字节数
/// - `Err(Error)`: 解析错误
///
/// # 示例
///
/// ```rust,ignore
/// use dlt645_2007::engine::decode_message;
///
/// let bytes = vec![0x68, 0x01, 0x00, ...]; // 完整帧
/// let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "国网")?;
///
/// println!("地址: {}", msg.frame.address);
/// println!("功能码: {:?}", msg.application.function);
/// ```
pub fn decode_message(buf: &[u8], protocol: &str, region: &str) -> Result<(Message, usize)> {
    // 1. 解析链路层帧
    let (frame, consumed) = Frame::decode(buf)?;

    // 2. 解析应用层
    let application = ApplicationLayer::decode(
        &frame.payload,
        frame.control.function,
        frame.control.direction,
        frame.control.is_error(),
        protocol,
        region,
    )?;

    Ok((Message { frame, application }, consumed))
}

/// 将链路层帧和应用层内容重新编码为完整字节序列
///
/// # 参数
///
/// - `frame_without_payload`: 链路层帧（payload 会被应用层内容覆盖）
/// - `application`: 应用层内容
/// - `include_preamble`: 是否包含前导字节（4个 FE）
///
/// # 返回
///
/// 编码后的完整字节序列
///
/// # 示例
///
/// ```rust,ignore
/// use dlt645_2007::engine::encode_message;
/// use dlt645_2007::{Address, ControlCode, FunctionCode, ApplicationLayer, ApplicationBody};
/// use dlt645_2007::link::Frame;
///
/// let frame = Frame {
///     address: Address::from_decimal_str("123456789012")?,
///     control: ControlCode::master_request(FunctionCode::ReadData),
///     payload: Vec::new(), // 会被应用层内容覆盖
/// };
///
/// let application = ApplicationLayer {
///     function: FunctionCode::ReadData,
///     body: ApplicationBody::ReadRequest { identifiers: vec![...] },
/// };
///
/// let bytes = encode_message(&frame, &application, false)?;
/// ```
pub fn encode_message(
    frame_without_payload: &Frame,
    application: &ApplicationLayer,
    include_preamble: bool,
) -> Vec<u8> {
    // 1. 编码应用层
    let payload = application.encode();

    // 2. 组装链路层帧
    let mut frame = frame_without_payload.clone();
    frame.payload = payload;

    // 3. 编码链路层帧
    frame.encode(include_preamble)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address;
    use crate::app::ApplicationBody;
    use crate::control::{ControlCode, Direction, FunctionCode};
    use crate::data_identifier::DataIdentifier;

    #[test]
    fn test_decode_message_read_request() {
        // 构造一个读数据请求帧
        let mut bytes = vec![
            0x68, 0x12, 0x90, 0x78, 0x56, 0x34, 0x12, 0x68,
            0x11, 0x04, 0x33, 0x33, 0x34, 0x33, // DI: 00 01 00 00 + 33H
            0x00, 0x16, // CS和结束符（先占位）
        ];

        // 计算正确的校验和
        let len = bytes.len();
        let cs: u8 = bytes[0..len - 2].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        bytes[len - 2] = cs;

        let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

        assert_eq!(consumed, bytes.len());
        assert_eq!(msg.frame.address.to_decimal_string(), "123456789012");
        assert_eq!(msg.frame.control.function, FunctionCode::ReadData);
        assert_eq!(msg.frame.control.direction, Direction::MasterToSlave);

        assert_eq!(msg.application.function, FunctionCode::ReadData);
        match &msg.application.body {
            ApplicationBody::ReadRequest { identifiers } => {
                assert_eq!(identifiers.len(), 1);
                assert_eq!(identifiers[0].to_u32(), 0x00010000);
            }
            _ => panic!("期望 ReadRequest"),
        }
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // 构造请求
        let frame = Frame {
            address: Address::from_decimal_str("000000000001").unwrap(),
            control: ControlCode::master_request(FunctionCode::ReadData),
            payload: Vec::new(), // 会被应用层覆盖
        };

        let application = ApplicationLayer {
            function: FunctionCode::ReadData,
            body: ApplicationBody::ReadRequest {
                identifiers: vec![DataIdentifier::from_bytes([0x00, 0x00, 0x01, 0x00])],
            },
        };

        // 编码
        let bytes = encode_message(&frame, &application, false);

        // 解码
        let (decoded, consumed) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

        // 验证
        assert_eq!(consumed, bytes.len());
        assert_eq!(
            decoded.frame.address.to_decimal_string(),
            frame.address.to_decimal_string()
        );
        assert_eq!(decoded.frame.control.to_byte(), frame.control.to_byte());
        assert_eq!(decoded.application.function, application.function);
        assert_eq!(decoded.application.body, application.body);
    }

    #[test]
    fn test_encode_message_with_preamble() {
        let frame = Frame {
            address: Address::from_decimal_str("123456789012").unwrap(),
            control: ControlCode::master_request(FunctionCode::ReadData),
            payload: Vec::new(),
        };

        let application = ApplicationLayer {
            function: FunctionCode::ReadData,
            body: ApplicationBody::ReadRequest {
                identifiers: vec![DataIdentifier::from_bytes([0x00, 0x01, 0x00, 0x00])],
            },
        };

        let with_preamble = encode_message(&frame, &application, true);
        let without_preamble = encode_message(&frame, &application, false);

        // 带前导的应该多4字节
        assert_eq!(with_preamble.len(), without_preamble.len() + 4);

        // 验证前导字节
        assert_eq!(&with_preamble[0..4], &[0xFE, 0xFE, 0xFE, 0xFE]);
    }

    #[test]
    fn test_decode_broadcast_time() {
        let frame = Frame {
            address: Address::broadcast(),
            control: ControlCode::master_request(FunctionCode::BroadcastTime),
            payload: vec![0x07, 0x08, 0x14, 0x15, 0x03, 0x24], // 2024-03-15 14:08:07
        };

        let bytes = frame.encode(false);
        let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

        assert!(msg.frame.address.is_broadcast());
        assert_eq!(msg.application.function, FunctionCode::BroadcastTime);

        match &msg.application.body {
            ApplicationBody::BroadcastTime { time } => {
                assert_eq!(time.year, 0x24);
                assert_eq!(time.month, 0x03);
                assert_eq!(time.day, 0x15);
            }
            _ => panic!("期望 BroadcastTime"),
        }
    }

    #[test]
    fn test_decode_error_response() {
        // 构造错误响应
        let frame = Frame {
            address: Address::from_decimal_str("123456789012").unwrap(),
            control: ControlCode::slave_error_response(FunctionCode::ReadData),
            payload: vec![0x02], // 错误码：数据不存在
        };

        let bytes = frame.encode(false);
        let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

        assert!(msg.application.is_error());
        assert_eq!(msg.frame.control.direction, Direction::SlaveToMaster);

        match &msg.application.body {
            ApplicationBody::Error { error_bits } => {
                assert_eq!(*error_bits, 0x02);
            }
            _ => panic!("期望 Error"),
        }
    }
}

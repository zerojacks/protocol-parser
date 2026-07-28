//! DL/T 645-2007 集成测试（使用新架构）

use dlt645_2007::{
    engine::{decode_message, encode_message},
    link::Frame,
    Address, ApplicationBody, ApplicationLayer, BroadcastTimeData, ControlCode, DataIdentifier,
    Direction, FreezeTime, FunctionCode,
};

#[test]
fn test_read_request_roundtrip() {
    // 读数据请求：往返测试
    let frame = Frame {
        address: Address::from_decimal_str("123456789012").unwrap(),
        control: ControlCode::master_request(FunctionCode::ReadData),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::ReadData,
        body: ApplicationBody::ReadRequest {
            identifiers: vec![DataIdentifier::from_bytes([0x00, 0x00, 0x00, 0x00])],
        },
    };

    let bytes = encode_message(&frame, &application, false);
    let (msg, consumed) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

    assert_eq!(consumed, bytes.len());
    assert_eq!(msg.frame.address.to_decimal_string(), "123456789012");
    assert_eq!(msg.frame.control.direction, Direction::MasterToSlave);
    assert_eq!(msg.frame.control.function, FunctionCode::ReadData);

    if let ApplicationBody::ReadRequest { identifiers } = &msg.application.body {
        assert_eq!(identifiers.len(), 1);
        assert_eq!(identifiers[0].to_u32(), 0x00000000);
    } else {
        panic!("期望 ReadRequest");
    }
}

#[test]
fn test_broadcast_time() {
    // 广播校时测试
    let frame = Frame {
        address: Address::broadcast(),
        control: ControlCode::master_request(FunctionCode::BroadcastTime),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::BroadcastTime,
        body: ApplicationBody::BroadcastTime {
            time: BroadcastTimeData {
                second: 0x07, // BCD: 07
                minute: 0x08, // BCD: 08
                hour: 0x14,   // BCD: 14 (表示14时)
                day: 0x15,    // BCD: 15
                month: 0x03,  // BCD: 03
                year: 0x24,   // BCD: 24
            },
        },
    };

    let bytes = encode_message(&frame, &application, false);
    let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

    assert!(msg.frame.address.is_broadcast());
    assert_eq!(msg.frame.control.function, FunctionCode::BroadcastTime);

    if let ApplicationBody::BroadcastTime { time } = &msg.application.body {
        assert_eq!(time.second, 0x07);
        assert_eq!(time.minute, 0x08);
        assert_eq!(time.hour, 0x14);
        assert_eq!(time.day, 0x15);
        assert_eq!(time.month, 0x03);
        assert_eq!(time.year, 0x24);
    } else {
        panic!("期望 BroadcastTime");
    }
}

#[test]
fn test_write_response_success() {
    // 写数据成功应答
    let frame = Frame {
        address: Address::from_decimal_str("112233445566").unwrap(),
        control: ControlCode::slave_response(FunctionCode::WriteData, false),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::WriteData,
        body: ApplicationBody::WriteResponse,
    };

    let bytes = encode_message(&frame, &application, false);
    let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

    assert_eq!(msg.frame.address.to_decimal_string(), "112233445566");
    assert_eq!(msg.frame.control.direction, Direction::SlaveToMaster);
    assert_eq!(msg.frame.control.function, FunctionCode::WriteData);
    assert!(matches!(
        msg.application.body,
        ApplicationBody::Empty | ApplicationBody::WriteResponse
    ));
}

#[test]
fn test_frame_with_preamble() {
    // 测试带前导字节的帧
    let frame = Frame {
        address: Address::from_decimal_str("000000000001").unwrap(),
        control: ControlCode::master_request(FunctionCode::ReadData),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::ReadData,
        body: ApplicationBody::ReadRequest {
            identifiers: vec![DataIdentifier::from_bytes([0x00, 0x01, 0x00, 0x00])],
        },
    };

    let bytes_with_preamble = encode_message(&frame, &application, true);
    let bytes_without_preamble = encode_message(&frame, &application, false);

    // 带前导字节的应该比不带的多4字节
    assert_eq!(
        bytes_with_preamble.len(),
        bytes_without_preamble.len() + 4
    );

    // 两种方式都应该能解析
    let (msg1, _) =
        decode_message(&bytes_with_preamble, "dlt645-2007", "国网").expect("带前导的解析失败");
    let (msg2, _) = decode_message(&bytes_without_preamble, "dlt645-2007", "国网")
        .expect("不带前导的解析失败");

    assert_eq!(
        msg1.frame.address.to_decimal_string(),
        msg2.frame.address.to_decimal_string()
    );
    assert_eq!(msg1.frame.control.to_byte(), msg2.frame.control.to_byte());

    // 验证解码后数据一致
    if let (
        ApplicationBody::ReadRequest { identifiers: ids1 },
        ApplicationBody::ReadRequest { identifiers: ids2 },
    ) = (&msg1.application.body, &msg2.application.body)
    {
        assert_eq!(ids1.len(), ids2.len());
        assert_eq!(ids1[0].to_u32(), ids2[0].to_u32());
    } else {
        panic!("期望两个都是 ReadRequest");
    }
}

#[test]
fn test_address_types() {
    // 测试不同类型的地址

    // 普通地址
    let addr1 = Address::from_decimal_str("123456789012").unwrap();
    assert_eq!(addr1.to_decimal_string(), "123456789012");
    assert!(!addr1.is_broadcast());

    // 广播地址
    let addr2 = Address::broadcast();
    assert_eq!(addr2.to_decimal_string(), "999999999999");
    assert!(addr2.is_broadcast());

    // 前导零地址
    let addr3 = Address::from_decimal_str("000000000001").unwrap();
    assert_eq!(addr3.to_decimal_string(), "000000000001");
}

#[test]
fn test_data_identifier() {
    // 测试数据标识
    let di = DataIdentifier::from_bytes([0x00, 0x00, 0x01, 0x00]);
    assert_eq!(di.to_u32(), 0x00010000);
    assert_eq!(di.as_bytes(), &[0x00, 0x00, 0x01, 0x00]);

    // 另一个数据标识
    let di2 = DataIdentifier::from_bytes([0x00, 0x02, 0x01, 0x02]);
    assert_eq!(di2.to_u32(), 0x02010200);
}

#[test]
fn test_control_code() {
    // 主站请求
    let cc1 = ControlCode::master_request(FunctionCode::ReadData);
    assert_eq!(cc1.direction, Direction::MasterToSlave);
    assert_eq!(cc1.function, FunctionCode::ReadData);
    assert!(!cc1.has_follow_up);

    // 从站应答
    let cc2 = ControlCode::slave_response(FunctionCode::ReadData, false);
    assert_eq!(cc2.direction, Direction::SlaveToMaster);
    assert_eq!(cc2.function, FunctionCode::ReadData);
    assert!(!cc2.has_follow_up);

    // 从站错误应答
    let cc3 = ControlCode::slave_error_response(FunctionCode::ReadData);
    assert_eq!(cc3.direction, Direction::SlaveToMaster);
    assert_eq!(cc3.function, FunctionCode::ReadData);
    assert!(cc3.is_error());
}

#[test]
fn test_freeze_command() {
    // 冻结命令
    let frame = Frame {
        address: Address::from_decimal_str("100000000001").unwrap(),
        control: ControlCode::master_request(FunctionCode::Freeze),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::Freeze,
        body: ApplicationBody::Freeze {
            freeze_time: FreezeTime {
                minute: 0x00,
                hour: 0x00,
                day: 0x01,
                month: 0x01,
            },
        },
    };

    let bytes = encode_message(&frame, &application, false);
    let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

    assert_eq!(msg.frame.control.function, FunctionCode::Freeze);
    if let ApplicationBody::Freeze { freeze_time } = &msg.application.body {
        assert_eq!(freeze_time.day, 0x01);
        assert_eq!(freeze_time.month, 0x01);
    } else {
        panic!("期望 Freeze");
    }
}

#[test]
fn test_empty_data() {
    // 空数据测试（如某些异常应答）
    let frame = Frame {
        address: Address::from_decimal_str("123456789012").unwrap(),
        control: ControlCode::slave_response(FunctionCode::WriteData, false),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::WriteData,
        body: ApplicationBody::Empty,
    };

    let bytes = encode_message(&frame, &application, false);
    let (msg, _) = decode_message(&bytes, "dlt645-2007", "国网").unwrap();

    assert!(matches!(msg.application.body, ApplicationBody::Empty));
}

#[test]
fn test_invalid_frame_start() {
    // 测试无效的起始符
    let invalid = vec![
        0x67, // 错误的起始符
        0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x68, 0x11, 0x00, 0x00, 0x16,
    ];

    assert!(decode_message(&invalid, "dlt645-2007", "国网").is_err());
}

#[test]
fn test_insufficient_length() {
    // 测试长度不足的帧
    let short = vec![0x68, 0x12, 0x34];
    assert!(decode_message(&short, "dlt645-2007", "国网").is_err());
}

//! 简单帧解析示例（使用新的 engine 接口）
//!
//! 运行方式：
//! ```bash
//! cargo run --example simple_parse
//! ```

use dlt645_2007::{
    engine::{decode_message, encode_message},
    link::Frame,
    Address, ApplicationBody, ApplicationLayer, BroadcastTimeData, ControlCode, DataIdentifier,
    FunctionCode,
};

fn main() {
    println!("=== DL/T 645-2007 简单帧解析示例 ===\n");

    // 示例1：读数据请求
    println!("--- 示例1：构造并编码读数据请求 ---");
    let frame = Frame {
        address: Address::from_decimal_str("123456789012").unwrap(),
        control: ControlCode::master_request(FunctionCode::ReadData),
        payload: Vec::new(), // 会被应用层覆盖
    };

    let application = ApplicationLayer {
        function: FunctionCode::ReadData,
        body: ApplicationBody::ReadRequest {
            identifiers: vec![DataIdentifier::from_bytes([0x00, 0x00, 0x00, 0x00])],
        },
    };

    let bytes = encode_message(&frame, &application, false);
    println!("编码结果: {}", format_hex(&bytes));
    println!("字节数: {}", bytes.len());

    // 解码验证
    match decode_message(&bytes, "dlt645-2007", "国网") {
        Ok((msg, consumed)) => {
            println!("✓ 解码成功 (消耗 {} 字节)", consumed);
            println!("  地址: {}", msg.frame.address.to_decimal_string());
            println!("  功能码: {:?}", msg.application.function);
        }
        Err(e) => println!("✗ 解码失败: {}", e),
    }

    println!();

    // 示例2：广播校时
    println!("--- 示例2：广播校时 ---");
    let time_frame = Frame {
        address: Address::broadcast(),
        control: ControlCode::master_request(FunctionCode::BroadcastTime),
        payload: Vec::new(),
    };

    let time_app = ApplicationLayer {
        function: FunctionCode::BroadcastTime,
        body: ApplicationBody::BroadcastTime {
            time: BroadcastTimeData {
                second: 0x07, // 07秒（BCD）
                minute: 0x08, // 08分
                hour: 0x14,   // 14时（20时，BCD: 2*10+0=20）
                day: 0x15,    // 15日（21日，BCD: 2*10+1=21）
                month: 0x03,  // 03月
                year: 0x24,   // 24年（2024）
            },
        },
    };

    let bytes = encode_message(&time_frame, &time_app, true); // 包含前导字节
    println!("编码结果 (含前导): {}", format_hex(&bytes));
    println!("字节数: {}", bytes.len());

    // 解码验证
    match decode_message(&bytes, "dlt645-2007", "国网") {
        Ok((msg, _)) => {
            println!("✓ 解码成功");
            println!(
                "  地址: {} ({})",
                msg.frame.address.to_decimal_string(),
                if msg.frame.address.is_broadcast() {
                    "广播"
                } else {
                    "单播"
                }
            );
            if let ApplicationBody::BroadcastTime { time } = &msg.application.body {
                println!(
                    "  时间: 20{:02X}-{:02X}-{:02X} {:02X}:{:02X}:{:02X}",
                    time.year, time.month, time.day, time.hour, time.minute, time.second
                );
            }
        }
        Err(e) => println!("✗ 解码失败: {}", e),
    }

    println!("\n=== 完成 ===");
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

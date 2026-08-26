//! 自动协议检测和解析示例

use std::print;

use protocol_parser::{auto_parse, detect_protocol};

fn main() {
    println!("=== 多协议自动解析示例 ===\n");

    // 示例1: CSG 本地通信帧
    println!("【示例 1】CSG 本地通信帧");
    let csg_local_frame = vec![
        0x68, 0x0C, 0x00, 0x80, // 起始符 + 长度 + 控制字
        0x00, 0x01, // AFN + SEQ
        0x01, 0x00, 0x01, 0xE8, // DI (小端序)
        0x6B, 0x16, // 校验和 + 结束符
    ];

    parse_and_print(&csg_local_frame);

    // 示例2: DLT645-2007 帧
    println!("\n【示例 2】DLT645-2007 帧");
    let dlt645_frame = vec![
        0x68, // 起始符
        0x12, 0x90, 0x78, 0x56, 0x34, 0x12, // 地址域
        0x68, // 第二个起始符
        0x11, // 控制码
        0x04, // 数据长度
        0x33, 0x33, 0x34, 0x33, // 数据域（+33H后）
        0xEB, // 校验和
        0x16, // 结束符
    ];

    parse_and_print(&dlt645_frame);

    // 示例3: 用户提供的真实报文
    println!("\n【示例 3】用户真实报文");
    let user_frame_hex = "68 49 00 40 04 11 02 04 02 E8 0A 18 39 36 00 19 00 50 39 36 00 19 00 07 09 37 00 19 00 85 31 39 00 19 00 35 24 45 00 20 00 48 24 45 00 20 00 27 52 46 00 20 00 24 56 46 00 20 00 18 58 46 00 20 00 26 77 46 00 20 00 56 16";
    let user_frame: Vec<u8> = user_frame_hex
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();

    parse_and_print(&user_frame);

    // 示例4: CSG1209022 上行通信帧
    println!("\n【示例 4】CSG1209022 上行通信帧");
    let frame_hex = "68 3A 00 3A 00 68 4B FF FF FF FF FF FF 0A 0F 60 00 00 01 00 01 E3 00 00 F9 04 40 7A 11 00 54 43 45 5F 44 4A 43 39 35 30 5F 32 36 30 34 32 39 5F 31 36 33 30 5F 56 31 2E 30 31 28 65 78 74 54 E8 FE 16"; 
    let csg1209022_frame: Vec<u8> = frame_hex
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();

    parse_and_print(&csg1209022_frame);

    // 示例5: 未知协议
    println!("\n【示例 5】未知协议");
    let unknown_frame = vec![0x00, 0x01, 0x02, 0x03];
    parse_and_print(&unknown_frame);
}

fn parse_and_print(buf: &[u8]) {
    println!("原始字节 ({} bytes): {:02X?}", buf.len(), &buf[..buf.len().min(16)]);
    if buf.len() > 16 {
        println!("           ... (共 {} 字节)", buf.len());
    }

    // 检测协议类型
    print!("协议检测: ");
    match detect_protocol(buf) {
        Some(protocol_type) => {
            println!("✓ 识别为 {}", protocol_type.name());

            // 自动解析
            match auto_parse(buf, None) {
                Ok((parsed_msg, consumed)) => {
                    println!("解析结果: ✓ 成功");
                    println!("  - 消耗字节数: {}", consumed);
                    println!("  - 协议名称: {}", parsed_msg.protocol_name());

                    // 打印详细信息
                    match parsed_msg {
                        protocol_parser::ParsedMessage::CsgLocalComm(msg) => {
                            println!("  - AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
                            println!("  - SEQ: {}", msg.app.seq);
                            println!("  - DI: {:08X}H", msg.app.di.to_u32());
                            println!("  - 控制字: {:?}", msg.frame.control);
                        }
                        protocol_parser::ParsedMessage::Dlt645(msg) => {
                            println!("  - 地址: {:?}", msg.frame.address);
                            println!("  - 控制码: {:?}", msg.frame.control);
                            println!("  - 功能码: {:?}", msg.application.function);
                        }
                        protocol_parser::ParsedMessage::Csg1209022(msg) => {
                            println!("  - 控制域: {:?}", msg.frame.control);
                            println!("  - 应用层: {:?}", msg.application);
                        }
                    }
                }
                Err(e) => {
                    println!("解析失败: ✗ {}", e);
                }
            }
        }
        None => {
            println!("✗ 无法识别协议");
        }
    }
    println!("{}", "-".repeat(60));
}

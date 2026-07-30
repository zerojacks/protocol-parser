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
    let frame_hex = "68 96 02 96 02 68 4B FF FF FF FF FF FF 0A 0F 6D 00 00 02 00 01 E3 F8 04 80 02 B0 3A 18 AB BE 81 E1 43 5E 7E 5E 85 39 17 2B 15 B6 70 88 42 6F 05 98 22 71 3D 1C E7 A3 D2 CE 7E 22 5F 7D A0 AB 75 E0 DB 62 A1 AF E7 5B 73 93 D0 9D 65 16 33 D4 DF 33 7D FB 2F 4E D9 0C 07 23 D4 FC 42 36 93 A7 20 6F 91 63 19 18 6F F5 F7 5B 59 A8 93 B4 8F 52 00 A1 6E 07 8F 58 32 97 05 74 D3 2A FE CC B4 4B 54 0F D1 8C AB 41 2F D8 85 3C 4C 53 6C 32 15 2A 43 18 61 B1 BE 37 CC E9 B5 B7 8D 76 4A 52 8A CD 71 B7 EE 79 82 07 BE 0E 1F F9 2A B9 A7 50 97 40 8C EE 03 F6 49 AC 17 8E 09 CD 68 5E AE A4 6E 20 B7 95 C9 93 3E A9 80 2F 4E DC 7A 4D 11 3A 2A 0E 52 48 D7 09 E1 57 F4 69 AD EE 29 DD FF BD C6 D7 AD FB E9 54 86 30 43 61 E7 02 52 28 58 2D 55 D5 A7 BD BC 20 C0 D0 C1 E6 68 58 D9 BC 33 6B 25 E7 B7 B9 22 61 8A BD 34 49 6B 81 DE 5E 45 B2 F6 E7 48 74 0E 4F 43 7B 31 E5 B3 A6 1B B4 09 08 22 27 F4 C8 50 16 12 C8 DE 27 2F 58 39 20 45 6C D2 A3 6E 46 D3 E8 71 64 B3 7C F7 D2 BB 45 20 F0 BA 4A 90 32 15 34 E8 BA 10 EC C2 78 E5 29 E6 3B BC DE DC 5C 8B 17 A6 9F 0E 09 72 B9 5D E6 AB 31 1D 90 97 28 FA 25 2C 54 98 1C 75 AB 31 84 73 3E 36 E7 DA 69 ED 06 57 E2 9F 81 4C A2 32 8E A4 A6 78 13 5F 85 3F 25 C6 41 30 F5 F1 FB AB BF BC 37 19 42 F8 0A 14 FC 45 91 2E 21 40 76 5C D1 EF DB B8 FF 9C C2 01 D6 E5 9B A6 11 9E 77 C2 23 12 5B DB E2 3B 91 B9 F5 FB C8 72 16 8A 5E 81 52 AE 6B 8C 9E 50 D9 2B 42 59 B9 8F 42 6B 3A 1A 32 01 B2 1B 0C B5 2E 32 A0 1C ED A5 48 76 15 0D 94 2F 83 5F 58 97 17 8C 42 62 E5 AB 02 73 2B 68 E9 D4 9E C2 F0 CB A8 11 A8 12 F5 DD C5 ED 35 97 AF 61 E9 FD E9 1A 9D 86 5B 35 51 87 7C 04 F7 71 2A 9F ED 4A 89 61 C6 79 53 43 BE 8A 91 F1 61 3B 3E E2 65 71 BF 42 38 73 1C C2 A6 A2 20 40 C1 7B BA 92 55 F7 00 35 AB 72 A4 2F 11 86 6A 77 BF 3B DD 2B 23 62 D4 D5 D6 1B C1 29 6E 75 6F 7F C9 78 6B B6 E4 6F 97 EF 7D 07 F6 05 CB DC 42 C4 D0 40 4C CE 06 E9 73 0D 50 5B 5A 16 1D ED D6 5B B8 EF D5 8B C7 05 36 E6 F7 B9 9B B5 B9 C9 4B 38 BD 47 98 5D 31 54 D2 8D D1 6A C8 FB 1D 29 B9 5A 4B 2F D7 05 64 36 E0 6C B1 E9 8D 3A 3A CD 09 B2 16 14 8A 89 A5 16";
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

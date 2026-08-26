//! 解析用户提供的 Q/CSG1209021-2019 协议帧

use csg_local_comm::{decode_message, report};

fn main() {
    // 用户提供的报文
    let hex_str = "68 49 00 40 04 11 02 04 02 E8 0A 18 39 36 00 19 00 50 39 36 00 19 00 07 09 37 00 19 00 85 31 39 00 19 00 35 24 45 00 20 00 48 24 45 00 20 00 27 52 46 00 20 00 24 56 46 00 20 00 18 58 46 00 20 00 26 77 46 00 20 00 56 16";
    
    // 解析十六进制字符串为字节数组
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();

    println!("=== Q/CSG1209021-2019 协议报文解析 ===\n");
    println!("原始字节 ({} bytes):", bytes.len());
    print_hex_dump(&bytes);

    println!("\n=== 开始解析 ===\n");

    match decode_message(&bytes) {
        Ok((msg, consumed)) => {
            println!("✓ 成功解析帧，消耗字节数: {}", consumed);
            println!("\n【链路层信息】");
            println!("  控制字: {:?}", msg.frame.control);
            println!("    - 方向: {:?}", msg.frame.control.direction);
            println!("    - 启动站: {}", msg.frame.control.is_primary);
            println!("    - 包含地址域: {}", msg.frame.control.has_address);
            
            if let Some(ref addr) = msg.frame.address {
                println!("  地址域:");
                println!("    - 源地址: {:02X?}", addr.source);
                println!("    - 目标地址: {:02X?}", addr.destination);
            }

            println!("\n【应用层信息】");
            println!("  AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
            println!("  SEQ: {}", msg.app.seq);
            println!("  DI: {:08X}H", msg.app.di.to_u32());
            println!("    - 节点角色: {:?}", msg.app.di.node_role);
            println!("    - 消息方向: {:?}", msg.app.di.direction);
            println!("    - AFN匹配: {:02X}H", msg.app.di.afn_match);
            println!("    - 子功能码: {:02X}H", msg.app.di.sub_function);

            println!("\n【数据内容】");
            match &msg.app.body {
                csg_local_comm::ApplicationBody::Empty => {
                    println!("  类型: 空数据");
                }
                csg_local_comm::ApplicationBody::Ack => {
                    println!("  类型: 确认 (ACK)");
                }
                csg_local_comm::ApplicationBody::Nack { error_code } => {
                    println!("  类型: 否认 (NACK)");
                    println!("  错误码: 0x{:02X}", error_code);
                }
                csg_local_comm::ApplicationBody::WithData { raw_data, parsed_value } => {
                    println!("  类型: 带数据内容");
                    println!("  原始数据 ({} bytes):", raw_data.len());
                    print_hex_dump(raw_data);
                    
                    if let Some(ref pv) = parsed_value {
                        println!("\n  spec-engine 解析结果:");
                        print_field_value(pv, 2);
                    } else {
                        println!("\n  (spec-engine 未解析)");
                    }
                }
            }

            // 渲染为树形结构
            println!("\n=== 树形展示 ===\n");
            if let Ok(tree) = report::render_message_as_value(&msg, "csg13", "南网") {
                print_value_tree(&tree, 0);
            }
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {:?}", e);
            eprintln!("\n可能的原因:");
            eprintln!("  - 帧格式错误");
            eprintln!("  - 校验和不匹配");
            eprintln!("  - 数据长度不足");
        }
    }
}

/// 打印十六进制转储
fn print_hex_dump(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("  {:04X}  ", i * 16);
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02X} ", byte);
            if j == 7 {
                print!(" ");
            }
        }
        // 填充空格
        for _ in chunk.len()..16 {
            print!("   ");
            if chunk.len() <= 8 {
                print!(" ");
            }
        }
        print!(" |");
        for byte in chunk {
            let c = if *byte >= 0x20 && *byte < 0x7F {
                *byte as char
            } else {
                '.'
            };
            print!("{}", c);
        }
        println!("|");
    }
}

/// 递归打印 FieldValue 树
fn print_value_tree(value: &proto_common::FieldValue, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        proto_common::FieldValue::Node { name, raw, value } => {
            if !raw.is_empty() {
                println!("{}{}: [{:02X?}]", prefix, name, raw);
            } else {
                println!("{}{}", prefix, name);
            }
            print_value_tree(value, indent + 1);
        }
        proto_common::FieldValue::List(items) => {
            for item in items {
                print_value_tree(item, indent);
            }
        }
        proto_common::FieldValue::Str(s) => {
            println!("{}{}", prefix, s);
        }
        proto_common::FieldValue::Int(i) => {
            println!("{}{}", prefix, i);
        }
        proto_common::FieldValue::Float(f) => {
            println!("{}{}", prefix, f);
        }
        proto_common::FieldValue::Bytes(b) => {
            println!("{}{:02X?}", prefix, b);
        }
        proto_common::FieldValue::Map(entries) => {
            for (key, val) in entries {
                println!("{}{}:", prefix, key);
                print_value_tree(val, indent + 1);
            }
        }
        proto_common::FieldValue::WithUnit { value, unit } => {
            print_value_tree(value, indent);
            println!("{} ({})", prefix, unit);
        }
        proto_common::FieldValue::Bit { bit_start, bit_end, bit_value, .. } => {
            println!("{}Bit[{}..{}] = {}", prefix, bit_start, bit_end, bit_value);
        }
        proto_common::FieldValue::Pn(pn) => {
            println!("{}Pn测量点号: {}", prefix, pn);
        }
        proto_common::FieldValue::Skip => {
            println!("{}(跳过)", prefix);
        }
        proto_common::FieldValue::Invalid { reason } => {
            println!("{}✗ 无效: {}", prefix, reason);
        }
    }
}

/// 打印 FieldValue（简化版）
fn print_field_value(value: &proto_common::FieldValue, indent: usize) {
    let prefix = "    ".repeat(indent);

    match value {
        proto_common::FieldValue::Int(i) => println!("{}整数: {}", prefix, i),
        proto_common::FieldValue::Float(f) => println!("{}浮点: {}", prefix, f),
        proto_common::FieldValue::Str(s) => println!("{}字符串: {}", prefix, s),
        proto_common::FieldValue::Bytes(b) => println!("{}字节: {:02X?}", prefix, b),
        proto_common::FieldValue::Invalid { reason } => println!("{}无效: {}", prefix, reason),
        proto_common::FieldValue::Pn(pn) => println!("{}测量点号: {}", prefix, pn),
        proto_common::FieldValue::Node { name, value, .. } => {
            println!("{}节点: {}", prefix, name);
            print_field_value(value, indent + 1);
        }
        proto_common::FieldValue::List(items) => {
            println!("{}列表 ({} 项):", prefix, items.len());
            for item in items {
                print_field_value(item, indent + 1);
            }
        }
        _ => println!("{}{:?}", prefix, value),
    }
}

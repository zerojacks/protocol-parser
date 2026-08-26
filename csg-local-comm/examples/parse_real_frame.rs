//! 解析真实的 Q/CSG1209021-2019 协议帧
//!
//! 本示例演示如何使用 csg-local-comm 库解析一条完整的协议帧。

use csg_local_comm::{decode_message, report};

fn main() {
    // 示例 1: 下行查询参数帧（带地址域）
    // 帧结构：68H | L(2B) | C | A(12B) | AFN | SEQ | DI(4B) | CS | 16H
    println!("=== 示例 1: 下行查询参数帧 ===");
    
    // 先构建帧内容（不含起始符、长度、校验和、结束符）
    let control = 0x60u8; // C: 下行, 启动站, 有地址域
    let address = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16]; // 12字节地址域
    let payload = [0x03, 0x01, 0x01, 0x03, 0x01, 0xE8]; // AFN + SEQ + DI(小端序：DI0 DI1 DI2 DI3)
    
    // 计算帧长度：68(1) + L(2) + C(1) + A(12) + payload(6) + CS(1) + 16(1) = 24
    let frame_len = 24u16;
    
    let mut query_frame = vec![0x68]; // 起始符
    query_frame.extend_from_slice(&frame_len.to_le_bytes()); // L
    query_frame.push(control); // C
    query_frame.extend_from_slice(&address); // A
    query_frame.extend_from_slice(&payload); // 用户数据区
    
    // 计算校验和：C + A + payload
    let cs_data = &query_frame[3..query_frame.len()]; // 从 C 开始
    let cs: u8 = cs_data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    query_frame.push(cs); // CS
    query_frame.push(0x16); // 结束符

    println!("生成的帧字节: {:02X?}", query_frame);
    
    match decode_message(&query_frame) {
        Ok((msg, consumed)) => {
            println!("✓ 成功解析帧，消耗字节数: {}", consumed);
            println!("  控制字: {:?}", msg.frame.control);
            println!("  AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
            println!("  SEQ: {}", msg.app.seq);
            println!("  DI: {:08X}H", msg.app.di.to_u32());
            println!("  数据体: {:?}", msg.app.body);

            // 渲染为树形结构
            if let Ok(tree) = report::render_message_as_value(&msg, "csg13", "南网") {
                println!("\n树形展示:");
                print_value_tree(&tree, 0);
            }
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {:?}", e);
        }
    }

    println!("\n=== 示例 2: 上行确认帧（无地址域）===");
    // 帧结构：68H | L(2B) | C | AFN | SEQ | DI(4B) | CS | 16H
    // 帧长度：68(1) + L(2) + C(1) + payload(6) + CS(1) + 16(1) = 12
    
    let control2 = 0x80u8; // C: 上行, 从动站, 无地址域
    let payload2 = [0x00, 0x01, 0xE8, 0x01, 0x00, 0x01]; // AFN + SEQ + DI
    let frame_len2 = 12u16;
    
    let mut ack_frame = vec![0x68];
    ack_frame.extend_from_slice(&frame_len2.to_le_bytes());
    ack_frame.push(control2);
    ack_frame.extend_from_slice(&payload2);
    
    let cs2_data = &ack_frame[3..];
    let cs2: u8 = cs2_data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    ack_frame.push(cs2);
    ack_frame.push(0x16);

    println!("生成的帧字节: {:02X?}", ack_frame);

    match decode_message(&ack_frame) {
        Ok((msg, consumed)) => {
            println!("✓ 成功解析帧，消耗字节数: {}", consumed);
            println!("  控制字: {:?}", msg.frame.control);
            println!("  AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
            println!("  SEQ: {}", msg.app.seq);
            println!("  DI: {:08X}H", msg.app.di.to_u32());
            println!("  数据体: {:?}", msg.app.body);

            if let Ok(tree) = report::render_message_as_value(&msg, "csg13", "南网") {
                println!("\n树形展示:");
                print_value_tree(&tree, 0);
            }
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {:?}", e);
        }
    }

    println!("\n=== 示例 3: 上行否认帧 ===");
    // 帧长度：68(1) + L(2) + C(1) + payload(7) + CS(1) + 16(1) = 13
    let control3 = 0x80u8;
    let payload3 = [0x00, 0x02, 0xE8, 0x01, 0x00, 0x02, 0x05]; // AFN + SEQ + DI + 错误码
    let frame_len3 = 13u16;
    
    let mut nack_frame = vec![0x68];
    nack_frame.extend_from_slice(&frame_len3.to_le_bytes());
    nack_frame.push(control3);
    nack_frame.extend_from_slice(&payload3);
    
    let cs3_data = &nack_frame[3..];
    let cs3: u8 = cs3_data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));
    nack_frame.push(cs3);
    nack_frame.push(0x16);

    println!("生成的帧字节: {:02X?}", nack_frame);

    match decode_message(&nack_frame) {
        Ok((msg, consumed)) => {
            println!("✓ 成功解析帧，消耗字节数: {}", consumed);
            println!("  控制字: {:?}", msg.frame.control);
            println!("  AFN: {:02X}H - {}", msg.app.afn.to_byte(), msg.app.afn.description());
            println!("  SEQ: {}", msg.app.seq);
            println!("  DI: {:08X}H", msg.app.di.to_u32());
            println!("  数据体: {:?}", msg.app.body);
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {:?}", e);
        }
    }
}

/// 递归打印 FieldValue 树
fn print_value_tree(value: &proto_common::FieldValue, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        proto_common::FieldValue::Node { name, raw, value } => {
            println!("{}{}: [{:02X?}]", prefix, name, raw);
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
        proto_common::FieldValue::Bytes(b) => {
            println!("{}{:02X?}", prefix, b);
        }
        _ => {
            println!("{}{:?}", prefix, value);
        }
    }
}

//! 解析真实报文示例（使用 Message::to_value_tree() 方法）
//!
//! 运行方式：
//! ```bash
//! cargo run --example parse_real_frame
//! ```

use dlt645_2007::decode_message;

fn main() {
    println!("=== DL/T 645-2007 真实报文解析（树形展示）===\n");

    // 真实报文
    let hex_str = "68 01 00 00 00 00 00 68 91 12 32 38 33 37 33 34 33 35 33 36 33 37 33 38 33 39 33 3A 2E 16";
    
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();

    println!("原始报文 ({} 字节):", bytes.len());
    for chunk in bytes.chunks(16) {
        println!("  {}", chunk.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
    }
    println!();

    // 解析报文
    match decode_message(&bytes, "dlt645-2007", "南网") {
        Ok((msg, consumed)) => {
            println!("✓ 解析成功 (消耗 {} 字节)\n", consumed);
            
            // 使用 Message::to_value_tree() 方法生成树形结构
            match msg.to_value_tree(&bytes[..consumed]) {
                Ok(tree) => {
                    print_value_tree(&tree, 0);
                }
                Err(e) => {
                    println!("✗ 渲染失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ 解析失败: {}", e);
        }
    }

    println!("\n=== 完成 ===");
}

/// 递归打印 FieldValue 树，模拟前端 UI 的三列表格显示
fn print_value_tree(value: &dlt645_2007::FieldValue, indent: usize) {
    use dlt645_2007::FieldValue;
    
    let prefix = "  ".repeat(indent);
    
    match value {
        FieldValue::Node { name, raw, value } => {
            // 打印节点：name | raw | value
            let raw_str = if raw.is_empty() {
                String::new()
            } else if raw.len() <= 8 {
                format!("[{}]", raw.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "))
            } else {
                format!("[{} 字节]", raw.len())
            };
            
            println!("{}【{}】 {} ", prefix, name, raw_str);
            
            // 递归打印子节点
            print_value_tree(value, indent + 1);
        }
        FieldValue::List(items) => {
            for item in items {
                print_value_tree(item, indent);
            }
        }
        FieldValue::Map(entries) => {
            for (key, val) in entries {
                println!("{}  {}: ", prefix, key);
                print_value_tree(val, indent + 1);
            }
        }
        FieldValue::Str(s) => {
            println!("{}  {}", prefix, s);
        }
        FieldValue::Int(i) => {
            println!("{}  {}", prefix, i);
        }
        FieldValue::Float(f) => {
            println!("{}  {}", prefix, f);
        }
        FieldValue::Bytes(b) => {
            println!("{}  {:02X?}", prefix, b);
        }
        FieldValue::WithUnit { value, unit } => {
            print_value_tree(value, indent);
            println!("{}  单位: {}", prefix, unit);
        }
        FieldValue::Bit { bit_start, bit_end, bit_value, bit_byte, value: bit_val } => {
            println!("{}  BIT({}-{})={}, 字节={:02X?}", 
                     prefix, bit_start, bit_end, bit_value, bit_byte);
            if let Some(val) = bit_val {
                print_value_tree(val, indent + 1);
            }
        }
        FieldValue::Invalid { reason } => {
            println!("{}  ✗ 无效: {}", prefix, reason);
        }
        FieldValue::Skip => {
            println!("{}  (跳过)", prefix);
        }
        FieldValue::Pn(pn) => {
            println!("{}  测量点号: {}", prefix, pn);
        }
    }
}

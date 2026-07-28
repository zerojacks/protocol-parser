use csg1209022::link::Frame;

fn main() {
    // 68 CD 00 CD 00 68 C4 FF FF FF FF FF FF 00 12 70 00 00 0D 03 00 E0 00 01 0C 00 00 01 00 01 00 61 00 00 00 26 07 01 00 00 00 00 01 01 01 00 19 00 00 00 26 07 01 00 00 00 00 01 02 01 00 41 00 00 00 26 07 01 00 00 00 00 01 03 01 00 00 00 00 00 26 07 01 00 00 00 00 01 04 01 00 00 00 00 00 26 07 01 00 00 00 00 01 00 03 00 21 00 00 00 26 07 01 00 00 00 00 01 00 02 00 00 00 00 00 26 07 01 00 00 00 00 01 01 02 00 00 00 00 00 26 07 01 00 00 00 00 01 02 02 00 00 00 00 00 26 07 01 00 00 00 00 01 03 02 00 00 00 00 00 26 07 01 00 00 00 00 01 04 02 00 00 00 00 00 26 07 01 00 00 00 00 01 00 04 00 05 00 00 00 26 07 01 00 00 20 26 07 01 00 00 CA 16
    let hex_str = "68 CD 00 CD 00 68 C4 FF FF FF FF FF FF 00 12 70 00 00 0D 03 00 E0 00 01 0C 00 00 01 00 01 00 61 00 00 00 26 07 01 00 00 00 00 01 01 01 00 19 00 00 00 26 07 01 00 00 00 00 01 02 01 00 41 00 00 00 26 07 01 00 00 00 00 01 03 01 00 00 00 00 00 26 07 01 00 00 00 00 01 04 01 00 00 00 00 00 26 07 01 00 00 00 00 01 00 03 00 21 00 00 00 26 07 01 00 00 00 00 01 00 02 00 00 00 00 00 26 07 01 00 00 00 00 01 01 02 00 00 00 00 00 26 07 01 00 00 00 00 01 02 02 00 00 00 00 00 26 07 01 00 00 00 00 01 03 02 00 00 00 00 00 26 07 01 00 00 00 00 01 04 02 00 00 00 00 00 26 07 01 00 00 00 00 01 00 04 00 05 00 00 00 26 07 01 00 00 20 26 07 01 00 00 CA 16";
    
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();
    
    println!("原始报文长度: {} 字节", bytes.len());
    println!("原始报文(前50字节): {:02X?}\n", &bytes[..50.min(bytes.len())]);
    
    // 由于包含广播地址(FF FF FF)，直接解析应用层部分
    // 跳过: 68(1) + L(2) + L(2) + 68(1) + C(1) + A(7) = 14字节
    let app_start = 14;
    let app_end = bytes.len() - 2; // 排除CS和16H
    let app_payload = &bytes[app_start..app_end];
    
    println!("应用层载荷长度: {} 字节\n", app_payload.len());
    
    println!("应用层载荷: {:02X?}", &app_payload[..30.min(app_payload.len())]);
    println!();
    
    // 使用 csg13 协议和南网地区
    use csg1209022::app::ApplicationLayer;
    use csg1209022::link::control::Direction;
    
    match ApplicationLayer::decode(app_payload, Direction::Up, "csg13", "南网") {
        Ok(application) => {
            println!("✓ 解析成功！\n");
            println!("========== 应用层解析结果 ==========");
            println!("AFN: {:?}", application.afn);
            println!("SEQ: {:?}", application.seq);
            println!();
            
            if let csg1209022::app::ApplicationBody::ReadTaskDataResponse(units) = &application.body {
                println!("数据单元数量: {}", units.len());
                for (i, unit) in units.iter().enumerate() {
                    println!("\n单元 #{}:", i + 1);
                    println!("  DA: {:?}", unit.da);
                    println!("  DI: 0x{:08X}", unit.di);
                    println!("  数据时间: {:?}", unit.time);
                    println!("  原始字节数: {}", unit.raw.len());
                    println!("  内容字节数: {}", unit.content_raw.len());
                    println!("  值: {:?}", unit.value);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ 解析失败: {:?}", e);
        }
    }
}

/// 解析 AFN=12H 读任务数据响应报文 - 自描述格式(表A.10.4)
/// 
/// 根据 Q/CSG1209022-2019 表A.10.4 自描述任务数据格式

fn main() {
    let hex_str = "68 CD 00 CD 00 68 C4 FF FF FF FF FF FF 00 12 70 00 00 0D 03 00 E0 00 01 0C 00 00 01 00 01 00 61 00 00 00 26 07 01 00 00 00 00 01 01 01 00 19 00 00 00 26 07 01 00 00 00 00 01 02 01 00 41 00 00 00 26 07 01 00 00 00 00 01 03 01 00 00 00 00 00 26 07 01 00 00 00 00 01 04 01 00 00 00 00 00 26 07 01 00 00 00 00 01 00 03 00 21 00 00 00 26 07 01 00 00 00 00 01 00 02 00 00 00 00 00 26 07 01 00 00 00 00 01 01 02 00 00 00 00 00 26 07 01 00 00 00 00 01 02 02 00 00 00 00 00 26 07 01 00 00 00 00 01 03 02 00 00 00 00 00 26 07 01 00 00 00 00 01 04 02 00 00 00 00 00 26 07 01 00 00 00 00 01 00 04 00 05 00 00 00 26 07 01 00 00 20 26 07 01 00 00 CA 16";
    
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();
    
    println!("========== Q/CSG1209022-2019 AFN=12H 自描述任务数据解析 ==========\n");
    
    let mut pos = 0;
    
    // 跳过链路层
    println!("【链路层】");
    pos += 1; // 起始符
    let length = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
    println!("  长度: {} 字节", length);
    pos += 4; // L + L
    pos += 1; // 起始符
    
    let control = bytes[pos];
    println!("  控制域: 0x{:02X} (上行报文)", control);
    pos += 1;
    
    println!("  地址域: FF FF FF FF FF FF 00 (广播地址)");
    pos += 7;
    
    println!();
    
    // 应用层
    println!("【应用层】");
    let afn = bytes[pos];
    println!("  AFN: 0x{:02X} (读任务数据)", afn);
    pos += 1;
    
    let seq = bytes[pos];
    println!("  SEQ: 0x{:02X} (单帧报文，需确认)", seq);
    pos += 1;
    
    let da = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
    println!("  DA: 0x{:04X} (终端)", da);
    pos += 2;
    
    let di = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]);
    let task_no = di & 0xFF;
    println!("  DI: 0x{:08X} (任务号 {})", di, task_no);
    pos += 4;
    
    println!();
    println!("【任务数据内容 - 表A.10.4 自描述任务数据格式】");
    
    let data_structure = bytes[pos];
    println!("  数据结构方式: {} ({})", data_structure, 
        if data_structure == 0 { "自描述方式" } else { "按任务定义" });
    pos += 1;
    
    // 读取数据组数 n×m
    let n = bytes[pos]; // 信息点标识组数
    let m = bytes[pos + 1]; // 数据标识编码组数
    println!("  数据组数: n={}, m={} (共 {} 个数据单元)", n, m, n as u32 * m as u32);
    pos += 2;
    
    println!();
    println!("  数据单元详情:");
    println!("  {}", "=".repeat(80));
    
    let mut unit_count = 0;
    let end_pos = bytes.len() - 2; // 排除CS和16H
    
    // 按照自描述格式：每个单元包含 信息点标识(2) + 数据标识(4) + 数据内容(变长) + 数据时间(5)
    for pn_idx in 0..n {
        for di_idx in 0..m {
            if pos >= end_pos - 11 {
                break;
            }
            
            unit_count += 1;
            println!("\n  单元 #{} (Pn#{}, DI#{}):", unit_count, pn_idx + 1, di_idx + 1);
            
            // 信息点标识(2字节)
            let pn = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
            println!("    信息点标识(DA): 0x{:04X} = {}", pn, 
                if pn == 0 { "终端".to_string() } else { format!("测量点{}", pn) });
            pos += 2;
            
            // 数据标识编码(4字节)
            let content_di = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]);
            let di0 = bytes[pos];
            let di1 = bytes[pos+1];
            let di2 = bytes[pos+2];
            let di3 = bytes[pos+3];
            
            println!("    数据标识(DI): 0x{:08X}", content_di);
            println!("      DI3={:02X}, DI2={:02X}, DI1={:02X}, DI0={:02X}", di3, di2, di1, di0);
            
            // 解析DI含义
            let di_desc = decode_di(content_di);
            if !di_desc.is_empty() {
                println!("      含义: {}", di_desc);
            }
            pos += 4;
            
            // 数据内容(变长) - 根据DI判断长度
            let content_len = estimate_content_length(content_di);
            
            if pos + content_len + 5 <= end_pos {
                print!("    数据内容: ");
                for i in 0..content_len {
                    print!("{:02X} ", bytes[pos + i]);
                }
                
                // 解析数据值
                let value = parse_content_value(&bytes[pos..pos+content_len], content_di);
                println!("({} 字节) = {}", content_len, value);
                pos += content_len;
                
                // 数据时间(5字节 BCD: YYMMDDhhmm)
                let time_bytes = &bytes[pos..pos+5];
                let year = bcd_to_decimal(time_bytes[0]);
                let month = bcd_to_decimal(time_bytes[1]);
                let day = bcd_to_decimal(time_bytes[2]);
                let hour = bcd_to_decimal(time_bytes[3]);
                let minute = bcd_to_decimal(time_bytes[4]);
                
                print!("    数据时间: ");
                for i in 0..5 {
                    print!("{:02X} ", time_bytes[i]);
                }
                println!("= 20{:02}年{:02}月{:02}日 {:02}:{:02}", 
                    year, month, day, hour, minute);
                pos += 5;
            } else {
                println!("    [数据不完整]");
                break;
            }
        }
    }
    
    println!();
    println!("  {}", "=".repeat(80));
    
    println!();
    println!("【帧尾】");
    println!("  校验和CS: 0x{:02X}", bytes[bytes.len() - 2]);
    println!("  结束符: 0x{:02X}", bytes[bytes.len() - 1]);
    
    println!();
    println!("========== 解析完成 ==========");
    println!("\n统计信息:");
    println!("  • 任务号: {}", task_no);
    println!("  • 数据格式: 自描述方式(表A.10.4)");
    println!("  • 信息点组数: {}", n);
    println!("  • 数据标识组数: {}", m);
    println!("  • 总数据单元: {}", unit_count);
    println!("  • 数据时间: 2026年7月1日 00:00");
}

fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

fn estimate_content_length(di: u32) -> usize {
    // 根据DI估算数据内容长度
    // 这里简化处理，实际需要完整的DI映射表
    let di0 = (di & 0xFF) as u8;
    let di1 = ((di >> 8) & 0xFF) as u8;
    let di2 = ((di >> 16) & 0xFF) as u8;
    
    match (di3_value(di), di2, di1, di0) {
        // C.x 附录中的常见数据项
        (0x00, 0x01, 0x00, 0x00) => 4, // 正向有功总电能
        (0x00, 0x01, 0x01, 0x00) => 4,
        (0x00, 0x01, 0x02, 0x00) => 4,
        (0x00, 0x01, 0x03, 0x00) => 4,
        (0x00, 0x01, 0x04, 0x00) => 4,
        (0x00, 0x03, 0x00, 0x00) => 4,
        (0x00, 0x02, 0x00..=0x04, 0x00) => 4,
        (0x00, 0x04, 0x00, 0x00) => 4,
        _ => 4, // 默认4字节
    }
}

fn di3_value(di: u32) -> u8 {
    ((di >> 24) & 0xFF) as u8
}

fn decode_di(di: u32) -> String {
    let di0 = (di & 0xFF) as u8;
    let di1 = ((di >> 8) & 0xFF) as u8;
    let di2 = ((di >> 16) & 0xFF) as u8;
    let di3 = ((di >> 24) & 0xFF) as u8;
    
    // 根据附录C解析常见DI
    match (di3, di2, di1, di0) {
        // 正向有功电能 (DI2=01)
        (0x00, 0x01, 0x00, 0x00) => "正向有功总电能(当前)".to_string(),
        (0x00, 0x01, 0x00, 0x01) => "正向有功总电能".to_string(),
        (0x00, 0x01, 0x01, 0x00) => "正向有功费率1电能(当前)".to_string(),
        (0x00, 0x01, 0x01, 0x01) => "正向有功费率1电能".to_string(),
        (0x00, 0x01, 0x02, 0x00) => "正向有功费率2电能(当前)".to_string(),
        (0x00, 0x01, 0x02, 0x01) => "正向有功费率2电能".to_string(),
        (0x00, 0x01, 0x03, 0x00) => "正向有功费率3电能(当前)".to_string(),
        (0x00, 0x01, 0x03, 0x01) => "正向有功费率3电能".to_string(),
        (0x00, 0x01, 0x04, 0x00) => "正向有功费率4电能(当前)".to_string(),
        (0x00, 0x01, 0x04, 0x01) => "正向有功费率4电能".to_string(),
        
        // 反向有功电能 (DI2=02)
        (0x00, 0x02, 0x00, 0x00) => "反向有功总电能(当前)".to_string(),
        (0x00, 0x02, 0x00, 0x01) => "反向有功总电能".to_string(),
        (0x00, 0x02, 0x01, 0x00) => "反向有功费率1电能(当前)".to_string(),
        (0x00, 0x02, 0x01, 0x01) => "反向有功费率1电能".to_string(),
        (0x00, 0x02, 0x02, 0x00) => "反向有功费率2电能(当前)".to_string(),
        (0x00, 0x02, 0x02, 0x01) => "反向有功费率2电能".to_string(),
        (0x00, 0x02, 0x03, 0x00) => "反向有功费率3电能(当前)".to_string(),
        (0x00, 0x02, 0x03, 0x01) => "反向有功费率3电能".to_string(),
        (0x00, 0x02, 0x04, 0x00) => "反向有功费率4电能(当前)".to_string(),
        (0x00, 0x02, 0x04, 0x01) => "反向有功费率4电能".to_string(),
        
        // 组合无功电能 (DI2=03/04)
        (0x00, 0x03, 0x00, 0x00) => "组合无功1总电能(当前)".to_string(),
        (0x00, 0x03, 0x00, 0x01) => "组合无功1总电能".to_string(),
        (0x00, 0x04, 0x00, 0x00) => "组合无功2总电能(当前)".to_string(),
        (0x00, 0x04, 0x00, 0x01) => "组合无功2总电能".to_string(),
        
        _ => String::new(),
    }
}

fn parse_content_value(data: &[u8], di: u32) -> String {
    if data.is_empty() {
        return "无数据".to_string();
    }
    
    // 大多数电能数据是4字节BIN格式，单位0.01kWh或0.01kvarh
    if data.len() == 4 {
        let value = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        
        // 判断是否是电能类数据
        let di2 = ((di >> 16) & 0xFF) as u8;
        if di2 >= 0x01 && di2 <= 0x04 {
            // 电能数据，单位0.01kWh
            return format!("{:.2} kWh", value as f64 / 100.0);
        }
        
        return format!("{}", value);
    }
    
    // 其他长度的数据暂时显示原始值
    let hex: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
    hex.join(" ")
}

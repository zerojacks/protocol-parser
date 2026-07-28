/// 手动解析 AFN=12H 读任务数据响应报文
/// 
/// 这个示例专门处理包含广播地址(FF FF FF FF FF FF)的报文
/// 因为广播地址不是有效的BCD编码，所以需要特殊处理

fn main() {
    let hex_str = "68 CD 00 CD 00 68 C4 FF FF FF FF FF FF 00 12 70 00 00 0D 03 00 E0 00 01 0C 00 00 01 00 01 00 61 00 00 00 26 07 01 00 00 00 00 01 01 01 00 19 00 00 00 26 07 01 00 00 00 00 01 02 01 00 41 00 00 00 26 07 01 00 00 00 00 01 03 01 00 00 00 00 00 26 07 01 00 00 00 00 01 04 01 00 00 00 00 00 26 07 01 00 00 00 00 01 00 03 00 21 00 00 00 26 07 01 00 00 00 00 01 00 02 00 00 00 00 00 26 07 01 00 00 00 00 01 01 02 00 00 00 00 00 26 07 01 00 00 00 00 01 02 02 00 00 00 00 00 26 07 01 00 00 00 00 01 03 02 00 00 00 00 00 26 07 01 00 00 00 00 01 04 02 00 00 00 00 00 26 07 01 00 00 00 00 01 00 04 00 05 00 00 00 26 07 01 00 00 20 26 07 01 00 00 CA 16";
    
    let bytes: Vec<u8> = hex_str
        .split_whitespace()
        .filter_map(|s| u8::from_str_radix(s, 16).ok())
        .collect();
    
    println!("========== Q/CSG1209022-2019 报文解析 ==========\n");
    println!("原始报文长度: {} 字节", bytes.len());
    println!();
    
    let mut pos = 0;
    
    // 链路层
    println!("【链路层】");
    println!("  起始符: 0x{:02X}", bytes[pos]);
    pos += 1;
    
    let length = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
    println!("  长度L: {} (0x{:04X}), 总长度 = {} + 8 = {}", 
        length, length, length, length as usize + 8);
    pos += 4; // L + L重复
    
    println!("  起始符: 0x{:02X}", bytes[pos]);
    pos += 1;
    
    let control = bytes[pos];
    println!("  控制域C: 0x{:02X} (二进制: {:08b})", control, control);
    println!("    - DIR(D7): {} ({})", control >> 7, if control >> 7 == 1 { "上行" } else { "下行" });
    println!("    - PRM(D6): {} ({})", (control >> 6) & 1, if (control >> 6) & 1 == 1 { "启动站" } else { "从动站" });
    println!("    - ACD(D5): {} ({})", (control >> 5) & 1, if (control >> 5) & 1 == 1 { "有告警" } else { "无告警" });
    println!("    - 保留(D4): {}", (control >> 4) & 1);
    println!("    - 功能码(D3-D0): {} (用户数据)", control & 0x0F);
    pos += 1;
    
    println!("  地址域A: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}", 
        bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3], 
        bytes[pos+4], bytes[pos+5], bytes[pos+6]);
    println!("    - A1(省地市区县): FF FF FF (广播地址)");
    println!("    - A2(终端地址): FF FF FF (广播地址)");
    println!("    - A3(主站地址): {:02X}", bytes[pos+6]);
    pos += 7;
    
    println!();
    
    // 应用层
    println!("【应用层】");
    let afn = bytes[pos];
    println!("  AFN: 0x{:02X} ({})", afn, match afn {
        0x12 => "读任务数据",
        _ => "其他"
    });
    pos += 1;
    
    let seq = bytes[pos];
    println!("  SEQ: 0x{:02X} (二进制: {:08b})", seq, seq);
    println!("    - TpV(D7): {} ({})", seq >> 7, if seq >> 7 == 1 { "有时间标签" } else { "无时间标签" });
    println!("    - FIR(D6): {} ({})", (seq >> 6) & 1, if (seq >> 6) & 1 == 1 { "首帧" } else { "非首帧" });
    println!("    - FIN(D5): {} ({})", (seq >> 5) & 1, if (seq >> 5) & 1 == 1 { "末帧" } else { "非末帧" });
    println!("    - CON(D4): {} ({})", (seq >> 4) & 1, if (seq >> 4) & 1 == 1 { "需确认" } else { "不需确认" });
    println!("    - 帧序号(D3-D0): {}", seq & 0x0F);
    pos += 1;
    
    let da = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
    println!("  DA(信息点标识): 0x{:04X} ({})", da, if da == 0 { "终端" } else { &format!("测量点{}", da) });
    pos += 2;
    
    let di = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]);
    println!("  DI(数据标识): 0x{:08X} (任务号{})", di, di & 0xFF);
    pos += 4;
    
    println!();
    println!("【任务数据内容】(表A.10.5 任务定义数据格式)");
    
    let data_structure = bytes[pos];
    println!("  数据结构方式: {} ({})", data_structure, 
        if data_structure == 0 { "按任务定义，只上报数据内容" } else { "未知" });
    pos += 1;
    
    let end_pos = bytes.len() - 2; // 排除CS和16H
    
    println!();
    println!("  剩余数据(十六进制查看):");
    println!("  位置 | 十六进制                                        | ASCII");
    println!("  -----|------------------------------------------------|--------");
    
    let mut view_pos = pos;
    while view_pos < end_pos {
        let line_len = (end_pos - view_pos).min(16);
        print!("  {:04X} | ", view_pos);
        
        for i in 0..line_len {
            print!("{:02X} ", bytes[view_pos + i]);
        }
        for _ in line_len..16 {
            print!("   ");
        }
        print!("| ");
        
        for i in 0..line_len {
            let b = bytes[view_pos + i];
            if b >= 0x20 && b < 0x7F {
                print!("{}", b as char);
            } else {
                print!(".");
            }
        }
        println!();
        view_pos += line_len;
    }
    
    println!();
    println!("  数据组解析:");
    
    let mut group_num = 1;
    
    while pos < end_pos - 10 { // 确保有足够的字节
        // 读取信息点标识(2字节)
        let pn = u16::from_le_bytes([bytes[pos], bytes[pos+1]]);
        println!("\n  组{}: 信息点标识 = 0x{:04X} ({})", 
            group_num, pn, if pn == 0 { "终端" } else { &format!("测量点{}", pn) });
        pos += 2;
        
        // 读取DI(4字节)
        let content_di = u32::from_le_bytes([bytes[pos], bytes[pos+1], bytes[pos+2], bytes[pos+3]]);
        println!("        数据标识 = 0x{:08X}", content_di);
        
        // 解析DI的含义
        let di0 = bytes[pos];
        let di1 = bytes[pos+1];
        let di2 = bytes[pos+2];
        let di3 = bytes[pos+3];
        
        println!("          DI0={:02X}, DI1={:02X}, DI2={:02X}, DI3={:02X}", di0, di1, di2, di3);
        
        // 根据DI判断数据长度(这里简化处理，实际需要查表)
        let data_len = estimate_data_length(content_di);
        pos += 4;
        
        // 读取数据内容
        if pos + data_len + 5 <= end_pos {
            print!("        数据内容 = ");
            for i in 0..data_len {
                print!("{:02X} ", bytes[pos + i]);
            }
            println!("({} 字节)", data_len);
            pos += data_len;
            
            // 读取数据时间(5字节 BCD: YYMMDDhhmm)
            let time_bytes = &bytes[pos..pos+5];
            print!("        数据时间 = ");
            for i in 0..5 {
                print!("{:02X} ", time_bytes[i]);
            }
            
            let year = bcd_to_decimal(time_bytes[0]);
            let month = bcd_to_decimal(time_bytes[1]);
            let day = bcd_to_decimal(time_bytes[2]);
            let hour = bcd_to_decimal(time_bytes[3]);
            let minute = bcd_to_decimal(time_bytes[4]);
            
            println!("= 20{:02}年{:02}月{:02}日 {:02}:{:02}", 
                year, month, day, hour, minute);
            pos += 5;
            
            group_num += 1;
        } else {
            break;
        }
    }
    
    println!();
    println!("【帧尾】");
    println!("  校验和CS: 0x{:02X}", bytes[bytes.len() - 2]);
    println!("  结束符: 0x{:02X}", bytes[bytes.len() - 1]);
    
    println!("\n========== 解析完成 ==========");
    println!("\n报文总结:");
    println!("  • 这是一个上行报文(终端→主站)");
    println!("  • 使用广播地址(FF FF FF FF FF FF)");
    println!("  • AFN=12H: 读任务数据响应");
    println!("  • 任务号: {}", di & 0xFF);
    println!("  • 包含 {} 组数据", group_num - 1);
    println!("  • 采用任务定义数据格式(表A.10.5)");
}

fn bcd_to_decimal(bcd: u8) -> u8 {
    (bcd >> 4) * 10 + (bcd & 0x0F)
}

fn estimate_data_length(di: u32) -> usize {
    // 根据DI的不同，数据长度也不同
    // 这里根据实际报文推测长度
    let di0 = (di & 0xFF) as u8;
    let di1 = ((di >> 8) & 0xFF) as u8;
    let di2 = ((di >> 16) & 0xFF) as u8;
    
    // 分析实际报文，第一组有特殊格式
    // 01 0C 00 00 01 00 01 00 61 00 00 00 (12字节)
    // 这可能是任务参数定义
    
    match (di2, di1, di0) {
        (0x01, 0x00, 0x00) => {
            // 第一次出现可能是任务参数定义，长度不同
            // 需要根据上下文判断
            10 // 暂定
        },
        _ => 4, // 其他默认4字节
    }
}

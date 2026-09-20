//! 构建和编码 Q/CSG1209021-2019 协议帧
//!
//! 本示例演示如何手动构建协议帧并编码为字节数组。

use csg_local_comm::{
    encode_message, Afn, ApplicationBody, ApplicationLayer, ControlByte, DataIdentifier, Message,
};
use csg_local_comm::app::{MessageDirection, NodeRole};
use csg_local_comm::link::{Address, AddressDomain, Frame};

fn main() {
    println!("=== 示例 1: 构建下行查询参数帧 ===");

    // 1. 构建链路层
    let frame = Frame {
        control: ControlByte::downlink_primary_with_address(),
        address: Some(AddressDomain {
            source: Address::from_bytes([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]),
            destination: Address::from_bytes([0x11, 0x12, 0x13, 0x14, 0x15, 0x16]),
        }),
        payload: Vec::new(), // 会被应用层填充
        checksum: 0,
    };

    // 2. 构建应用层
    let app = ApplicationLayer {
        afn: Afn::QueryParam,
        seq: 1,
        di: DataIdentifier {
            node_role: NodeRole::Concentrator,
            direction: MessageDirection::BothSameFormat,
            afn_match: 0x03,
            sub_function: 0x01,
        },
        body: ApplicationBody::Empty,
    };

    // 3. 组合为完整消息
    let msg = Message { frame, app };

    // 4. 编码
    match encode_message(&msg) {
        Ok(bytes) => {
            println!("✓ 编码成功，字节数: {}", bytes.len());
            println!("原始字节: {:02X?}", bytes);
            print_hex_dump(&bytes);
        }
        Err(e) => {
            eprintln!("✗ 编码失败: {:?}", e);
        }
    }

    println!("\n=== 示例 2: 构建上行确认帧 ===");

    let ack_frame = Frame {
        control: ControlByte::uplink_response_without_address(),
        address: None, // 无地址域
        payload: Vec::new(),
        checksum: 0,
    };

    let ack_app = ApplicationLayer {
        afn: Afn::AckNack,
        seq: 1,
        di: DataIdentifier::ack(NodeRole::Concentrator),
        body: ApplicationBody::Ack,
    };

    let ack_msg = Message {
        frame: ack_frame,
        app: ack_app,
    };

    match encode_message(&ack_msg) {
        Ok(bytes) => {
            println!("✓ 编码成功，字节数: {}", bytes.len());
            println!("原始字节: {:02X?}", bytes);
            print_hex_dump(&bytes);
        }
        Err(e) => {
            eprintln!("✗ 编码失败: {:?}", e);
        }
    }

    println!("\n=== 示例 3: 构建上行否认帧 ===");

    let nack_frame = Frame {
        control: ControlByte::uplink_response_without_address(),
        address: None,
        payload: Vec::new(),
        checksum: 0,
    };

    let nack_app = ApplicationLayer {
        afn: Afn::AckNack,
        seq: 2,
        di: DataIdentifier::nack(NodeRole::Concentrator),
        body: ApplicationBody::Nack { error_code: 0x05 }, // 错误码 0x05
    };

    let nack_msg = Message {
        frame: nack_frame,
        app: nack_app,
    };

    match encode_message(&nack_msg) {
        Ok(bytes) => {
            println!("✓ 编码成功，字节数: {}", bytes.len());
            println!("原始字节: {:02X?}", bytes);
            print_hex_dump(&bytes);

            // 验证编码-解码往返
            println!("\n验证编码-解码往返...");
            match csg_local_comm::decode_message(&bytes) {
                Ok((decoded, _)) => {
                    println!("✓ 解码成功");
                    println!("  AFN: {:?}", decoded.app.afn);
                    println!("  SEQ: {}", decoded.app.seq);
                    println!("  DI: {:08X}H", decoded.app.di.to_u32());
                    println!("  数据体: {:?}", decoded.app.body);
                }
                Err(e) => {
                    eprintln!("✗ 解码失败: {:?}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ 编码失败: {:?}", e);
        }
    }

    println!("\n=== 示例 4: 构建带数据内容的帧 ===");

    let data_frame = Frame {
        control: ControlByte::downlink_primary_without_address(),
        address: None,
        payload: Vec::new(),
        checksum: 0,
    };

    let data_app = ApplicationLayer {
        afn: Afn::SetParam,
        seq: 10,
        di: DataIdentifier {
            node_role: NodeRole::Concentrator,
            direction: MessageDirection::BothSameFormat,
            afn_match: 0x02,
            sub_function: 0x05,
        },
        body: ApplicationBody::WithData {
            raw_data: vec![0x01, 0x02, 0x03, 0x04], // 示例数据
            parsed_value: None,
        },
    };

    let data_msg = Message {
        frame: data_frame,
        app: data_app,
    };

    match encode_message(&data_msg) {
        Ok(bytes) => {
            println!("✓ 编码成功，字节数: {}", bytes.len());
            println!("原始字节: {:02X?}", bytes);
            print_hex_dump(&bytes);
        }
        Err(e) => {
            eprintln!("✗ 编码失败: {:?}", e);
        }
    }
}

/// 打印十六进制转储（类似 hexdump）
fn print_hex_dump(data: &[u8]) {
    println!("\n十六进制转储:");
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04X}  ", i * 16);
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02X} ", byte);
            if j == 7 {
                print!(" ");
            }
        }
        // 填充空格
        for _ in chunk.len()..16 {
            print!("   ");
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

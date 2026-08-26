//! 演示 ReadRequest 中显示 DI 名称的功能

use dlt645_2007::{
    decode_message, Address, ApplicationBody, ApplicationLayer, ControlCode, DataIdentifier,
    FunctionCode, Frame,
};

fn main() {
    // 构造一个读数据请求，包含多个 DI
    let frame = Frame {
        address: Address::from_decimal_str("123456789012").unwrap(),
        control: ControlCode::master_request(FunctionCode::ReadData),
        payload: Vec::new(),
    };

    let application = ApplicationLayer {
        function: FunctionCode::ReadData,
        body: ApplicationBody::ReadRequest {
            identifiers: vec![
                DataIdentifier::from_bytes([0x00, 0x01, 0x00, 0x00]), // 组合有功电能
                DataIdentifier::from_bytes([0x01, 0x01, 0x00, 0x00]), // 正向有功总电能
                DataIdentifier::from_bytes([0x02, 0x01, 0x00, 0x00]), // 反向有功总电能
            ],
        },
    };

    // 编码
    let bytes = dlt645_2007::encode_message(&frame, &application, false);

    println!("编码后的报文: {:02X?}\n", bytes);

    // 解码并渲染为树形结构
    let (msg, _) = decode_message(&bytes, "dlt645-2007", "南网").unwrap();
    let tree = msg.to_value_tree("dlt645-2007", "南网").unwrap();

    // 打印树形结构
    println!("解析结果:");
    print_value_tree(&tree, 0);
}

fn print_value_tree(value: &proto_common::FieldValue, indent: usize) {
    use proto_common::FieldValue;
    
    let prefix = "  ".repeat(indent);
    
    match value {
        FieldValue::Node { name, raw, value } => {
            println!("{}📦 {} [原始字节: {} bytes]", prefix, name, raw.len());
            print_value_tree(value, indent + 1);
        }
        FieldValue::List(items) => {
            for item in items {
                print_value_tree(item, indent);
            }
        }
        FieldValue::Str(s) => {
            println!("{}  └─ {}", prefix, s);
        }
        FieldValue::Int(i) => {
            println!("{}  └─ {}", prefix, i);
        }
        FieldValue::Bit { bit_start, bit_end, bit_value, value: Some(v), .. } => {
            println!("{}  🔹 D{}~D{} = {}", prefix, bit_start, bit_end, bit_value);
            print_value_tree(v, indent + 1);
        }
        _ => {}
    }
}

//! 按用户截图里给出的原始报文手工拼字节，验证 decode_message_as_value 的输出。
//!
//! 报文: 68 16 00 16 00 68 89 00 00 44 11 00 00 01 00 E5 00 00 00 00 00 E0 01 20 11 38 15 00 23 16

fn main() {
    let hex = "68 16 00 16 00 68 89 00 00 44 11 00 00 01 00 E5 00 00 00 00 00 E0 01 20 11 38 15 00 23 16";
    let bytes: Vec<u8> = hex
        .split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect();

    match csg1209022::decode_message_as_value(&bytes, "csg13", "南网") {
        Ok((tree, consumed)) => {
            println!("consumed = {consumed}, total = {}", bytes.len());
            println!("{tree:#?}");
        }
        Err(e) => println!("解析失败: {e}"),
    }
}

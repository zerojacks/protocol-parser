//! 用用户截图里给出的真实报文做回归测试：
//! `68 16 00 16 00 68 89 00 00 44 11 00 00 01 00 E5 00 00 00 00 00 E0 01 20 11 38 15 00 23 16`
//!
//! 这条报文是终端对"确认/否定"类命令的上行否定响应（AFN=00H，DI=E0000000 全部否定），
//! 带时间标签 Tp。逐字段核对过之后，这里锁定关键字段的渲染结果，防止后续改动悄悄破坏。

use csg1209022::FieldValue as Value;

fn sample_bytes() -> Vec<u8> {
    let hex = "68 16 00 16 00 68 89 00 00 44 11 00 00 01 00 E5 00 00 00 00 00 E0 01 20 11 38 15 00 23 16";
    hex.split_whitespace()
        .map(|b| u8::from_str_radix(b, 16).unwrap())
        .collect()
}

fn find_child<'a>(rows: &'a [Value], name: &str) -> &'a Value {
    rows.iter()
        .find(|v| matches!(v, Value::Node { name: n, .. } if n == name))
        .unwrap_or_else(|| panic!("找不到名为 {name} 的节点"))
}

fn as_list(v: &Value) -> &[Value] {
    match v {
        Value::Node { value, .. } => match value.as_ref() {
            Value::List(items) => items,
            other => panic!("期望 List，实际 {other:?}"),
        },
        other => panic!("期望 Node，实际 {other:?}"),
    }
}

fn as_str(v: &Value) -> &str {
    match v {
        Value::Node { value, .. } => match value.as_ref() {
            Value::Str(s) => s.as_str(),
            other => panic!("期望 Str，实际 {other:?}"),
        },
        other => panic!("期望 Node，实际 {other:?}"),
    }
}

fn raw_of(v: &Value) -> &[u8] {
    match v {
        Value::Node { raw, .. } => raw,
        other => panic!("期望 Node，实际 {other:?}"),
    }
}

#[test]
fn matches_real_frame_from_screenshot() {
    let bytes = sample_bytes();
    let (tree, consumed) = csg1209022::decode_message_as_value(&bytes, "csg13", "南网").unwrap();
    assert_eq!(consumed, bytes.len());
    assert_eq!(consumed, 30);

    let Value::Node { name: root_name, raw: root_raw, value } = &tree else {
        panic!("根节点必须是 Node");
    };
    assert_eq!(root_name, "报文");
    assert_eq!(root_raw, &bytes);
    let Value::List(rows) = value.as_ref() else {
        panic!("根节点的值必须是 List");
    };

    // 长度：22 / 30
    assert_eq!(
        as_str(find_child(rows, "长度")),
        "长度=22，总长度=30(总长度=长度+8)"
    );

    // 控制域：89H，D3~D0=9（否定：无所召唤数据，来自从动站）
    let control_children = as_list(find_child(rows, "控制域C"));
    let fc = control_children
        .iter()
        .find(|v| matches!(v, Value::Node{name,..} if name=="D3~D0功能码"))
        .unwrap();
    if let Value::Node { value, .. } = fc {
        if let Value::Bit { bit_value, value: desc, .. } = value.as_ref() {
            assert_eq!(*bit_value, 9);
            assert_eq!(
                desc.as_deref().unwrap().as_str().unwrap(),
                "来自从动站:否定：无所召唤数据"
            );
        } else {
            panic!("期望 Bit");
        }
    }

    // 地址域：省44/地市00/区县00，终端地址十六进制 000011（不是十进制 000017）
    let addr_children = as_list(find_child(rows, "地址域A"));
    assert_eq!(
        as_str(find_child(addr_children, "省地市区县码A1")),
        "省地市区县码=440000省44,地市00,区县00"
    );
    assert_eq!(
        as_str(find_child(addr_children, "终端地址A2")),
        "终端地址=000011"
    );
    assert_eq!(
        as_str(find_child(addr_children, "主站地址A3")),
        "主站地址=1"
    );

    // AFN=00H 确认/否定
    assert_eq!(as_str(find_child(rows, "应用层功能码AFN")), "确认/否定");

    // SEQ=E5H：单帧、不需要确认、帧内序号=5
    let seq_children = as_list(find_child(rows, "命令序号SEQ"));
    let frame_no = seq_children
        .iter()
        .find(|v| matches!(v, Value::Node{name,..} if name=="D3~D0帧内序号"))
        .unwrap();
    if let Value::Node { value, .. } = frame_no {
        if let Value::Bit { bit_value, .. } = value.as_ref() {
            assert_eq!(*bit_value, 5);
        }
    }

    // 信息体 -> <第1组> -> 数据标识内容 应该是 spec-engine 解析出来的 E0000000 节点，raw=[1]
    let body_children = as_list(find_child(rows, "信息体"));
    assert_eq!(body_children.len(), 1);
    let group_children = as_list(&body_children[0]);
    let da_node = find_child(group_children, "信息点标识DA");
    assert_eq!(as_str(da_node), "Pn=测量点：0(终端)");
    assert_eq!(raw_of(da_node), &[0, 0]);

    let di_node = find_child(group_children, "数据标识编码DI");
    assert_eq!(
        as_str(di_node),
        "数据标识编码：[E0000000]-全部进行确定/否定"
    );
    assert_eq!(raw_of(di_node), &[0x00, 0x00, 0x00, 0xE0]);

    let content_node = find_child(group_children, "数据标识内容");
    assert_eq!(raw_of(content_node), &[0x01]);
    if let Value::Node { value, .. } = content_node {
        if let Value::Node {
            name: inner_name,
            value: inner_value,
            ..
        } = value.as_ref()
        {
            assert_eq!(inner_name, "E0000000_全部进行确定/否定");
            assert_eq!(inner_value.as_ref(), &Value::Int(1)); // 1=否定
        } else {
            panic!("期望内层是 spec-engine 的 Node");
        }
    }

    // 时间标签：20日11时38分15秒，允许延时0分
    let tp_node = find_child(rows, "时间标签Tp");
    assert_eq!(raw_of(tp_node), &[0x20, 0x11, 0x38, 0x15, 0x00]);
    assert_eq!(
        as_str(tp_node),
        "启动帧发送时标：20日11时38分15秒。允许发送传输延迟时间：0分"
    );

    // 校验码/结束符
    assert_eq!(as_str(find_child(rows, "校验码CS")), "校验正确");
    assert_eq!(raw_of(find_child(rows, "校验码CS")), &[0x23]);
    assert_eq!(raw_of(find_child(rows, "结束符")), &[0x16]);
}

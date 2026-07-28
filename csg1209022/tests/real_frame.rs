//! 端到端集成测试：构造一条真实的"读当前数据"上行（应答）报文，
//! 验证 链路层Frame -> 应用层ApplicationLayer -> spec-engine内容解析 全链路能打通。
//!
//! DI=0x00010000 对应 csg13/南网 字典里的"(当前)正向有功总电能"，
//! 字节 `23 01 00 00` 按 BCD + decimal(2) 解出应为 1.23 kWh
//! （这个换算已经在 spec-engine 侧用 `cargo run --example` 实测验证过）。

use csg1209022::app::{Afn, ApplicationBody, DataAddress, PointSelector, SeqField};
use csg1209022::link::address::RegionCode;
use csg1209022::link::{AddressField, ControlField, Frame};
use csg1209022::{decode_message, ProtoError};
use csg1209022::FieldValue as Value;

fn sample_address() -> AddressField {
    AddressField {
        region: RegionCode {
            province: 44,
            city: 1,
            county: 5,
        },
        terminal_addr: 123_456,
        master_addr: 1,
    }
}

/// 手工拼出应用层字节：AFN=0CH, SEQ(单帧), DA(p1) + DI(0x00010000) + 内容(4字节BCD)
fn build_read_current_data_response_payload() -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(Afn::ReadCurrentData.to_u8()); // 0x0C
    payload.push(SeqField::single_frame(false, 0).unwrap().encode());

    let da = DataAddress::from_point_number(1).unwrap();
    payload.extend_from_slice(&da.encode()); // DA1,DA2

    let di: u32 = 0x0001_0000;
    payload.extend_from_slice(&di.to_le_bytes()); // DI0,DI1,DI2,DI3（小端）

    payload.extend_from_slice(&[0x23, 0x01, 0x00, 0x00]); // 内容：1.23（BCD, decimal=2）
    payload
}

#[test]
fn decodes_real_read_current_data_response_through_spec_engine() {
    let frame = Frame {
        control: ControlField::new_up(false, false, false, 8).unwrap(),
        address: sample_address(),
        payload: build_read_current_data_response_payload(),
    };
    let bytes = frame.encode().expect("帧编码不应失败");

    let (msg, consumed) =
        decode_message(&bytes, "csg13", "南网").expect("完整报文应能成功解码");
    assert_eq!(consumed, bytes.len());
    assert_eq!(msg.application.afn, Afn::ReadCurrentData);
    assert!(msg.application.seq.is_single_frame());

    let units = match msg.application.body {
        ApplicationBody::ReadCurrentDataResponse(units) => units,
        other => panic!("期望 ReadCurrentDataResponse，实际得到 {other:?}"),
    };
    assert_eq!(units.len(), 1);

    let unit = &units[0];
    assert_eq!(unit.di, 0x0001_0000);
    assert_eq!(unit.da.point_selector(), PointSelector::Points(vec![1]));
    assert!(unit.time.is_none()); // 读当前数据的信息体不带数据时间域

    match &unit.value {
        Value::Node { name, value, .. } => {
            assert!(name.contains("正向有功总电能"));
            match value.as_ref() {
                Value::WithUnit { value, unit } => {
                    assert_eq!(unit, "kWh");
                    match value.as_ref() {
                        Value::Float(f) => assert!((f - 1.23).abs() < 1e-9),
                        other => panic!("期望 Float，实际 {other:?}"),
                    }
                }
                other => panic!("期望 WithUnit，实际 {other:?}"),
            }
        }
        other => panic!("期望 Node，实际 {other:?}"),
    }
}

#[test]
fn rejects_frame_with_corrupted_checksum() {
    let frame = Frame {
        control: ControlField::new_up(false, false, false, 8).unwrap(),
        address: sample_address(),
        payload: build_read_current_data_response_payload(),
    };
    let mut bytes = frame.encode().unwrap();
    let cs_index = bytes.len() - 2;
    bytes[cs_index] ^= 0xFF;

    let err = decode_message(&bytes, "csg13", "南网").unwrap_err();
    assert!(matches!(err, ProtoError::ChecksumMismatch { .. }));
}

#[test]
fn unknown_di_surfaces_as_dict_error_not_panic() {
    let mut payload = Vec::new();
    payload.push(Afn::ReadCurrentData.to_u8());
    payload.push(SeqField::single_frame(false, 0).unwrap().encode());
    payload.extend_from_slice(&DataAddress::from_point_number(1).unwrap().encode());
    payload.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes()); // 字典里不存在的DI
    payload.extend_from_slice(&[0x00]);

    let frame = Frame {
        control: ControlField::new_up(false, false, false, 8).unwrap(),
        address: sample_address(),
        payload,
    };
    let bytes = frame.encode().unwrap();

    let err = decode_message(&bytes, "csg13", "南网").unwrap_err();
    assert!(matches!(err, ProtoError::Dict(_)));
}

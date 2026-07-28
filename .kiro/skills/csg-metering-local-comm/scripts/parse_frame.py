#!/usr/bin/env python3
"""
Parse / build frames for Q/CSG1209021-2019
"计量自动化终端本地通信模块接口协议".

Usage:
    python parse_frame.py "68 0F 00 41 02 01 03 00 E8 02 01 03 30 16"
    python parse_frame.py --hex 680F0041020103 00E80201033016
    python parse_frame.py --build --afn 01 --di E8 02 01 03 --seq 00 \
        --dir down --prm 1 --add 0 --data ""

Notes on byte order: the spec (§5.2) says multi-byte BIN fields are transmitted
low-byte-first. This script decodes L as little-endian by default; pass
--big-endian-length if a real capture turns out to use big-endian (some vendor
implementations do this inconsistently -- always cross-check against the total
byte count of the frame you were given).
"""
import argparse
import sys

# ---------------------------------------------------------------------------
# Minimal AFN/DI name lookup (see references/afn-di-codes.md for the full,
# authoritative table -- this dict covers the most common commands so the
# script can label a frame without a second file read).
# ---------------------------------------------------------------------------
DI_NAMES = {
    # 集中器接口 (E8)
    (0xE8, 0x01, 0x00, 0x01): "确认",
    (0xE8, 0x01, 0x00, 0x02): "否认",
    (0xE8, 0x02, 0x01, 0x01): "复位硬件",
    (0xE8, 0x02, 0x01, 0x02): "初始化档案",
    (0xE8, 0x02, 0x01, 0x03): "初始化任务",
    (0xE8, 0x02, 0x02, 0x01): "添加任务",
    (0xE8, 0x02, 0x02, 0x02): "删除任务",
    (0xE8, 0x00, 0x02, 0x03): "查询未完成任务数",
    (0xE8, 0x03, 0x02, 0x04): "查询未完成任务列表",
    (0xE8, 0x04, 0x02, 0x04): "返回查询未完成任务列表",
    (0xE8, 0x03, 0x02, 0x05): "查询未完成任务详细信息",
    (0xE8, 0x04, 0x02, 0x05): "返回查询未完成任务详细信息",
    (0xE8, 0x00, 0x02, 0x06): "查询剩余可分配任务数",
    (0xE8, 0x02, 0x02, 0x07): "添加多播任务",
    (0xE8, 0x02, 0x02, 0x08): "启动任务",
    (0xE8, 0x02, 0x02, 0x09): "暂停任务",
    (0xE8, 0x00, 0x03, 0x01): "查询厂商代码和版本信息",
    (0xE8, 0x00, 0x03, 0x02): "查询本地通信模块运行模式信息",
    (0xE8, 0x00, 0x03, 0x03): "查询主节点地址",
    (0xE8, 0x03, 0x03, 0x04): "查询通信延时时长",
    (0xE8, 0x04, 0x03, 0x04): "返回查询通信延时时长",
    (0xE8, 0x00, 0x03, 0x05): "查询从节点数量",
    (0xE8, 0x03, 0x03, 0x06): "查询从节点信息",
    (0xE8, 0x04, 0x03, 0x06): "返回查询从节点信息",
    (0xE8, 0x00, 0x03, 0x07): "查询从节点主动注册进度",
    (0xE8, 0x03, 0x03, 0x08): "查询从节点的父节点",
    (0xE8, 0x04, 0x03, 0x08): "返回查询从节点的父节点",
    (0xE8, 0x00, 0x03, 0x09): "查询映射表从节点数量",
    (0xE8, 0x03, 0x03, 0x0A): "查询从节点通信地址映射表",
    (0xE8, 0x04, 0x03, 0x0A): "返回查询从节点通信地址映射表",
    (0xE8, 0x00, 0x03, 0x0B): "查询任务建议超时时间",
    (0xE8, 0x03, 0x03, 0x0C): "查询从节点相位信息",
    (0xE8, 0x04, 0x03, 0x0C): "返回查询从节点相位信息",
    (0xE8, 0x03, 0x03, 0x0D): "批量查询从节点相位信息",
    (0xE8, 0x04, 0x03, 0x0D): "返回批量查询从节点相位信息",
    (0xE8, 0x03, 0x03, 0x0E): "查询表档案的台区识别结果",
    (0xE8, 0x04, 0x03, 0x0E): "返回查询表档案的台区识别结果",
    (0xE8, 0x03, 0x03, 0x0F): "查询多余节点的台区识别结果",
    (0xE8, 0x04, 0x03, 0x0F): "返回查询多余节点的台区识别结果",
    (0xE8, 0x02, 0x04, 0x01): "设置主节点地址",
    (0xE8, 0x02, 0x04, 0x02): "添加从节点",
    (0xE8, 0x02, 0x04, 0x03): "删除从节点",
    (0xE8, 0x02, 0x04, 0x04): "允许/禁止上报从节点事件",
    (0xE8, 0x02, 0x04, 0x05): "激活从节点主动注册",
    (0xE8, 0x02, 0x04, 0x06): "终止从节点主动注册",
    (0xE8, 0x02, 0x04, 0x07): "添加从节点通信地址映射表",
    (0xE8, 0x05, 0x05, 0x01): "上报任务数据",
    (0xE8, 0x05, 0x05, 0x02): "上报从节点事件",
    (0xE8, 0x05, 0x05, 0x03): "上报从节点信息",
    (0xE8, 0x05, 0x05, 0x04): "上报从节点注册结束",
    (0xE8, 0x05, 0x05, 0x05): "上报任务状态",
    (0xE8, 0x06, 0x06, 0x01): "请求/返回集中器时间",
    (0xE8, 0x02, 0x07, 0x01): "启动文件传输",
    (0xE8, 0x02, 0x07, 0x02): "传输文件内容",
    (0xE8, 0x00, 0x07, 0x03): "查询文件信息",
    (0xE8, 0x00, 0x07, 0x04): "查询文件处理进度",
    (0xE8, 0x03, 0x07, 0x04): "查询文件传输失败节点(汇总表DI0=04, 见6.8.8可能为05)",
    (0xE8, 0x04, 0x07, 0x04): "返回查询文件传输失败节点(同上不一致)",
    (0xE8, 0x03, 0x07, 0x05): "查询文件传输失败节点(详细条款6.8.8)",
    (0xE8, 0x04, 0x07, 0x05): "返回查询文件传输失败节点(详细条款6.8.8)",
    (0xE8, 0x03, 0x10, 0x10): "查询从节点邻居表",
    (0xE8, 0x04, 0x10, 0x10): "返回查询从节点邻居表",
    (0xE8, 0x03, 0x10, 0x11): "查询主节点状态",
    (0xE8, 0x04, 0x10, 0x11): "返回主节点状态",
    (0xE8, 0x03, 0x10, 0x12): "读取入网节点信息",
    (0xE8, 0x04, 0x10, 0x12): "返回入网节点信息",
    (0xE8, 0x03, 0x10, 0x13): "读取未入网节点信息",
    (0xE8, 0x04, 0x10, 0x13): "返回未入网节点信息",
    (0xE8, 0x02, 0x10, 0x14): "触发指定节点网络维护",
    (0xE8, 0x03, 0x10, 0x15): "请求切换通信速率和信道",
    (0xE8, 0x04, 0x10, 0x15): "响应切换通信速率和信道",
    (0xE8, 0x02, 0x10, 0x16): "恢复通信速率和信道",
    # 采集器接口 (EA)
    (0xEA, 0x01, 0x00, 0x01): "确认",
    (0xEA, 0x01, 0x00, 0x02): "否认",
    (0xEA, 0x06, 0x21, 0x01): "请求表地址个数",
    (0xEA, 0x04, 0x21, 0x02): "请求表地址",
    (0xEA, 0x03, 0x21, 0x02): "返回表地址",
    (0xEA, 0x06, 0x21, 0x03): "请求采集器地址",
    (0xEA, 0x05, 0x21, 0x04): "电表探测列表",
    (0xEA, 0x06, 0x21, 0x05): "电表探测状态",
    (0xEA, 0x06, 0x21, 0x06): "请求模块当前通信地址/请求采集器绑定地址",
    (0xEA, 0x04, 0x22, 0x01): "透传上行数据到采集器",
    (0xEA, 0x03, 0x22, 0x01): "透传上行数据到采集器应答",
    (0xEA, 0x00, 0x23, 0x01): "查询厂商代码和版本信息",
    (0xEA, 0x05, 0x24, 0x01): "启动文件传输",
    (0xEA, 0x05, 0x24, 0x02): "传输文件内容",
    (0xEA, 0x06, 0x24, 0x03): "请求文件信息",
    (0xEA, 0x06, 0x24, 0x04): "请求文件处理进度",
    (0xEA, 0x06, 0x25, 0x01): "查询设备类型",
    (0xEA, 0x06, 0x31, 0x01): "请求映射表通信地址个数",
    (0xEA, 0x04, 0x31, 0x02): "请求映射表通信地址",
    (0xEA, 0x03, 0x31, 0x02): "返回映射表通信地址",
    (0xEA, 0x05, 0x31, 0x04): "映射探测列表",
    (0xEA, 0x06, 0x31, 0x05): "映射探测状态",
}

CONFIRM_DENY_DI0 = {0x01, 0x02}  # any (xx, xx, 00, 01/02) is confirm/deny


def clean_hex(s: str) -> bytes:
    s = s.strip()
    for junk in ("0x", "0X", "H", "h", ","):
        s = s.replace(junk, " ")
    s = "".join(s.split())
    if len(s) % 2:
        raise ValueError(f"Odd number of hex digits after cleanup: {s!r}")
    return bytes.fromhex(s)


def decode_control_byte(c: int) -> dict:
    return {
        "raw": f"0x{c:02X}",
        "DIR": (c >> 7) & 0x01,
        "DIR_meaning": "上行(module->concentrator/collector)" if (c >> 7) & 1 else "下行(concentrator/collector->module)",
        "PRM": (c >> 6) & 0x01,
        "PRM_meaning": "启动站(initiator)" if (c >> 6) & 1 else "从动站(responder)",
        "ADD": (c >> 5) & 0x01,
        "ADD_meaning": "带地址域" if (c >> 5) & 1 else "不带地址域",
        "VER": (c >> 3) & 0x03,
        "reserved": c & 0x07,
    }


def checksum_of(payload: bytes) -> int:
    """payload = control byte + 用户数据区 (address if present + AFN + SEQ + DI + data)."""
    return sum(payload) & 0xFF


def parse_frame(raw: bytes, length_little_endian: bool = True) -> dict:
    if len(raw) < 8:
        raise ValueError("Frame too short to be valid (need at least start+L+C+AFN+SEQ+DI+CS+end).")
    if raw[0] != 0x68:
        raise ValueError(f"Expected start byte 0x68, got 0x{raw[0]:02X}")
    if raw[-1] != 0x16:
        raise ValueError(f"Expected end byte 0x16, got 0x{raw[-1]:02X}")

    l_bytes = raw[1:3]
    L = int.from_bytes(l_bytes, "little" if length_little_endian else "big")

    result = {"total_bytes_in_input": len(raw), "L_field": L, "L_field_bytes_order": "little-endian" if length_little_endian else "big-endian"}
    if L != len(raw):
        result["length_warning"] = (
            f"L field ({L}) does not match actual total frame length ({len(raw)}). "
            "Try --big-endian-length, double check for stray whitespace, or the frame may be truncated/corrupted."
        )

    c_byte = raw[3]
    control = decode_control_byte(c_byte)
    result["control_byte"] = control

    idx = 4
    address = None
    if control["ADD"] == 1:
        if len(raw) < idx + 12:
            raise ValueError("ADD=1 but not enough bytes remain for a 12-byte address domain.")
        asrc = raw[idx: idx + 6]
        adst = raw[idx + 6: idx + 12]
        address = {"ASRC": asrc.hex().upper(), "ADST": adst.hex().upper()}
        idx += 12
    result["address"] = address

    if len(raw) < idx + 6:  # AFN(1)+SEQ(1)+DI(4) at minimum
        raise ValueError("Not enough bytes remain for AFN+SEQ+DI.")
    afn = raw[idx]
    seq = raw[idx + 1]
    di = raw[idx + 2: idx + 6]
    idx += 6

    data = raw[idx: -2]  # everything up to CS
    cs_byte = raw[-2]

    di_tuple = tuple(di)
    di_name = DI_NAMES.get(di_tuple)
    if di_name is None and di_tuple[2] == 0x00 and di_tuple[3] in CONFIRM_DENY_DI0:
        di_name = "确认" if di_tuple[3] == 0x01 else "否认"

    result["AFN"] = f"0x{afn:02X}"
    result["SEQ"] = seq
    result["DI"] = "".join(f"{b:02X} " for b in di).strip()
    result["DI3_role"] = {0xE8: "集中器与本地模块通信", 0xEA: "采集器与本地模块通信"}.get(di[0], "未知(既非E8也非EA)")
    result["command_name"] = di_name or "未在内置表中找到 -- 查 references/afn-di-codes.md"
    result["data_content_hex"] = data.hex().upper()
    result["data_content_len"] = len(data)

    # checksum verification: control byte + address(if any) + AFN + SEQ + DI + data
    user_area = bytes([c_byte]) + (raw[4:4 + 12] if address else b"") + bytes([afn, seq]) + di + data
    computed_cs = checksum_of(user_area)
    result["CS_field"] = f"0x{cs_byte:02X}"
    result["CS_computed"] = f"0x{computed_cs:02X}"
    result["checksum_ok"] = computed_cs == cs_byte

    # decode ack/nack error code if this is a 否认 frame
    if di_tuple[2] == 0x00 and di_tuple[3] == 0x02 and len(data) >= 1:
        result["nack_error_code"] = data[0]

    return result


def build_frame(afn: int, di: bytes, seq: int, direction_up: bool, prm_initiator: bool,
                 address: bytes, data: bytes, length_little_endian: bool = True) -> bytes:
    add_bit = 1 if address else 0
    c = (int(direction_up) << 7) | (int(prm_initiator) << 6) | (add_bit << 5)
    user_area = bytes([c]) + (address or b"") + bytes([afn, seq]) + di + data
    l1 = len(user_area) - 1  # minus control byte itself is NOT subtracted; L1 = whole user-data region excluding C
    # Per spec: L = L1(用户数据区, i.e. address+AFN+SEQ+DI+data, NOT including C) + 6
    l1_correct = len(user_area) - 1  # user_area currently includes C; strip it for L1
    l1_correct = len(address or b"") + 2 + len(di) + len(data)
    total_len = l1_correct + 6
    cs = checksum_of(user_area)
    frame = bytes([0x68]) + total_len.to_bytes(2, "little" if length_little_endian else "big") + user_area + bytes([cs, 0x16])
    return frame


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("hexstr", nargs="?", help="Frame as a hex string (spaces/0x/H optional)")
    ap.add_argument("--hex", dest="hexstr2", help="Alternative way to pass the hex string")
    ap.add_argument("--big-endian-length", action="store_true", help="Treat the 2-byte L field as big-endian instead of the spec default little-endian")
    ap.add_argument("--build", action="store_true", help="Build mode instead of parse mode")
    ap.add_argument("--afn", help="(build) AFN byte, hex, e.g. 01")
    ap.add_argument("--di", help="(build) DI, 4 hex bytes space-separated, e.g. 'E8 02 01 03'")
    ap.add_argument("--seq", help="(build) SEQ byte, hex, e.g. 00")
    ap.add_argument("--dir", choices=["up", "down"], default="down", help="(build) DIR bit")
    ap.add_argument("--prm", choices=["0", "1"], default="1", help="(build) PRM bit")
    ap.add_argument("--address", help="(build) 12-byte address domain ASRC+ADST as hex, omit for no address domain")
    ap.add_argument("--data", default="", help="(build) data content as hex string")
    args = ap.parse_args()

    little_endian = not args.big_endian_length

    if args.build:
        afn = int(args.afn, 16)
        di = clean_hex(args.di)
        seq = int(args.seq, 16)
        address = clean_hex(args.address) if args.address else b""
        data = clean_hex(args.data) if args.data else b""
        frame = build_frame(afn, di, seq, args.dir == "up", args.prm == "1", address, data, little_endian)
        print(frame.hex().upper())
        return

    hexstr = args.hexstr or args.hexstr2
    if not hexstr:
        ap.error("Provide a hex frame string (positional or --hex), or use --build.")
    raw = clean_hex(hexstr)
    result = parse_frame(raw, little_endian)
    for k, v in result.items():
        print(f"{k}: {v}")


if __name__ == "__main__":
    main()

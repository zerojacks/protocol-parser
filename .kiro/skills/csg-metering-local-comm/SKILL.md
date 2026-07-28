---
name: csg-metering-local-comm
description: Parse, decode, build, and validate frames of the China Southern Power Grid (中国南方电网) standard Q/CSG1209021-2019 "计量自动化终端本地通信模块接口协议" (metering automation terminal local communication module interface protocol) — the protocol between a 集中器 (concentrator) or 采集器 (collector) and its local communication module (电力线载波/微功率无线/以太网 modules). Use this whenever the user pastes/uploads a hex string or byte sequence that looks like a frame starting with 68H and ending with 16H in this domain, mentions "Q/CSG1209021", "本地通信模块接口协议", "集中器/采集器本地通信", asks to decode/parse/analyze a frame from this protocol, asks about AFN (应用功能码), DI (数据标识编码, e.g. codes starting with E8 or EA), the frame checksum CS, or asks to build a command frame (添加任务、查询从节点信息、启动文件传输等) for this protocol. Do NOT use for DL/T645 meter frames (68H...68H...16H with two 68H — that's a different protocol, see the dlt645-2007 skill), Modbus, IEC 103/104, or DLMS/COSEM.
---

# CSG1209021-2019 本地通信模块接口协议

This skill lets Claude parse and construct frames for Q/CSG1209021-2019, the interface protocol between
a concentrator (集中器) or collector (采集器) and its pluggable local communication module (载波/微功率
无线/以太网). It is **not** DL/T645 (no double 68H, no BCD data field with ±33H offset) — if the frame
starts `68 .. .. .. 68` with two start characters, that is DL/T645; use the dlt645-2007 skill instead.

## Workflow for parsing a frame

1. Normalize the input to a flat byte array (strip spaces/`0x`/`H` suffixes, uppercase hex).
2. Run `scripts/parse_frame.py` (see below) to split the frame into fields and verify the checksum.
3. Look up the AFN + DI tuple in `references/afn-di-codes.md` to get the human-readable command name.
4. Look up the exact data-content layout for that DI in `references/data-content-formats.md` to decode
   the payload field-by-field.
5. Report to the user: direction (up/down), node roles, command name, decoded fields, and whether the
   checksum was valid. If the checksum fails, say so explicitly — per the spec the frame should be
   discarded by a real receiver.

For **building** a frame, do the same steps in reverse: find the DI in the reference tables, assemble the
payload bytes per its format, then use `scripts/parse_frame.py --build` (or hand-roll using the checksum
rule below) to produce the full frame including L and CS.

## Frame structure (§5.3)

```
68H | L (2B) | C (1B) | [A: ASRC(6B)+ADST(6B)] | AFN(1B) | SEQ(1B) | DI(4B) | 数据标识内容(变长) | CS(1B) | 16H
      \_____________________________ 用户数据区 (length = L1) ______________________________/
```

- **68H** — start character (fixed).
- **L** — 2 bytes, BIN. Total frame length = `L1 + 6`, where L1 = length of the 用户数据区 (地址域 A +
  AFN + SEQ + DI + 数据标识内容), and 6 = the fixed bytes (68H, the 2 L bytes, C, CS, 16H). So `L` equals
  the **total byte count of the whole frame**. Multi-byte BIN fields in this protocol are transmitted low
  byte first (little-endian) per §5.2 ("低字节在前，高字节在后"), unless the field is a fixed-order
  address/BCD example, which is documented byte-for-byte in the spec and reference tables — when a
  reference table doesn't explicitly reverse an address, treat it as written left-to-right.
- **C** — control byte, one byte, bit layout (D7 = MSB):
  | D7 | D6 | D5 | D4-D3 | D2-D0 |
  |----|----|----|-------|-------|
  | DIR | PRM | ADD | VER | 保留(reserved, =0) |
  - **DIR**: 0 = 下行 (downlink, from 集中器/采集器 to module), 1 = 上行 (uplink, from module).
  - **PRM**: 1 = frame from the initiating (启动) station, 0 = from the responding (从动) station.
  - **ADD**: 1 = frame carries an address domain A, 0 = no address domain.
  - **VER**: protocol version, currently always 0.
- **A (地址域)** — only present when ADD=1. `ASRC` (6B) + `ADST` (6B), both BIN. Broadcast address =
  `99 99 99 99 99 99H`. Downlink: ASRC = concentrator/collector-side MAC; uplink: ADST = same. See
  §6.2 for the exact source/destination convention per node type.
- **AFN** — 1 byte, application function code. Different tables for concentrator-role frames (集中器,
  DI3=E8) vs collector-role frames (采集器, DI3=EA) — see `references/afn-di-codes.md`.
- **SEQ** — 1 byte, frame sequence number 0–255, cyclic; matches uplink/downlink request-response pairs.
- **DI (数据标识编码)** — 4 bytes: `DI3 DI2 DI1 DI0`, each a hex byte.
  - **DI3**: `E8` = 集中器与本地模块通信 (concentrator↔module), `EA` = 采集器与本地模块通信
    (collector↔module).
  - **DI2**: message direction/content type:
    | DI2 | Meaning |
    |-----|---------|
    | 00 | up & down both used, downlink has no data content |
    | 01 | up & down both used, same data-content format both ways |
    | 02 | downlink only; the corresponding uplink is a 确认/否认 (ack/nack) frame |
    | 03 | downlink only, carries data; the matching uplink DI2 is 04 |
    | 04 | uplink only, carries data; the matching downlink DI2 is 03 |
    | 05 | uplink only; the corresponding downlink is a 确认/否认 frame |
    | 06 | up & down both used, uplink has no data content |
  - **DI1**: matches the AFN value.
  - **DI0**: sub-function code distinguishing commands within the same AFN.
- **数据标识内容** — the actual payload, variable length, format defined per DI — see
  `references/data-content-formats.md`.
- **CS** — checksum, 1 byte: arithmetic sum of every byte in **C + 用户数据区** (i.e. control byte through
  the end of the data content, NOT including 68H/L/CS/16H itself), truncated mod 256 (overflow ignored).
- **16H** — end character (fixed).

## Byte-level UART framing (§5.2, informational — not part of the logical frame you're given as hex)

Each on-wire byte also carries 1 start bit, 8 data bits, 1 even-parity bit, 1 stop bit at 9600bps (or
115200bps). This only matters for physical-layer questions, not for decoding a hex frame the user pastes.

## Link transmission service classes (§5.4)

- **S1 发送/无回答**: fire-and-forget, no reply expected.
- **S2 发送/确认**: initiator sends a command, responder replies 确认(ack)/否认(nack).
- **S3 请求/响应**: initiator requests, responder replies ack/nack/data. If no reply within 10s, the
  initiator resends (at least 2 retries).

## Common decode pitfall checklist

- Confirm DI3 is `E8` or `EA` before picking which AFN table to use — the same AFN byte value can mean
  different things depending on node role (e.g. AFN=21H only exists for 采集器/EA).
- A frame's uplink/downlink direction is given by control-byte DIR, **not** by which side captured it.
- 确认/否认 (ack/nack) replies always use DI = `xx 01 00 01` (确认) or `xx 01 00 02` (否认) regardless of
  which command they're acknowledging — the SEQ byte is what ties them back to the original command.
- 否认 frames carry a 1-byte 错误状态字(error code) — decode it via the table in
  `references/afn-di-codes.md`.
- Some AFN/DI listings in the source standard have small internal inconsistencies between the summary
  table and the detailed clause (e.g. AFN=07H "查询文件传输失败节点" is listed as DI0=04 in the summary
  table §6.4.1 but DI0=05 in the detailed clause §6.8.8). When you hit a mismatch, say so rather than
  silently picking one, and prefer the detailed per-AFN clause value (documented in
  `references/data-content-formats.md`) since that is used to build the actual data-content layout.

## Reference files

- `references/afn-di-codes.md` — full AFN + DI code tables for 集中器 (E8), 采集器 (EA), 采集器红外接口,
  无线维护接口, plus the 确认/否认 error-code tables. Read this to name a command from its AFN/DI bytes.
- `references/data-content-formats.md` — byte-by-byte data-content layout for every DI listed above,
  including worked examples (e.g. 从节点通信地址映射表, 从节点相位信息, 请求集中器时间). Read this to
  actually decode/build the payload bytes once you know which DI you're looking at.
- `scripts/parse_frame.py` — run this first on any hex frame. It splits the frame into 68H/L/C/A(if
  present)/AFN/SEQ/DI/data/CS/16H, verifies the checksum, decodes the control byte, and looks up the
  AFN/DI name from an embedded table. Use `--build` mode to assemble a frame from field values (it
  computes L and CS for you).

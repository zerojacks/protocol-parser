---
name: dlt645-2007
description: Parse, decode, build, or validate DL/T 645-2007 frames — the Chinese national standard communication protocol for multi-function watt-hour (electricity) meters (多功能电能表通信协议). Use this whenever the user pastes/uploads a hex string or byte sequence that looks like a 645 meter frame (typically starts with 68H ... 68H ... 16H), asks to decode/parse/analyze a "645报文"/"645帧"/"电表报文", asks to build a read/write/broadcast-time-sync/freeze/clear-demand/change-baud/change-password command frame for a DL/T645 meter, asks about DI3DI2DI1DI0 data identifiers, control codes (控制码), the +33H/-33H data offset, or meter checksum (CS) calculation. Also use for DL/T 645-1997 frames (same frame skeleton, narrower control-code/DI table — note differences when relevant). Do NOT use for Modbus, IEC 61850/103/104, DLMS/COSEM, or other meter protocols — this skill is specific to DL/T 645.
---

# DL/T 645-2007 多功能电能表通信协议

This skill lets Claude decode a raw DL/T 645-2007 frame (hex bytes) into a
human-readable structure, or encode a request frame from a stated intent
(read energy, write time, freeze, clear demand, change address, etc.).

Full data-identifier code tables (电能量/最大需量/变量/事件/参变量/冻结量/负荷记录)
are in `references/data-identifiers.md` — read that file whenever the frame's
DI3DI2DI1DI0 needs to be resolved to a data item name, and it isn't one of the
common ones already listed inline below. Status-word / feature-word /
error-word bit tables (电表运行状态字, 通信速率特征字, 错误信息字 ERR, etc.)
are in `references/status-and-feature-words.md`.

## 1. Overall frame skeleton

Every frame (both directions) has 7 fields, transmitted **byte by byte, low
byte first** within multi-byte fields:

```
68H | A0 A1 A2 A3 A4 A5 | 68H | C | L | DATA (L bytes) | CS | 16H
     └── 地址域 6 bytes ──┘
```

| Field | Name | Size | Notes |
|---|---|---|---|
| 68H | 帧起始符 (start) | 1 | fixed 0x68 |
| A0..A5 | 地址域 (address) | 6 | 6 bytes, each 2-digit BCD → 12-digit decimal address. Low byte (A0) transmitted first. Padded with leading `0` (i.e. high-order bytes = 00) if the real address is shorter than 12 digits. |
| 68H | 帧起始符 (repeated) | 1 | fixed 0x68 |
| C | 控制码 (control code) | 1 | see §3 |
| L | 数据域长度 (data length) | 1 | byte count of DATA; read: L≤200, write: L≤50; L=0 → no DATA field |
| DATA | 数据域 | L | content depends on C; see §5 and per-command sections; every byte here (except in broadcast-time/freeze plain time fields — see note) is transmitted with **+33H added** by the sender and **-33H subtracted** by the receiver (see §4) |
| CS | 校验码 (checksum) | 1 | sum (mod 256) of every byte from the *first* 0x68 up to (not including) CS itself |
| 16H | 帧结束符 (end) | 1 | fixed 0x16 |

Special addresses:
- `999999999999H` (all-9s BCD) = **broadcast address**, valid only for broadcast commands (broadcast time-sync 08H, broadcast freeze). Broadcast frames get **no slave response**.
- **Wildcard / abbreviated addressing**: from some low-order digit upward, remaining high digits may be set to `AAH` as a "don't care" wildcard for read operations; the slave's response frame returns its *actual* full address.

Before the 68H start byte, the master sends **4 preamble bytes `FEH FEH FEH FEH`** to wake up the slave (§ "前导字节" — not counted in checksum or length).

## 2. Byte/serial format (physical layer)

Each on-wire byte = 1 start bit (0) + 8 data bits (D0 first / LSB first) + 1 even parity bit + 1 stop bit (1) = 11 bits total. Standard baud rates: 600/1200/2400/9600/19200 bps (default/缺省 2400bps for infrared, 1200bps for modulated infrared, 2400bps for RS-485).

## 3. Control code C (1 byte) — bit layout

```
 D7        D6              D5              D4~D0
direction  slave-resp flag continuation    function code
```

- **D7** — 0 = command frame from master, 1 = response frame from slave
- **D6** — slave response status: 0 = normal (正确应答), 1 = abnormal (异常应答)
- **D5** — 1 = has-following-data-frame (有后续数据帧, i.e. more data to come — used with "read follow-on data" command 12H), 0 = no follow-on frame
- **D4~D0** — function code, see table below

### Function code table (D4~D0) and resulting control-code bytes

| Func (D4-D0) | Meaning | Master req C | Slave OK C (no/with follow-up) | Slave error C |
|---|---|---|---|---|
| 00000 | 保留 reserved | — | — | — |
| 01000 | 广播校时 broadcast time-sync | 08H | (no response) | — |
| 10001 | 读数据 read data | 11H | 91H / B1H | D1H |
| 10010 | 读后续数据 read follow-on data | 12H | 92H / B2H | D2H |
| 10011 | 读通信地址 read comm. address | 13H | 93H | (no response on error) |
| 10100 | 写数据 write data | 14H | 94H | D4H |
| 10101 | 写通信地址 write comm. address | 15H | 95H | (no response on error) |
| 10110 | 冻结命令 freeze command | 16H | 96H | D6H |
| 10111 | 更改通信速率 change baud rate | 17H | 97H | D7H |
| 11000 | 修改密码 change password | 18H | 98H | D8H |
| 11001 | 最大需量清零 clear max demand | 19H | 99H | D9H |
| 11010 | 电表清零 clear meter | 1AH | 9AH | DAH |
| 11011 | 事件清零 clear event records | 1BH | 9BH | DBH |

Any function code not in this table (or a mismatched checksum/parity) → the
slave silently drops the frame (does not respond).

## 4. Data-domain byte transformation (+33H / -33H)

The **DATA field only** (not address, not control code, not length, not
checksum) is transmitted with every byte **increased by 0x33** by the sender,
and the receiver **subtracts 0x33** from every byte to recover the real
value. This is why raw captured DATA bytes look "off" from the plain
BCD/ASCII value — always undo this before interpreting DATA content
(including the DI3DI2DI1DI0 identifier bytes, BCD data values, password
bytes, operator-code bytes, etc.). Example: to send the value 0x12 you
actually put 0x45 on the wire (0x12+0x33); a byte read off the wire as 0xAB
represents 0xAB-0x33 = 0x78.

## 5. Checksum (CS)

CS = arithmetic sum, modulo 256 (i.e. take only the low byte, discard
carry/overflow), of every byte starting from the **first** 0x68 start byte
through the end of the DATA field (i.e. everything except CS and 16H itself).
To validate a received frame, recompute this sum and compare to the CS byte
present; a mismatch means the frame must be discarded (no response is sent
by a real slave in that case, but when *you* are just parsing/decoding a
frame for the user, still report the mismatch).

## 6. Response timing / transmission rules (informational, rarely needed for parsing)

- Response delay Td: 20ms ≤ Td ≤ 500ms after the request.
- Inter-byte gap Tb ≤ 500ms.
- "读后续数据" (read follow-on data, func 12H) uses a 1-byte frame sequence number SEQ (1~255, incrementing) appended after the data identifier(s), to guard against duplicate/lost frames; the slave echoes the same SEQ back.

## 7. Data identifier (DI) structure

Every data item or data block is addressed by a 4-byte identifier
`DI3 DI2 DI1 DI0` (DI0 lowest, transmitted first, like any other multi-byte
field — low byte first). Data type categories, by DI3:

| DI3 | Category |
|---|---|
| 00 | 电能量 Energy (kWh/kvarh/kVAh totals & rates & historical settlement days) |
| 01 | 最大需量及发生时间 Max demand & occurrence time |
| 02 | 变量 Instantaneous variables (voltage, current, power, power factor, THD, temperature, etc.) |
| 03 | 事件记录 Event records (sag/swell/loss of voltage or current, phase reversal, programming logs, cover-open, etc.) |
| 04 | 参变量 Parameters (date/time, meter constants, rate/time-of-use tables, passwords, display config, etc.) |
| 05 | 冻结量 Freeze data (timed/instantaneous/TOU-switch freeze snapshots) |
| 06 | 负荷记录 Load profile |

**Data-block wildcard rule**: if DI2, DI1, or DI0 (never DI3) = `FFH`, that
byte's whole dimension is treated as "all values" and the identifier now
addresses a **data block** (a whole family of related data items returned
together) rather than one item. E.g. `00 01 00 FF` = 正向有功总电能数据块
(block containing current + all settlement-day forward active total
energies); `00 01 FF 00` = current forward-active energy across total +
all rates (block by tariff/rate).

Most **data items** use compressed BCD encoding (e.g. an energy value
`XXXXXX.XX` kWh occupies 4 bytes BCD, 2 decimal digits per byte, low byte
first). A handful of string-type items (meter model, manufacture date,
asset number, protocol version, comm-speed feature bytes context) use plain
ASCII instead — see the notes in `references/data-identifiers.md`.

### Frequently-used identifiers (memorize these; consult the reference file for the rest)

| DI3 DI2 DI1 DI0 | Item | Format | Bytes |
|---|---|---|---|
| 00 00 00 00 | 当前正向有功组合总电能 (current combined active total energy) — actually 00 00 corresponds to combined; see table | XXXXXX.XX kWh | 4 |
| 00 01 00 00 | 当前正向有功总电能 current forward active total energy | XXXXXX.XX kWh | 4 |
| 00 02 00 00 | 当前反向有功总电能 current reverse active total energy | XXXXXX.XX kWh | 4 |
| 02 01 01/02/03/FF | A/B/C 相电压 phase voltage (block=FF) | XXX.X V | 2 |
| 02 02 01/02/03/FF | A/B/C 相电流 phase current (block=FF) | XXX.XXX A | 3 |
| 02 03 00 | 瞬时总有功功率 total instantaneous active power | XX.XXXX kW (sign in top nibble) | 3 |
| 04 00 01 02 | 时间 (time) hh:mm:ss | 3 | 
| 04 00 01 01 | 日期及星期 date & weekday YYMMDDWW | 4 |

## 8. Step-by-step decode algorithm (use this to parse any raw frame)

1. Strip any leading `FE FE FE FE` preamble bytes.
2. Confirm frame starts `68H` and, 8 bytes later, another `68H` (i.e. bytes[0]==0x68 and bytes[7]==0x68). If not, the capture may include multiple frames or garbage — locate the real 68...68 pair.
3. Bytes[1..6] = address A0..A5. Each byte is 2-digit BCD; **reverse the byte order** (A5 A4 A3 A2 A1 A0) and read each byte as two decimal digits to get the 12-digit address string. Check for all-`AA` (wildcard) or all-`99`/broadcast pattern.
4. Byte[8] = control code C. Decode D7 (direction), D6 (error flag, only meaningful if D7=1), D5 (continuation flag), D4-D0 (function, via §3 table).
5. Byte[9] = L (data length, decimal value of the byte, not BCD).
6. Bytes[10 .. 10+L-1] = DATA. Subtract 0x33 from every byte first (§4).
   - If C indicates an **error response** (D6=1, e.g. D1H/D2H/D4H/D6H-DBH/D7H/D8H/D9H), DATA is exactly 1 byte = ERR bit-field — decode via `references/status-and-feature-words.md` (错误信息字 ERR table): bit0 其他错误, bit1 无请求数据, bit2 密码错/未授权, bit3 通讯速率不能更改, bit4 年时区数超, bit5 日时段数超, bit6 费率数超, bit7 保留.
   - Otherwise, DATA's first 4 bytes (after de-offsetting) are usually the DI3DI2DI1DI0 identifier (low byte first: byte0=DI0, byte1=DI1, byte2=DI2, byte3=DI3) for read/write/freeze-style commands — reverse to get DI3DI2DI1DI0, then look it up (§7 / reference file) to know the value's format and remaining byte layout. For multi-item requests (m>0) additional 4-byte identifiers follow. For responses, after the identifier(s) come the actual N1..Nm value bytes in the format the DI table specifies (still BCD/ASCII, low byte first within each value).
   - For **write** (14H) and analogous "requires password" commands, DATA = `DI0DI1DI2DI3` + 4-byte password (with 1 high nibble = 权限 permission level PA, remaining P0P1P2) + 4-byte operator code (C0C1C2C3) + actual data.
   - For **broadcast time sync** (08H), DATA = 6 bytes `ss mm hh DD MM YY` (seconds..year, each BCD, low field first) — no DI identifier, no +33H password wrapper needed beyond the standard data-domain offset.
   - For **freeze command** (16H), DATA = 4 bytes `mm hh DD MM` (minute/hour/day/month), or the special all-`99`/`FF` patterns meaning periodic freeze (see §"7.7" in the standard: 99DDhhmm=按月周期, 9999hhmm=按日周期, 999999mm=按小时周期, 99999999=瞬时冻结).
7. Byte[10+L] = CS. Recompute checksum per §5 over bytes[0..10+L-1] and compare.
8. Last byte = 16H (frame end) — verify.

## 9. Step-by-step encode algorithm (build a request frame)

1. Pick control code C per §3 for the desired operation.
2. Build DATA in plain (real) values first (identifier bytes low-to-high = DI0,DI1,DI2,DI3; then any password/operator-code/value bytes per the command's layout in §8 or the standard's §7.x sections), THEN add 0x33 to every DATA byte.
3. L = length of that (already-offset) DATA in bytes.
4. Address: convert the 12-digit decimal address to 6 BCD bytes, then reverse the byte order for transmission (A0 = lowest-order pair first).
5. Assemble: `68H, A0..A5, 68H, C, L, DATA..., CS, 16H` where CS = sum(mod 256) of bytes from the first 68H through the end of DATA.
6. Optionally prepend `FE FE FE FE` wake-up preamble.

## 10. Worked example

Request to read the current forward active total energy (DI=00 01 00 00)
from meter address `123456789012`:

- Address BCD bytes (12-digit → 6 bytes, digit pairs): 12,34,56,78,90,12 → transmitted low-byte-first as A0..A5 = `12 90 78 56 34 12`
- C = 11H (read data, master request)
- DATA (plain) = DI0 DI1 DI2 DI3 = `00 00 01 00` → add 0x33 each → `33 33 34 33`
- L = 04H
- Frame so far: `68 12 90 78 56 34 12 68 11 04 33 33 34 33`
- CS = sum of all bytes above, mod 256
- Final: `68 12 90 78 56 34 12 68 11 04 33 33 34 33 <CS> 16`

A normal slave response (no follow-on data) would use C=91H and DATA =
same DI (offset) + the 4-byte BCD energy value (also +33H offset each byte).

## 11. DL/T 645-1997 differences (if the user says "1997" or the frame doesn't match 2007 patterns)

The 1997 version uses the same 68H...16H frame skeleton and +33H data
offset, but: 2-byte (not 4-byte) data identifiers, no 读通信地址/写通信地址/冻结/电表清零/事件清零/更改通信速率 control codes (those were added in 2007), and no password-verification/operator-code requirement on write-type commands. If asked to parse a 1997 frame, flag the narrower DI width and control-code set explicitly rather than assuming 2007 semantics.

## 12. When the reference tables don't cover an identifier

If a DI doesn't match anything in this file or `references/data-identifiers.md`,
say so plainly rather than guessing — report the raw DI bytes, the category implied by DI3, and note it may be a manufacturer-specific extension (04 80 00.. range is explicitly manufacturer-defined: 厂家软件版本号/厂家硬件版本号/厂家编号) or a table not yet transcribed into the reference file.

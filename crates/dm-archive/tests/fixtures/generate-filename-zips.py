"""Generate synthetic stored ZIPs with explicit filename bytes/flags (issue #50)."""
from pathlib import Path
import struct
import zlib

ROOT = Path(__file__).parent


def write_zip(filename, entries):
    output = bytearray()
    central = bytearray()
    for raw, flags, extra in entries:
        data = b"hello"
        crc = zlib.crc32(data)
        offset = len(output)
        output += struct.pack(
            "<IHHHHHIIIHH", 0x04034B50, 20, flags, 0, 0, 33,
            crc, len(data), len(data), len(raw), len(extra)
        ) + raw + extra + data
        central += struct.pack(
            "<IHHHHHHIIIHHHHHII", 0x02014B50, 20, 20, flags, 0, 0, 33,
            crc, len(data), len(data), len(raw), len(extra), 0, 0, 0, 0x20, offset
        ) + raw + extra
    offset = len(output)
    output += central
    output += struct.pack(
        "<IHHHHIIH", 0x06054B50, 0, 0, len(entries), len(entries),
        len(central), offset, 0
    )
    (ROOT / filename).write_bytes(output)


write_zip("cp932.zip", [(name.encode("cp932"), 0, b"") for name in (
    "サンプル/説明書.txt", "サンプル/画像フォルダ/表紙.txt", "ソ能十予.txt", "表"
)])
write_zip("unflagged-utf8.zip", [("日本語.txt".encode(), 0, b"")])
# Valid Shift-JIS must not override an explicit (but invalid) UTF-8 declaration.
write_zip("invalid-flagged-utf8.zip", [("表.txt".encode("cp932"), 0x800, b"")])
# An incomplete lead byte: neither UTF-8 nor CP932; fail after a valid entry.
write_zip("invalid-filename.zip", [(b"first.txt", 0, b""), (b"\x81", 0, b"")])
for kind, name in (
    ("traversal", "../表.txt"), ("backslash", "表\\file.txt"),
    ("absolute", "/表.txt"),
):
    write_zip(f"cp932-{kind}.zip", [(name.encode("cp932"), 0, b"")])
raw = "別名.txt".encode("cp932")
name = "正しい名前.txt".encode()
payload = b"\x01" + struct.pack("<I", zlib.crc32(raw)) + name
extra = struct.pack("<HH", 0x7075, len(payload)) + payload
write_zip("unicode-path.zip", [(raw, 0, extra)])

"""float32 WAV in and out, no dependency beyond numpy."""
import struct
import numpy as np

def write(path, frames, rate=48000):
    data = np.asarray(frames, np.float32)
    ch = data.shape[1]
    payload = data.astype("<f4").tobytes()
    fmt = struct.pack("<HHIIHH", 3, ch, rate, rate * ch * 4, ch * 4, 32)
    with open(path, "wb") as f:
        f.write(b"RIFF" + struct.pack("<I", 4 + 8 + len(fmt) + 8 + len(payload)) + b"WAVE")
        f.write(b"fmt " + struct.pack("<I", len(fmt)) + fmt + b"data" + struct.pack("<I", len(payload)) + payload)

def read(path):
    b = open(path, "rb").read()
    assert b[:4] == b"RIFF" and b[8:12] == b"WAVE", path
    i, fmt = 12, None
    while i < len(b):
        cid, size = b[i:i + 4], struct.unpack("<I", b[i + 4:i + 8])[0]
        body = b[i + 8:i + 8 + size]
        if cid == b"fmt ":
            fmt = struct.unpack("<HHIIHH", body[:16])
        elif cid == b"data":
            tag, ch, rate, _, _, bits = fmt
            if tag == 3 and bits == 32:
                x = np.frombuffer(body, "<f4")
            elif tag == 1 and bits == 16:
                x = np.frombuffer(body, "<i2") / 32768.0
            elif tag == 1 and bits == 24:
                a = np.frombuffer(body, np.uint8).reshape(-1, 3).astype(np.int32)
                x = ((a[:, 0] | (a[:, 1] << 8) | (a[:, 2] << 16)) << 8 >> 8) / 8388608.0
            elif tag == 0xFFFE and bits == 32:
                x = np.frombuffer(body, "<f4")
            else:
                raise ValueError(f"{path}: format {fmt}")
            return x.reshape(-1, ch).astype(np.float64), rate
        i += 8 + size + (size & 1)
    raise ValueError(f"{path}: no data chunk")

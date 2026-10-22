"""One song, written against export v0 (Gain=ParamID 2, Tilt=3), played unchanged against each edited
export. The lane is on "2"; Tilt is set to 0.75 (plain +0.5); the synth's Level to 0.25."""
import json, os, shutil, sys
sys.path.insert(0, os.path.dirname(__file__))
import mcp, wav, m1_render
import numpy as np
SP = sys.argv[1]; OUT = f"{SP}/m1"
p = f"{OUT}/matrix.escri"
man0 = m1_render.manifest("v0", ["syn0p", "fx0p"])
s, fx = m1_render.build(p, man0)
synth = next(t for t in s.call("get_song", {})["tracks"].values() if t["name"] == "Synth")
s.call("set_param", {"device_id": synth["instrument"]["id"], "param": "2", "value": 0.25})
s.close()
rows = []
for name, fxv in [("v0", "fx0p"), ("insert", "fx1p"), ("reorder", "fx2p"), ("rename-id", "fx3p"),
                  ("range", "fx5p"), ("remove", "fx6p")]:
    man = m1_render.manifest(name, ["syn0p", fxv])
    s = mcp.Session(p, man, create=False)
    try:
        r = s.call("render_export", {"output_path": f"{OUT}/matrix-{name}.wav"}, ok=False)
        if r.get("valid") is False:
            rows.append((name, "refused", json.dumps(r.get("errors"))[:200])); continue
        y, rate = wav.read(f"{OUT}/matrix-{name}.wav")
        env = [tuple(np.abs(y[int(t * rate) - 200:int(t * rate) + 200]).max(0).round(3)) for t in (1, 2, 3)]
        rows.append((name, "rendered", env))
    except Exception as e:
        rows.append((name, "error", str(e)[:200]))
    finally:
        s.close()
for r in rows: print(*r)

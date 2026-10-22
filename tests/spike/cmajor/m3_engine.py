"""Measurement 3 through core and the real engine: an audio clip into the exported, adapted graph."""
import base64, hashlib, json, os, shutil, sys
sys.path.insert(0, os.path.dirname(__file__))
import mcp, wav, m1_render
import numpy as np
SP = sys.argv[1]; OUT = f"{SP}/m3e"; os.makedirs(OUT, exist_ok=True)
man = m1_render.manifest("m3", ["net_ff", "net_ffw"])
def render(name, clip, effect=None):
    p = f"{OUT}/{name}.escri"; shutil.rmtree(p, ignore_errors=True)
    s = mcp.Session(p, man)
    s.call("add_track", {"name": "T", "kind": "TRACK_KIND_AUDIO"})
    t = next(t for t in s.call("get_song", {})["tracks"].values() if t["name"] == "T")
    if effect:
        s.call("add_effect", {"track_id": t["id"], "ref": {"plugin": {"plugin_id": effect, "version": "1.0"}}})
    h = s.call("add_asset", {"content": base64.b64encode(open(clip, "rb").read()).decode()})["asset_hash"]
    s.call("add_clip", {"track_id": t["id"], "start_tick": 0, "length_ticks": 1920, "audio_clip": {"asset_hash": h}})
    s.call("render_export", {"output_path": f"{OUT}/{name}.wav"})
    s.close()
    y, _ = wav.read(f"{OUT}/{name}.wav")
    return y, hashlib.sha256(open(f"{OUT}/{name}.wav", "rb").read()).hexdigest()[:12]
w = {k: np.array(v, np.float64) for k, v in json.load(open(f"{SP}/m3/models/weights.json")).items()}
f = lambda x, w2: np.tanh(x @ w["W1"] + w["B1"]) @ w2 + w["B2"]
st, si = f"{SP}/m3/stairs.wav", f"{SP}/m3/in2.wav"
dry, _ = render("dry", st)
wet, h1 = render("wet", st, "Escribass/Spike Net")
wet2, h2 = render("wet-again", st, "Escribass/Spike Net")
wetw, _ = render("wet-weights", st, "Escribass/Spike Net W")
dry_s, _ = render("dry-sines", si)
wet_s, _ = render("wet-sines", si, "Escribass/Spike Net")
q = 2 ** -23  # one 24-bit step: the engine writes 24-bit PCM
for name, y, x, w2 in [("wet", wet, dry, w["W2"]), ("wet-weights", wetw, dry, w["W2"] * 0.5), ("wet-sines", wet_s, dry_s, w["W2"])]:
    r = f(x, w2); n = min(len(y), len(r)); e = np.abs(y[:n] - r[:n])
    print(f"{name:12s} frames={len(y)} maxerr={e.max():.2e} ({e.max()/q:.1f} LSB of 24-bit) rms={np.sqrt((y**2).mean()):.4f}")
print("repeat render identical:", h1 == h2, h1, h2)
print("weights change the sound:", not np.array_equal(wet, wetw), "input changes the sound:", not np.array_equal(wet[:1000], wet_s[:1000]))

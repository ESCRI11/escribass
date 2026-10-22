import json, os, shutil, sys, threading, subprocess, time
sys.path.insert(0, os.path.dirname(__file__))
import mcp
SP = sys.argv[1]; OUT = f"{SP}/m1"; case = sys.argv[2]; man = f"{OUT}/manifest-" + os.environ.get("SPIKE_MANIFEST", "fx0") + ".json"
p = f"{OUT}/bisect-{case}.escri"; shutil.rmtree(p, ignore_errors=True)
s = mcp.Session(p, man)
kind = "TRACK_KIND_AUDIO" if case == "fx_only" else "TRACK_KIND_INSTRUMENT"
ref = {"ref": {"plugin": {"plugin_id": "Escribass/Spike Syn", "version": "1.0"}}} if case != "fx_only" else {}
if case == "dexed_fx": ref = {"ref": {"plugin": {"plugin_id": "Digital Suburban/Dexed", "version": "1.0.1"}}}
s.call("add_track", {"name": "T", "kind": kind, **ref})
t = next(t for t in s.call("get_song", {})["tracks"].values() if t["name"] == "T")
if case in ("fx_only", "chain", "dexed_fx"):
    s.call("add_effect", {"track_id": t["id"], "ref": {"plugin": {"plugin_id": "Escribass/Spike Fx", "version": "1.0"}}})
if case == "fx_only":
    import base64
    h = s.call("add_asset", {"content": base64.b64encode(open(f"{SP}/m3/in2.wav", "rb").read()).decode()}); print("asset", json.dumps(h)[:200])
    s.call("add_clip", {"track_id": t["id"], "start_tick": 0, "length_ticks": 3840, "audio_clip": {"asset_hash": h.get("hash") or h.get("asset_hash")}})
else:
    s.call("add_clip", {"track_id": t["id"], "start_tick": 0, "length_ticks": 3840,
                        "note_clip": {"notes": {"n": {"pitch": 69, "start_tick": 0, "length_ticks": 3840, "velocity": 100}}}})
def watchdog():
    time.sleep(60)
    subprocess.run(["pkill", "-f", f"{OUT}/bisect-{case}.wav"], check=False)
    subprocess.run(["pkill", "-x", "escribass_engin"], check=False)
threading.Thread(target=watchdog, daemon=True).start()
t0 = time.time()
try:
    r = s.call("render_export", {"output_path": f"{OUT}/bisect-{case}.wav"})
    print(case, "rendered in", round(time.time() - t0, 2), "s")
except Exception as e:
    print(case, "FAILED after", round(time.time() - t0, 2), "s:", str(e)[:300])
s.close()

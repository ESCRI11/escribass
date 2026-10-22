"""m5.1-spike measurement 1, through core and the real engine: a Cmajor instrument into a Cmajor
effect, with a lane automating the effect's Gain by its ParamID. Renders against one manifest per
variant, so the same song can be played against an edited export."""
import json, os, shutil, sys
sys.path.insert(0, os.path.dirname(__file__))
import mcp

SP = sys.argv[1]; R = mcp.REPO
OUT = f"{SP}/m1"; os.makedirs(OUT, exist_ok=True)

def manifest(name, variants):
    m = json.load(open(f"{R}/engine/build/manifest.json"))
    for v in variants:
        m["plugins"].update(json.load(open(f"{SP}/scan/{v}.json"))["plugins"])
    path = f"{OUT}/manifest-{name}.json"
    json.dump(m, open(path, "w"), indent=1)
    return path

def build(project, man, gain_param="2", tilt_param="3", lane=True):
    if os.path.exists(project): shutil.rmtree(project)
    s = mcp.Session(project, man)
    s.call("add_track", {"name": "Synth", "kind": "TRACK_KIND_INSTRUMENT",
                         "ref": {"plugin": {"plugin_id": "Escribass/Spike Syn", "version": "1.0"}}})
    synth = lambda song: next(t for t in song["tracks"].values() if t["name"] == "Synth")
    track = synth(s.call("get_song", {}))["id"]
    s.call("add_effect", {"track_id": track, "ref": {"plugin": {"plugin_id": "Escribass/Spike Fx", "version": "1.0"}}})
    t = synth(s.call("get_song", {}))
    inst = t["instrument"]["id"]; fx = next(iter(t["fx_chain"]))
    s.call("set_param", {"device_id": inst, "param": "2", "value": 0.25})     # Spike Syn level = 0.25
    s.call("set_param", {"device_id": fx, "param": tilt_param, "value": 0.75}) # tilt plain +0.5
    s.call("add_clip", {"track_id": track, "start_tick": 0, "length_ticks": 7680,
                        "note_clip": {"notes": {"n": {"pitch": 69, "start_tick": 0, "length_ticks": 7680, "velocity": 100}}}})
    if lane:
        s.call("add_automation", {"target": {"device_id": fx, "param": gain_param}, "points": {
            "a": {"tick": 0, "value": 0.0, "curve": "CURVE_LINEAR"},
            "b": {"tick": 7680, "value": 1.0, "curve": "CURVE_LINEAR"}}})
    return s, fx

if __name__ == "__main__":
    what = sys.argv[2]
    man = manifest(what, sys.argv[3].split(","))
    s, fx = build(f"{OUT}/{what}.escri", man)
    r = s.call("render_export", {"output_path": f"{OUT}/{what}.wav"})
    print(what, json.dumps(r)[:300])
    print("close", s.close())

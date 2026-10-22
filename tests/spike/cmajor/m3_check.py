"""Compare each cmaj render with numpy's reference of the same graph, frame by frame."""
import json, sys
import numpy as np
sys.path.insert(0, __import__("os").path.dirname(__file__))
import wav
M = sys.argv[1] + "/m3"
w = {k: np.array(v, np.float64) for k, v in json.load(open(f"{M}/models/weights.json")).items()}
x, _ = wav.read(f"{M}/in2.wav")
ff = lambda x, w2: np.tanh(x @ w["W1"] + w["B1"]) @ w2 + w["B2"]
C = np.array([[0.1, -0.2]])
ref = {
    "ff": ff(x, w["W2"]), "ff_w": ff(x, w["W2"] * 0.5),
    "gemm_full": x @ w["G"] + C, "gemm_vec": x @ w["G"] + C[0], "gemm_noc": x @ w["G"],
    "gemm_alphabeta": 0.5 * (x @ w["G"]) + 2.0 * C, "gemm_transB": x @ w["G"].T + C,
}
for n, r in ref.items():
    y, _ = wav.read(f"{M}/render/{n}.wav")
    lead = 20480  # cmaj render's pre-roll: 20,000 frames rounded up to its 512-frame block
    assert not np.any(y[:lead]), n
    # With --input the pre-roll blanks output frames rather than delaying the input: output
    # frame n is the graph applied to input frame n, measured with a staircase input.
    y = y[lead:]; r = r[lead:len(y) + lead]
    e = np.abs(y - r)
    rel = e.max() / np.abs(r).max()
    print(f"{n:15s} rel={rel:.2e} compared={len(y)} maxerr={e.max():.3e} at={np.unravel_index(e.argmax(), e.shape)} rms(y)={np.sqrt((y**2).mean()):.4f} rms(ref)={np.sqrt((r**2).mean()):.4f}")

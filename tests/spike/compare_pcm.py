# m5.1-spike. Run: python3 tests/spike/compare_pcm.py <directory of raw float32 renders>
# Compares the pairs the findings table reports; needs numpy, which this repository does not pin.
import numpy as np, os, sys
d = sys.argv[1]
def L(n): return np.fromfile(os.path.join(d, n), dtype='<f4')
def cmp(a, b, label=""):
    x, y = L(a), L(b)
    diff = x != y; n = int(diff.sum())
    if n == 0: return f"{label:42s} IDENTICAL"
    pk = float(np.abs(x - y).max()); db = 20*np.log10(pk) if pk > 0 else -999
    return (f"{label:42s} {n}/{x.size} differ ({100*n/x.size:6.2f}%), peak {pk:.4g} ({db:+.1f} dBFS)")
rows = [
 ("G1.raw","G2.raw","Galactic: fresh process 1 vs 2"),
 ("G2.raw","G3.raw","Galactic: fresh process 2 vs 3"),
 ("C1.raw","C2.raw","Channel5: fresh process 1 vs 2"),
 ("C2.raw","C3.raw","Channel5: fresh process 2 vs 3"),
 ("GfA.raw","GfB.raw","Galactic: instance A vs B, one process"),
 ("GfA.raw","GrA.raw","Galactic: A forward vs A reversed order"),
 ("GfB.raw","GrB.raw","Galactic: B forward vs B reversed order"),
 ("CfA.raw","CfB.raw","Channel5: instance A vs B, one process"),
 ("CfA.raw","CrA.raw","Channel5: A forward vs A reversed order"),
 ("Grst1.raw","Grst2.raw","Galactic: pass1 vs pass2 after reset()"),
 ("Crst1.raw","Crst2.raw","Channel5: pass1 vs pass2 after reset()"),
 ("Crep1.raw","Crep2.raw","Channel5: pass1 vs pass2 after reprepare"),
 ("Crst2.raw","Crep2.raw","Channel5: reset pass2 vs reprepare pass2"),
 ("in.raw","Cbypass.raw","Channel5 bypassed vs the raw input"),
 ("in.raw","C1.raw","Channel5 engaged vs the raw input"),
 ("in.raw","Gdry.raw","Galactic at Dry/Wet=0 vs the raw input"),
 ("G1.raw","Gdry.raw","Galactic wet=1 vs wet=0"),
 ("C1.raw","Cwet0.raw","Channel5 with slot4=1 vs slot4=0 (inactive slot)"),
]
for a,b,l in rows:
    try: print(cmp(a,b,l))
    except FileNotFoundError: print(l, "missing")
print()
for n in ["in.raw","C1.raw","Cbypass.raw","G1.raw","Gdry.raw"]:
    x = L(n).astype(np.float64)
    print(f"{n:14s} rms={np.sqrt((x**2).mean()):.6g} peak={np.abs(x).max():.6g}")

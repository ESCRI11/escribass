"""M4 PR 0 spike, measurement 9: the DSL's one number.

This file is on `m4.0-spike`, which is never merged.  It is here so a reader of
that pull request can re-take the measurement rather than believe it:

    for v in 3.11 3.12 3.13 3.14; do uv run --python $v tests/spike/numeric_domain.py; done

Every `combined_sha256` should read 1730edd3f9825ec91c0b0456f477a15fcf2687d37c8d7224cc0f77a7bf741755.

Question 3's platform-free sentence claims that a generator's numeric domain --
integer ticks, integer pitch and velocity, `Fraction` in between, and one
`random.Random` seeded from `Generator.seed` -- produces the same bytes under any
CPython 3.12.  This prints a digest of exactly those three things so two
interpreters can be compared byte for byte.

No float, no `math`, no `libm` anywhere below.
"""
import hashlib
import json
import random
import sys
from fractions import Fraction

out = {"version": sys.version.split()[0], "implementation": sys.implementation.name,
       "maxsize": sys.maxsize, "hash_info_width": sys.hash_info.width}

# 1. The seeded generator.  2**63+1 is past 64 bits on purpose: CPython seeds
#    Mersenne Twister from the absolute value's 32-bit limbs, so a seed that needs
#    three limbs exercises the path a 1 does not.
rng = random.Random(2**63 + 1)
draws = [rng.getrandbits(32) for _ in range(64)]
draws += [rng.randrange(0, 960) for _ in range(64)]          # ticks in a beat
draws += [rng.randint(21, 108) for _ in range(64)]            # MIDI pitch
draws += [rng.choice([1, 2, 3, 4, 6, 8, 12, 16]) for _ in range(64)]
seq = list(range(32))
rng.shuffle(seq)                                              # shuffle's own algorithm
draws += seq
draws += rng.sample(range(128), 32)                           # sample's set/pool split
out["rng_sha256"] = hashlib.sha256(
    json.dumps(draws, separators=(",", ":")).encode()).hexdigest()
out["rng_head"] = draws[:4]

# 2. The Fraction chain.  Long, deliberately reducible, and ending somewhere a
#    float could not represent -- this is what stands in for "anything in between".
f = Fraction(1, 3)
chain = []
for n in range(1, 400):
    f = (f + Fraction(n, n * n + 1)) * Fraction(n + 1, n + 2)
    if n % 40 == 0:
        f = Fraction(f.numerator % (10**24) or 1, f.denominator % (10**24) or 1)
    chain.append((f.numerator, f.denominator))
# The places a tick actually comes from: a rational position rounded to integer ticks.
ppq = 960
ticks = [ (Fraction(num, den) * ppq).__ceil__() % 100000 for num, den in chain[:64] ]
ticks += [ (Fraction(num, den) * ppq).__floor__() % 100000 for num, den in chain[:64] ]
ticks += [ round(Fraction(num, den) * ppq) % 100000 for num, den in chain[:64] ]  # banker's
out["fraction_sha256"] = hashlib.sha256(
    json.dumps([chain, ticks], separators=(",", ":")).encode()).hexdigest()
out["fraction_final"] = [chain[-1][0] % (10**18), chain[-1][1] % (10**18)]

# 3. Integer arithmetic a compiler would lean on: divmod, //, %, pow, sorted stability.
ints = []
for n in range(1, 200):
    q, r = divmod(n * 9973, 7919)
    ints += [q, r, (-n) // 7, (-n) % 7, pow(n, 17, 2**61 - 1)]
pairs = sorted(((n % 7, n) for n in range(100)), key=lambda p: p[0])
out["int_sha256"] = hashlib.sha256(
    json.dumps([ints, pairs], separators=(",", ":")).encode()).hexdigest()

# 4. The whole thing as one digest, which is what a golden would hold.
out["combined_sha256"] = hashlib.sha256(
    (out["rng_sha256"] + out["fraction_sha256"] + out["int_sha256"]).encode()).hexdigest()

print(json.dumps(out, indent=2, sort_keys=True))

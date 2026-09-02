//! Entity ids, as an injectable dependency.
//!
//! §4.3 requires a ULID on every entity: a 48-bit millisecond timestamp and 80 further bits,
//! rendered as 26 Crockford base32 characters. That makes ids sort in creation order, which
//! is why canonical JSON map keys are also chronological (ADR 0002 §4).
//!
//! A ULID mints wall-clock time and randomness, both forbidden inside `core` by §11, so the
//! source is a constructor parameter: production uses [`UlidSource`], tests use
//! [`SeededIds`] and get byte-identical output (ADR 0001 §5).

use crate::clock::{Clock, SystemClock};

/// Crockford base32: no I, L, O or U, so an id cannot be misread aloud or mistyped into one.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Mints entity ids.
///
/// `&mut self` is deliberate: an id source carries state — the last millisecond it saw and
/// the counter within it — and hiding that behind interior mutability would make it look
/// shareable when it is not.
pub trait IdSource {
    fn next_id(&mut self) -> String;
}

/// Renders 48 bits of timestamp and 80 further bits as a 26-character Crockford ULID.
fn render(ms: u64, tail: u128) -> String {
    let value = ((ms as u128 & ((1 << 48) - 1)) << 80) | (tail & ((1 << 80) - 1));
    let mut out = [0u8; 26];
    let mut v = value;
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(v & 31) as usize];
        v >>= 5;
    }
    // Every byte came from ALPHABET, which is ASCII.
    String::from_utf8(out.to_vec()).expect("Crockford base32 is ASCII")
}

/// The real id source: a ULID per call, monotonic within a millisecond.
///
/// The 80-bit tail is seeded from the OS once and then incremented while the millisecond
/// holds, which is the ULID specification's own monotonic behaviour and makes ids minted in
/// the same millisecond sort in creation order.
///
/// `ponytail:` entropy comes from `std::collections::hash_map::RandomState`, which the
/// standard library seeds from the OS — no dependency for what is a uniqueness requirement,
/// not a secrecy one. If ids ever need to be unguessable, or unique across machines with no
/// coordination, swap this for `getrandom` and pin it in `lock.baseline.json`.
#[derive(Debug)]
pub struct UlidSource<C: Clock = SystemClock> {
    clock: C,
    last_ms: u64,
    tail: u128,
}

impl Default for UlidSource<SystemClock> {
    fn default() -> Self {
        Self::new(SystemClock)
    }
}

impl<C: Clock> UlidSource<C> {
    pub fn new(clock: C) -> Self {
        Self { clock, last_ms: 0, tail: os_entropy() }
    }
}

impl<C: Clock> IdSource for UlidSource<C> {
    fn next_id(&mut self) -> String {
        let ms = self.clock.now_ms().max(0) as u64;
        if ms == self.last_ms {
            // Same millisecond: increment, so ids stay sortable by creation order.
            self.tail = self.tail.wrapping_add(1);
        } else {
            self.last_ms = ms;
            self.tail = os_entropy();
        }
        render(ms, self.tail)
    }
}

/// 80 bits from the standard library's OS-seeded hasher keys.
fn os_entropy() -> u128 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let mix = |salt: u64| {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(salt);
        h.finish() as u128
    };
    // Two independently keyed hashers, because one gives 64 bits and a ULID tail is 80.
    (mix(0) << 64 | mix(1)) & ((1 << 80) - 1)
}

/// A deterministic id source for tests and fixtures.
///
/// Produces well-formed ULIDs — they pass the validator's `id_not_ulid` rule — from a fixed
/// timestamp and a counter, so the same sequence of calls always yields the same ids and a
/// determinism suite can compare canonical JSON byte for byte.
#[derive(Debug, Clone)]
pub struct SeededIds {
    ms: u64,
    next: u128,
}

impl SeededIds {
    /// `ms` becomes the timestamp half of every id; `seed` the starting counter.
    pub fn new(ms: i64, seed: u64) -> Self {
        Self { ms: ms.max(0) as u64, next: seed as u128 }
    }
}

impl Default for SeededIds {
    /// 2026-09-02T00:00:00Z, the instant the fixtures use.
    fn default() -> Self {
        Self::new(1_788_307_200_000, 1)
    }
}

impl IdSource for SeededIds {
    fn next_id(&mut self) -> String {
        let id = render(self.ms, self.next);
        self.next = self.next.wrapping_add(1);
        id
    }
}

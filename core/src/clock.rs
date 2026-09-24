//! Time, as an injectable dependency.
//!
//! §11 forbids wall-clock dependence inside `core`, and §4.3 puts a `created_at` on every
//! entity. Both hold only if the clock is a constructor parameter rather than a call to
//! `SystemTime::now` buried in a mutation: tests then fix it and compare output byte for
//! byte, with no normalisation step (ADR 0001 §5).
//!
//! Every clock here yields **milliseconds**. ADR 0002 §4 makes that the stored precision, so
//! producing it at the source means nothing downstream truncates and the validator's
//! `timestamp_precision` rule can never fire on a value this module produced.

use escribass_schema::pbjson_types::Timestamp;

/// Milliseconds since the Unix epoch.
pub trait Clock {
    fn now_ms(&self) -> i64;

    /// The current instant as a protobuf `Timestamp`, at millisecond precision.
    fn now(&self) -> Timestamp {
        timestamp_from_ms(self.now_ms())
    }

    /// A copy of this clock, reading what it reads.
    ///
    /// [`IdSource::fork`](crate::id::IdSource::fork)'s mirror, for the one caller that needs
    /// both: a proposal is a session on a copy of the project (ADR 0019 §1), and a session
    /// owns a boxed clock. Not `Clone` on the trait, which would make it un-object-safe.
    fn fork(&self) -> Box<dyn Clock + Send>;
}

/// Converts epoch milliseconds to a `Timestamp` whose `nanos` are a whole number of
/// milliseconds. Negative instants (before 1970) normalise so `nanos` stays non-negative,
/// which is what the protobuf encoding requires.
pub fn timestamp_from_ms(ms: i64) -> Timestamp {
    let seconds = ms.div_euclid(1_000);
    let nanos = (ms.rem_euclid(1_000) * 1_000_000) as i32;
    Timestamp { seconds, nanos }
}

/// The real clock. The only place in `core` that reads wall-clock time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn fork(&self) -> Box<dyn Clock + Send> {
        Box::new(*self)
    }

    fn now_ms(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as i64,
            // Before 1970 only if the machine's clock is badly wrong; still returns a usable
            // instant rather than panicking in the middle of a user's edit.
            Err(e) => -(e.duration().as_millis() as i64),
        }
    }
}

/// A clock that does not move. Tests set it and compare bytes.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(pub i64);

impl Clock for FixedClock {
    fn fork(&self) -> Box<dyn Clock + Send> {
        Box::new(*self)
    }

    fn now_ms(&self) -> i64 {
        self.0
    }
}

//! One plan in, one `RenderResult` out, through a real engine — for the checks that are shell.
//!
//! **Why this exists.** The engine job in `.github/workflows/checks.yml` asserts six things
//! nothing else in this repository asks: that a hosted note sounds, that an asset plays at its
//! gain and with its fades, that a stretched clip fills its clip, that a sampler plays an SFZ,
//! and that a crowded, oversized or nonsense render fails instead of lying. All six generate
//! their fixtures in shell and Python and compare envelopes sample by sample, and until M2 PR 9
//! they drove the engine by writing a plan to its stdin. gRPC replaced that, and a shell script
//! cannot speak HTTP/2.
//!
//! So this is the client those steps need, and it is deliberately **not a second one**: it is
//! `core`'s own [`Engine`], the same spawn, the same address line, the same call and the same
//! failure classification that `render_export` uses. A C++ client in the engine's own build tree
//! would have been cheaper to run and would have been a copy of this one, free to drift — which
//! is the objection ADR 0006 §1 makes to every second implementation of one contract. The
//! engine job pays a cargo build for it, and gets those six steps testing the shipping client
//! rather than a protocol nothing else speaks.
//!
//! Usage, which is the shape the engine's own stdio mode had, so the steps that call it changed
//! by one word:
//!
//! ```text
//! render-once <engine> <manifest.json> < plan.binpb > result.binpb
//! ```
//!
//! A failure is a non-zero exit and the reason on stderr — including the tail of what the engine
//! itself said, which `core` collects (ADR 0008 §1).

use escribass_core::Engine;
use escribass_proto::render::{RenderPlan, RenderResult};
use prost::Message;
use std::io::{Read, Write};

fn main() -> std::process::ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [engine, manifest] = arguments.as_slice() else {
        eprintln!("usage: render-once <engine> <manifest.json> < plan.binpb > result.binpb");
        return std::process::ExitCode::from(2);
    };

    let mut plan = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut plan) {
        eprintln!("render-once: cannot read the plan: {e}");
        return std::process::ExitCode::from(2);
    }
    let plan = match RenderPlan::decode(&plan[..]) {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("render-once: stdin is not a RenderPlan: {e}");
            return std::process::ExitCode::from(2);
        }
    };

    match Engine::new(engine, manifest).render(&plan) {
        Err(failed) => {
            // `rule` is the same one `render_export` would have reported — engine_missing,
            // engine_failed or engine_unreadable — so a step that fails here names which side
            // of ADR 0006 §2's line it landed on.
            eprintln!("render-once: {}: {}", failed.rule, failed.message);
            std::process::ExitCode::from(3)
        }
        Ok(result) => write_out(&result),
    }
}

/// The answer on stdout, checked after the flush.
///
/// The engine learnt this the hard way (M1 PR 13): a `RenderResult` fits a buffer, so the real
/// `write(2)` happens in the flush and an unchecked one is exit 0 with an empty stdout.
fn write_out(result: &RenderResult) -> std::process::ExitCode {
    let mut out = std::io::stdout();
    if let Err(e) = out.write_all(&result.encode_to_vec()).and_then(|()| out.flush()) {
        eprintln!("render-once: could not write the RenderResult to stdout: {e}");
        return std::process::ExitCode::from(4);
    }
    std::process::ExitCode::SUCCESS
}

//! Testable CLI entry point. `run()` parses args from an iterator, reads/writes
//! the provided streams, and returns an exit code so it can be exercised in-process.
use crate::pouw::{mine, min_challenges, prove, verify, Block, Proof};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Parser)]
#[command(name = "bse-consensus", version, about = "PoUW consensus over a simulated market")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Mine a block (runs the deterministic optimisation) and print {block, proof, challenges} JSON.
    Mine {
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 3)]
        difficulty: u32,
        /// Minimum challenges to request. Automatically raised to min_challenges(difficulty) if lower.
        #[arg(long, default_value_t = 12)]
        challenges: u32,
    },
    /// Verify a {block, proof, challenges} JSON bundle from stdin. Exits 0 if VALID, 1 if invalid.
    Verify,
    /// Inspect a bundle from stdin (human-readable summary).
    Inspect,
}

#[derive(Serialize, Deserialize)]
struct Bundle {
    block: Block,
    proof: Proof,
    challenges: u32,
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Read and parse a bundle from `stdin`. Returns `Err` with an exit code on failure.
fn read_bundle(stdin: &mut dyn Read, stdout: &mut dyn Write) -> Result<Bundle, i32> {
    serde_json::from_reader(stdin).map_err(|e| {
        let _ = writeln!(stdout, "error: invalid bundle JSON: {e}");
        1
    })
}

/// Parse `args`, dispatch the subcommand reading from `stdin` and writing to `stdout`,
/// and return the process exit code (0 ok, 1 invalid, 2 usage error).
///
/// Behaviour matches the original binary: same output text, same `--challenges`
/// flooring to `min_challenges(difficulty)`, and the same exit codes.
pub fn run(
    args: impl IntoIterator<Item = String>,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let _ = write!(stdout, "{e}");
            // clap treats --help/--version as "errors" that should exit 0.
            return if e.use_stderr() { 2 } else { 0 };
        }
    };

    match cli.cmd {
        Cmd::Mine {
            seed,
            difficulty,
            challenges,
        } => {
            // Enforce the challenge floor: never prove with fewer than min_challenges(difficulty).
            let k = challenges.max(min_challenges(difficulty));
            let (block, frames) = mine(seed, difficulty);
            let proof = prove(&block, &frames, k);
            let bundle = Bundle {
                block,
                proof,
                challenges: k,
            };
            let _ = writeln!(stdout, "{}", serde_json::to_string_pretty(&bundle).unwrap());
            0
        }
        Cmd::Verify => {
            let bundle = match read_bundle(stdin, stdout) {
                Ok(b) => b,
                Err(code) => return code,
            };
            match verify(&bundle.block, &bundle.proof, bundle.challenges) {
                Ok(()) => {
                    let _ = writeln!(
                        stdout,
                        "VALID: fitness {} >= threshold, all spot-checks pass",
                        bundle.block.claimed_fitness
                    );
                    0
                }
                Err(e) => {
                    let _ = writeln!(stdout, "INVALID: {:?}", e);
                    1
                }
            }
        }
        Cmd::Inspect => {
            let bundle = match read_bundle(stdin, stdout) {
                Ok(b) => b,
                Err(code) => return code,
            };
            let _ = writeln!(
                stdout,
                "seed={} difficulty={} frames={} challenges={}\nbest={:?} fitness={}\nroot={}",
                bundle.block.seed,
                bundle.block.difficulty,
                bundle.block.n_frames,
                bundle.challenges,
                bundle.block.best_strategy,
                bundle.block.claimed_fitness,
                hex32(&bundle.block.root),
            );
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn argv(parts: &[&str]) -> Vec<String> {
        std::iter::once("bse-consensus")
            .chain(parts.iter().copied())
            .map(String::from)
            .collect()
    }

    /// Run `mine` and return its stdout bytes (the JSON bundle).
    fn mine_bundle(seed: &str, difficulty: &str) -> Vec<u8> {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(
            argv(&["mine", "--seed", seed, "--difficulty", difficulty]),
            &mut stdin,
            &mut stdout,
        );
        assert_eq!(code, 0, "mine must exit 0");
        stdout
    }

    #[test]
    fn mine_then_verify_roundtrip_returns_zero() {
        let bundle = mine_bundle("7", "2");
        assert!(!bundle.is_empty(), "mine must produce JSON");

        let mut stdin = Cursor::new(bundle);
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["verify"]), &mut stdin, &mut stdout);
        assert_eq!(code, 0, "valid bundle must verify with exit 0");
        let out = String::from_utf8(stdout).unwrap();
        assert!(out.contains("VALID"), "verify must print VALID: {out}");
    }

    #[test]
    fn tampered_bundle_returns_nonzero() {
        let bundle = mine_bundle("42", "1");
        let mut v: serde_json::Value = serde_json::from_slice(&bundle).unwrap();
        v["block"]["claimed_fitness"] = serde_json::json!(0);
        let tampered = serde_json::to_vec(&v).unwrap();

        let mut stdin = Cursor::new(tampered);
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["verify"]), &mut stdin, &mut stdout);
        assert_ne!(code, 0, "tampered bundle must exit non-zero");
        let out = String::from_utf8(stdout).unwrap();
        assert!(out.contains("INVALID"), "tampered must print INVALID: {out}");
    }

    #[test]
    fn inspect_prints_summary_returns_zero() {
        let bundle = mine_bundle("3", "1");
        let mut stdin = Cursor::new(bundle);
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["inspect"]), &mut stdin, &mut stdout);
        assert_eq!(code, 0, "inspect must exit 0");
        let out = String::from_utf8(stdout).unwrap();
        assert!(out.contains("seed=3"), "inspect must include seed: {out}");
        assert!(out.contains("difficulty=1"), "inspect must include difficulty: {out}");
    }

    #[test]
    fn verify_invalid_json_returns_one() {
        let mut stdin = Cursor::new(b"not json at all".to_vec());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["verify"]), &mut stdin, &mut stdout);
        assert_eq!(code, 1, "invalid bundle JSON must exit 1");
        let out = String::from_utf8(stdout).unwrap();
        assert!(out.contains("error"), "must report a JSON error: {out}");
    }

    #[test]
    fn inspect_invalid_json_returns_one() {
        let mut stdin = Cursor::new(b"{garbage".to_vec());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["inspect"]), &mut stdin, &mut stdout);
        assert_eq!(code, 1, "invalid inspect JSON must exit 1");
    }

    #[test]
    fn unknown_subcommand_returns_usage_code() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["frobnicate"]), &mut stdin, &mut stdout);
        assert_eq!(code, 2, "unknown subcommand is a usage error (exit 2)");
    }

    #[test]
    fn missing_args_returns_usage_code() {
        // no subcommand at all
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&[]), &mut stdin, &mut stdout);
        assert_eq!(code, 2, "missing subcommand is a usage error (exit 2)");
    }

    #[test]
    fn mine_missing_required_seed_returns_usage_code() {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(argv(&["mine", "--difficulty", "1"]), &mut stdin, &mut stdout);
        assert_eq!(code, 2, "mine without --seed is a usage error (exit 2)");
    }

    #[test]
    fn challenges_flooring_preserved() {
        // request a tiny --challenges; the bundle's stored value must be floored
        // up to min_challenges(difficulty) and still verify.
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout: Vec<u8> = Vec::new();
        let code = run(
            argv(&["mine", "--seed", "9", "--difficulty", "2", "--challenges", "1"]),
            &mut stdin,
            &mut stdout,
        );
        assert_eq!(code, 0);
        let v: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
        let stored = v["challenges"].as_u64().unwrap() as u32;
        assert_eq!(
            stored,
            min_challenges(2),
            "challenges must be floored to min_challenges(difficulty)"
        );
    }
}

use bse_consensus::pouw::{mine, prove, verify, min_challenges, Block, Proof};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

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

fn main() {
    match Cli::parse().cmd {
        Cmd::Mine {
            seed,
            difficulty,
            challenges,
        } => {
            // Enforce the challenge floor: never prove with fewer than min_challenges(difficulty).
            let k = challenges.max(min_challenges(difficulty));
            let (block, frames) = mine(seed, difficulty);
            let proof = prove(&block, &frames, k);
            println!(
                "{}",
                serde_json::to_string_pretty(&Bundle {
                    block,
                    proof,
                    challenges: k,
                })
                .unwrap()
            );
        }
        Cmd::Verify => {
            let bundle: Bundle =
                serde_json::from_reader(std::io::stdin()).unwrap_or_else(|e| {
                    eprintln!("error: invalid bundle JSON: {e}");
                    std::process::exit(1);
                });
            match verify(&bundle.block, &bundle.proof, bundle.challenges) {
                Ok(()) => {
                    eprintln!(
                        "VALID: fitness {} >= threshold, all spot-checks pass",
                        bundle.block.claimed_fitness
                    );
                }
                Err(e) => {
                    eprintln!("INVALID: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Cmd::Inspect => {
            let bundle: Bundle =
                serde_json::from_reader(std::io::stdin()).unwrap_or_else(|e| {
                    eprintln!("error: invalid bundle JSON: {e}");
                    std::process::exit(1);
                });
            println!(
                "seed={} difficulty={} frames={} challenges={}\nbest={:?} fitness={}\nroot={}",
                bundle.block.seed,
                bundle.block.difficulty,
                bundle.block.n_frames,
                bundle.challenges,
                bundle.block.best_strategy,
                bundle.block.claimed_fitness,
                hex32(&bundle.block.root),
            );
        }
    }
}

fn hex32(b: &[u8; 32]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

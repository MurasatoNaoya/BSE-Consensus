use std::io::Write;
use std::process::{Command, Stdio};

/// Mine at difficulty 2 (min_challenges = 10); omit --challenges so the CLI
/// auto-floors to 10. The bundle's stored `challenges` value is guaranteed >= floor,
/// so verify must accept it and exit 0.
#[test]
fn mine_then_verify_roundtrip() {
    let mine_out = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["mine", "--seed", "7", "--difficulty", "2"])
        .output()
        .expect("mine command failed to spawn");

    assert!(
        mine_out.status.success(),
        "mine must exit 0; stderr: {}",
        String::from_utf8_lossy(&mine_out.stderr)
    );
    assert!(
        !mine_out.stdout.is_empty(),
        "mine must produce JSON on stdout"
    );

    let mut verify_proc = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["verify"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("verify command failed to spawn");

    verify_proc
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&mine_out.stdout)
        .expect("writing to verify stdin failed");

    let res = verify_proc
        .wait_with_output()
        .expect("waiting on verify failed");

    assert!(
        res.status.success(),
        "valid bundle must verify (exit 0); stderr: {}",
        String::from_utf8_lossy(&res.stderr)
    );
}

/// Tampered bundle (block fitness set to 0) must exit non-zero.
#[test]
fn tampered_bundle_exits_nonzero() {
    let mine_out = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["mine", "--seed", "42", "--difficulty", "1"])
        .output()
        .expect("mine command failed to spawn");

    assert!(mine_out.status.success(), "mine must exit 0");

    // Parse and tamper the bundle
    let mut bundle: serde_json::Value =
        serde_json::from_slice(&mine_out.stdout).expect("mine output is valid JSON");
    bundle["block"]["claimed_fitness"] = serde_json::json!(0);
    let tampered = serde_json::to_vec(&bundle).unwrap();

    let mut verify_proc = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["verify"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("verify command failed to spawn");

    verify_proc
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&tampered)
        .expect("writing to verify stdin failed");

    let res = verify_proc
        .wait_with_output()
        .expect("waiting on verify failed");

    assert!(
        !res.status.success(),
        "tampered bundle (fitness=0) must exit non-zero"
    );
}

/// inspect must exit 0 and print human-readable output.
#[test]
fn inspect_exits_zero() {
    let mine_out = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["mine", "--seed", "3", "--difficulty", "1"])
        .output()
        .expect("mine command failed to spawn");

    assert!(mine_out.status.success(), "mine must exit 0");

    let mut inspect_proc = Command::new(env!("CARGO_BIN_EXE_bse-consensus"))
        .args(["inspect"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("inspect command failed to spawn");

    inspect_proc
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&mine_out.stdout)
        .expect("writing to inspect stdin failed");

    let res = inspect_proc
        .wait_with_output()
        .expect("waiting on inspect failed");

    assert!(res.status.success(), "inspect must exit 0");
    let stdout = String::from_utf8_lossy(&res.stdout);
    assert!(stdout.contains("seed=3"), "inspect output must include seed");
    assert!(
        stdout.contains("difficulty=1"),
        "inspect output must include difficulty"
    );
}

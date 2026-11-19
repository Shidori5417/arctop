//! İkili düzeyinde CLI testleri: yalnız argüman ayrıştırma yolları (donanımdan bağımsız).

use std::process::Command;

/// Test edilen ikiliyi `args` ile çalıştıran `Command` döndürür.
fn arctop(args: &[&str]) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arctop"));
    c.args(args);
    c
}

#[test]
fn json_alone_fails_requesting_once() {
    let out = arctop(&["--json"]).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--once"), "stderr: {stderr}");
}

#[test]
fn interval_out_of_range_fails() {
    for arg in ["100", "6000"] {
        let out = arctop(&["--interval", arg]).output().unwrap();
        assert!(!out.status.success(), "--interval {arg} reddedilmeli");
    }
}

#[test]
fn help_lists_once_json_and_interval() {
    let out = arctop(&["--help"]).output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--once"), "stdout: {stdout}");
    assert!(stdout.contains("--json"), "stdout: {stdout}");
    assert!(stdout.contains("--interval"), "stdout: {stdout}");
}

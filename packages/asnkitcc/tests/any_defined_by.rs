// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

use asnkitrt::{
    container::Checks,
    runtime::{Options, Specification},
    types::Codec,
    value::Value,
};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(folder: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asnkitcc"))
        .current_dir(folder)
        .args(args)
        .output()
        .unwrap()
}

fn fixture(folder: &Path) {
    fs::write(
        folder.join("m.asn"),
        "M DEFINITIONS ::= BEGIN
T ::= SEQUENCE { kind INTEGER, payload ANY DEFINED BY kind }
U ::= SEQUENCE { kind INTEGER, payload ANY DEFINED BY kind }
END",
    )
    .unwrap();
    for (file, ty) in [("t.toml", "T"), ("u.toml", "U")] {
        fs::write(
            folder.join(file),
            format!(
                "[[any_defined_by]]\ntype='M.{ty}'\nmember='payload'\nchoices={{'1'='INTEGER'}}\n"
            ),
        )
        .unwrap();
    }
}

fn check(path: &Path, name: &str) {
    let asni = fs::read(path).unwrap();
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Der)).unwrap();
    let value = Value::Map(vec![
        ("kind".into(), Value::Integer(1)),
        ("payload".into(), Value::Integer(42)),
    ]);
    let encoded = spec.encode(name, &value).unwrap();
    assert_eq!(encoded, [0x30, 6, 2, 1, 1, 2, 1, 42]);
    assert_eq!(spec.decode(name, &encoded).unwrap(), value);
}

#[test]
fn repeated_configuration_files_and_errors() {
    let folder = tempfile::tempdir().unwrap();
    fixture(folder.path());
    let output = run(
        folder.path(),
        &[
            "compile",
            "--any-defined-by",
            "t.toml",
            "--any-defined-by",
            "u.toml",
            "-o",
            "m.asni",
            "m.asn",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check(&folder.path().join("m.asni"), "T");
    check(&folder.path().join("m.asni"), "U");
    let duplicate = run(
        folder.path(),
        &[
            "compile",
            "--any-defined-by",
            "t.toml",
            "--any-defined-by",
            "t.toml",
            "-o",
            "bad.asni",
            "m.asn",
        ],
    );
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("Duplicate mapping"));
    assert!(!folder.path().join("bad.asni").exists());
    let missing = run(
        folder.path(),
        &["compile", "--any-defined-by", "missing.toml", "m.asn"],
    );
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("missing.toml"));
    fs::write(
        folder.path().join("bad.toml"),
        "[[any_defined_by]]\ntypo='M.T'",
    )
    .unwrap();
    let invalid = run(
        folder.path(),
        &["compile", "--any-defined-by", "bad.toml", "m.asn"],
    );
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("bad.toml"));
    for args in [
        vec!["parse", "--any-defined-by", "t.toml", "m.asn"],
        vec!["convert", "-t", "T", "--any-defined-by", "t.toml", "m.asni"],
    ] {
        let output = run(folder.path(), &args);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("for compile and generate"));
    }
}

#[cfg(unix)]
#[test]
fn generator_receives_the_mapped_schema_and_original_syntax_tree() {
    use std::os::unix::fs::PermissionsExt;
    let folder = tempfile::tempdir().unwrap();
    fixture(folder.path());
    let plugin = folder.path().join("capture-plugin");
    fs::write(
        &plugin,
        "#!/bin/sh\ncp \"$2\" \"$6/m.asni\"\ncp \"$4\" \"$6/m.asnst\"\n",
    )
    .unwrap();
    fs::set_permissions(&plugin, fs::Permissions::from_mode(0o755)).unwrap();
    let output = run(
        folder.path(),
        &[
            "generate",
            "--plugin",
            plugin.to_str().unwrap(),
            "--any-defined-by",
            "t.toml",
            "-o",
            "out",
            "m.asn",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check(&folder.path().join("out/m.asni"), "T");
    assert!(
        run(folder.path(), &["parse", "-o", "original.asnst", "m.asn"])
            .status
            .success()
    );
    assert_eq!(
        fs::read(folder.path().join("original.asnst")).unwrap(),
        fs::read(folder.path().join("out/m.asnst")).unwrap()
    );
}

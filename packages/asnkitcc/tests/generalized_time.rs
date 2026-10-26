// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

//! A GeneralizedTime read from the form DER, PER and UPER have is in UTC,
//! and stays so when it is written in another codec.

use asnkitrt::{
    container::Checks,
    runtime::{Options, Specification},
    types::Codec,
    value::{Date, DateTime, Time, Value},
};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

fn run(folder: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_asnkitcc"))
        .current_dir(folder)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn the_z_of_a_restricted_generalized_time_is_kept() {
    let folder = tempfile::tempdir().unwrap();
    let folder = folder.path();
    fs::write(
        folder.join("t.asn"),
        "T DEFINITIONS ::= BEGIN G ::= GeneralizedTime END",
    )
    .unwrap();
    stdout(run(folder, &["compile", "-o", "t.asni", "t.asn"], b""));
    let asni = fs::read(folder.join("t.asni")).unwrap();

    // As it was, and not a local time.
    let value = Value::DateTime(DateTime {
        date: Date {
            year: 2026,
            month: 10,
            day: 26,
        },
        time: Time {
            hour: 15,
            minute: 49,
            second: 44,
            microsecond: 0,
            offset: Some(0),
        },
    });
    for codec in [Codec::Der, Codec::Per, Codec::Uper, Codec::Oer] {
        let spec =
            Specification::from_asni(&asni, &Checks::default(), Options::new(codec)).unwrap();
        let encoded = spec.encode("G", &value).unwrap();
        assert_eq!(spec.decode("G", &encoded).unwrap(), value, "{codec:?}");
    }

    let der = "180f32303236313032363135343934345a";
    for (to, expected) in [
        ("gser", "\"20261026154944Z\""),
        ("jer", "\"20261026154944Z\""),
        ("xer", "<G>20261026154944Z</G>"),
    ] {
        let output = stdout(run(
            folder,
            &[
                "convert",
                "-t",
                "G",
                "--from",
                "der",
                "--input-hex",
                "--to",
                to,
                "t.asni",
            ],
            der.as_bytes(),
        ));
        assert!(output.contains(expected), "{to}: {output}");
    }
}

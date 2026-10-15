// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Runs a plugin, a program that makes something from a specification
//! (see `docs/plugins.md`).
//!
//! The plugin is run with `--asni FILE --asnst FILE --out FOLDER` and
//! an `--opt KEY=VALUE` for each option. The files are in a folder of
//! their own that is removed once the plugin is done.
//!
//! ```no_run
//! use asnkit_compiler::container::Protection;
//! use asnkit_compiler::{Input, compile_to_asni, parse_to_asnst, plugin};
//! use std::path::Path;
//!
//! let input = Input::String(std::fs::read_to_string("foo.asn")?);
//! let asni = compile_to_asni(&input, None, None, &Protection::default())?;
//! let asnst = parse_to_asnst(&input)?;
//!
//! plugin::run("rust", &[], Path::new("src"), &asni, &asnst)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::Error;

fn plugin_error(message: String) -> Error {
    Error {
        kind: "PluginError".to_string(),
        message,
    }
}

/// The program to run for a plugin given by name or by path: a name is
/// `asnkit-gen-` and the name, found where programs are.
pub fn program(plugin: &str) -> PathBuf {
    if plugin.contains(std::path::MAIN_SEPARATOR) || plugin.contains('/') {
        PathBuf::from(plugin)
    } else {
        PathBuf::from(format!("asnkit-gen-{plugin}"))
    }
}

/// A folder that is removed when it is dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Scratch, Error> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path =
            std::env::temp_dir().join(format!("asnkit-plugin-{}-{nanos}", std::process::id()));

        std::fs::create_dir(&path)
            .map_err(|error| plugin_error(format!("{}: {error}", path.display())))?;

        Ok(Scratch(path))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs the plugin, by name or by path, on a specification, which is
/// given as the contents of its `.asni` and `.asnst` files, to write to
/// the folder `out`. What the plugin writes to its standard output and
/// error goes to those of this process.
pub fn run(
    plugin: &str,
    options: &[String],
    out: &Path,
    asni: &[u8],
    asnst: &[u8],
) -> Result<(), Error> {
    let scratch = Scratch::new()?;
    let asni_path = scratch.0.join("specification.asni");
    let asnst_path = scratch.0.join("specification.asnst");

    for (path, data) in [(&asni_path, asni), (&asnst_path, asnst)] {
        std::fs::write(path, data)
            .map_err(|error| plugin_error(format!("{}: {error}", path.display())))?;
    }

    std::fs::create_dir_all(out)
        .map_err(|error| plugin_error(format!("{}: {error}", out.display())))?;

    let program = program(plugin);
    let mut command = Command::new(&program);
    command
        .arg("--asni")
        .arg(&asni_path)
        .arg("--asnst")
        .arg(&asnst_path)
        .arg("--out")
        .arg(out);

    for option in options {
        command.arg("--opt").arg(option);
    }

    let status = command.status().map_err(|error| {
        plugin_error(if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "The plugin '{plugin}' is not found: there is no program {} to run.",
                program.display()
            )
        } else {
            format!("{}: {error}", program.display())
        })
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(plugin_error(format!(
            "The plugin '{plugin}' failed with {status}."
        )))
    }
}

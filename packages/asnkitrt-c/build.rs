// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

fn main() {
    // The shared library is found by the runpath of what links it, as on
    // Linux, and not by the path it was built at.
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple") {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/libasnkitrt_c.dylib");
    }
}

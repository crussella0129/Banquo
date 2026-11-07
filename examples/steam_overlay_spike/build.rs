//! Places the Steamworks redistributable and `steam_appid.txt` (480) beside the
//! spike binary at build time.
//!
//! The redistributable (`steam_api64.dll` / `libsteam_api.so`) is proprietary
//! and never committed. It comes from the `steamworks-sys` crate, whose own build
//! script copies it from its bundled SDK (`lib/steam/redistributable_bin/<platform>`)
//! into its `OUT_DIR`. `steamworks-sys` is also a build-dependency of this
//! package, so that copy exists before this script runs. `STEAM_SDK_LOCATION`,
//! when set, takes precedence (same variable `steamworks-sys` honours).
//! On Windows the binary will not start without the DLL beside it (load-time
//! import), which is why this happens at build time rather than run time.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=STEAM_SDK_LOCATION");

    let target = env::var("TARGET").unwrap_or_default();
    let (lib_name, sdk_subdir) = if target.contains("windows") {
        ("steam_api64.dll", "win64")
    } else if target.contains("linux") {
        ("libsteam_api.so", "linux64")
    } else if target.contains("darwin") {
        ("libsteam_api.dylib", "osx")
    } else {
        println!("cargo:warning=steam_overlay_spike: unsupported target {target}");
        return;
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    // OUT_DIR = <target>/[<triple>/]<profile>/build/<pkg>-<hash>/out
    let Some(profile_dir) = out_dir.ancestors().nth(3).map(Path::to_path_buf) else {
        return;
    };

    let source = env::var_os("STEAM_SDK_LOCATION")
        .map(|sdk| {
            PathBuf::from(sdk)
                .join("redistributable_bin")
                .join(sdk_subdir)
                .join(lib_name)
        })
        .filter(|p| p.is_file())
        .or_else(|| find_in_build_dirs(&profile_dir, lib_name));

    match source {
        Some(src) => {
            let dest = profile_dir.join(lib_name);
            if let Err(e) = fs::copy(&src, &dest) {
                println!(
                    "cargo:warning=steam_overlay_spike: could not copy {} to {}: {e}",
                    src.display(),
                    dest.display()
                );
            }
        }
        None => println!(
            "cargo:warning=steam_overlay_spike: {lib_name} not found in the steamworks-sys build \
             output; copy it beside the binary from the Steamworks SDK \
             (redistributable_bin/{sdk_subdir})"
        ),
    }

    // Linux does not search the executable's directory for shared libraries;
    // an `$ORIGIN` rpath lets the binary find the libsteam_api.so beside it.
    if target.contains("linux") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
    }

    // Spacewar's App ID, for runs started outside Steam.
    let _ = fs::write(profile_dir.join("steam_appid.txt"), "480");
}

/// Newest `<build>/steamworks-sys-*/out/<lib_name>` in this profile's build dir
/// or, when cross-compiling, the host profile's build dir (build-dependencies).
fn find_in_build_dirs(profile_dir: &Path, lib_name: &str) -> Option<PathBuf> {
    let mut build_dirs = vec![profile_dir.join("build")];
    if let (Some(profile), Some(target_root)) = (
        profile_dir.file_name(),
        profile_dir.parent().and_then(Path::parent),
    ) {
        build_dirs.push(target_root.join(profile).join("build"));
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in build_dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with("steamworks-sys-") {
                continue;
            }
            let candidate = entry.path().join("out").join(lib_name);
            if let Ok(meta) = fs::metadata(&candidate) {
                let modified = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                if best.as_ref().is_none_or(|(t, _)| modified > *t) {
                    best = Some((modified, candidate));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

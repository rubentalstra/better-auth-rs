//! Porting + upstream-sync tooling for the better-auth → Rust port.
//!
//! Model: **in-crate co-location** (Bun-style). Each upstream `.ts` source is placed next to
//! where its `.rs` will go, *inside* the crate `src/` (e.g. `crates/.../src/crypto/password.ts`
//! beside `password.rs`). Cargo compiles only `.rs`; the `.ts` are excluded from the published
//! crate (`exclude = ["**/*.ts"]`) and deleted once a file is fully ported.
//!
//! Subcommands:
//!   vendor --from <clone>   copy upstream sources (+ their *.test.ts) to their in-crate
//!                           sibling locations and (re)write `port/manifest.tsv`
//!   manifest                refresh `port/manifest.tsv` (loc/status) from existing rows
//!   sync                    [stub] diff upstream vs `port/UPSTREAM_PORTED` (Phase 7+)
//!   differential            [stub] run the TS-vs-Rust differential harness (Phase 4+)
//!
//! std-only on purpose: must build and run with zero network access.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MANIFEST_PATH: &str = "port/manifest.tsv";
const UPSTREAM_PORTED_PATH: &str = "port/UPSTREAM_PORTED";
const MANIFEST_HEADER: &str = "ts_path\tloc\trust_path\tstatus\tconfidence\tupstream_sha\n";

/// Top-level upstream packages we deliberately do NOT port (JS/TS-frontend only).
const SKIP_PACKAGES: &[&str] = &["expo", "electron"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("vendor") => {
            let from = flag_value(&args, "--from");
            match from {
                Some(dir) => vendor(&dir),
                None => Err("usage: cargo xtask vendor --from <path-to-upstream-clone>".into()),
            }
        }
        Some("manifest") => refresh_manifest(),
        Some("sync") => {
            println!("`sync` is not implemented yet (Phase 7 / ongoing upstream tracking).");
            return ExitCode::SUCCESS;
        }
        Some("differential") => {
            println!("`differential` is not implemented yet (Phase 4).");
            return ExitCode::SUCCESS;
        }
        other => Err(format!(
            "usage: cargo xtask <vendor|manifest|sync|differential>\n  unknown subcommand: {}",
            other.unwrap_or("<none>")
        )),
    };
    match result {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn upstream_sha() -> String {
    fs::read_to_string(UPSTREAM_PORTED_PATH)
        .ok()
        .and_then(|s| s.lines().next().map(|l| l.trim().to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "UNSET".to_string())
}

/// The `better-auth-rs-utils` crate ports the *separate* `@better-auth/utils` npm package, pinned in
/// its own `UPSTREAM` file (a different repo than `UPSTREAM_PORTED`). Rows under that crate get this
/// commit so the manifest's `upstream_sha` column stays truthful per origin.
const UTILS_UPSTREAM_PATH: &str = "crates/better-auth-rs-utils/UPSTREAM";
const UTILS_PREFIX: &str = "crates/better-auth-rs-utils/";

fn utils_upstream_sha() -> Option<String> {
    fs::read_to_string(UTILS_UPSTREAM_PATH).ok().and_then(|s| {
        s.lines()
            .find_map(|l| {
                l.trim()
                    .strip_prefix("commit:")
                    .map(|v| v.trim().to_string())
            })
            .filter(|s| !s.is_empty())
    })
}

/// `vendor`: copy each portable upstream source (+ its sibling `*.test.ts`) into the crate
/// `src/` at its rust sibling's directory, then (re)write the manifest.
fn vendor(from: &str) -> Result<String, String> {
    let pkgs = Path::new(from).join("packages");
    if !pkgs.is_dir() {
        return Err(format!(
            "{} not found (expected an upstream better-auth clone)",
            pkgs.display()
        ));
    }

    let mut sources: Vec<PathBuf> = Vec::new();
    collect_sources(&pkgs, &mut sources)?;
    sources.sort();

    let prior = load_prior_status();
    let sha = upstream_sha();
    let mut rows: Vec<(String, usize, String, String, String)> = Vec::new();
    let mut copied = 0usize;
    let mut tests = 0usize;

    for src in &sources {
        let upstream_rel = rel_under(from, src)?; // e.g. "packages/better-auth/src/crypto/password.ts"
        let rust_path = derive_rust_path(&upstream_rel);
        let dir = Path::new(&rust_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let base = src.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        let vendored = dir.join(base);

        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        fs::copy(src, &vendored).map_err(|e| format!("copy {}: {e}", src.display()))?;
        copied += 1;

        // Co-locate the sibling test (`<stem>.test.ts`) as reference too.
        if let Some(stem) = base
            .strip_suffix(".ts")
            .or_else(|| base.strip_suffix(".tsx"))
        {
            let test_name = format!("{stem}.test.ts");
            let test_src = src.with_file_name(&test_name);
            if test_src.is_file() {
                let _ = fs::copy(&test_src, dir.join(&test_name));
                tests += 1;
            }
        }

        let ts_path = vendored.to_string_lossy().replace('\\', "/");
        let loc = count_loc(&vendored);
        let (status, confidence) = prior
            .get(&ts_path)
            .cloned()
            .unwrap_or_else(|| ("todo".to_string(), "-".to_string()));
        rows.push((ts_path, loc, rust_path, status, confidence));
    }

    write_manifest(&rows, &sha)?;
    Ok(format!(
        "vendored {copied} source files (+{tests} test refs) into crates/; wrote {} ({} rows) @ upstream {}",
        MANIFEST_PATH,
        rows.len(),
        sha
    ))
}

/// `manifest`: rewrite the manifest from the co-located `.ts` already in the crates, preserving
/// each row's status/confidence. Use after deleting/porting files; `vendor` adds new files.
fn refresh_manifest() -> Result<String, String> {
    let crates = Path::new("crates");
    if !crates.is_dir() {
        return Err("crates/ not found".into());
    }
    let mut sources: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(crates).map_err(|e| e.to_string())? {
        let p = entry.map_err(|e| e.to_string())?.path();
        let src = p.join("src");
        if src.is_dir() {
            collect_vendored(&src, &mut sources)?;
        }
    }
    sources.sort();

    let prior = load_prior_status();
    let sha = upstream_sha();
    let mut rows = Vec::new();
    for ts in &sources {
        let ts_path = ts.to_string_lossy().replace('\\', "/");
        let loc = count_loc(ts);
        let rust_path = sibling_rust_path(&ts_path);
        let (status, confidence) = prior
            .get(&ts_path)
            .cloned()
            .unwrap_or_else(|| ("todo".to_string(), "-".to_string()));
        rows.push((ts_path, loc, rust_path, status, confidence));
    }
    write_manifest(&rows, &sha)?;
    Ok(format!(
        "refreshed {} ({} rows) @ upstream {}",
        MANIFEST_PATH,
        rows.len(),
        sha
    ))
}

fn write_manifest(
    rows: &[(String, usize, String, String, String)],
    sha: &str,
) -> Result<(), String> {
    fs::create_dir_all("port").map_err(|e| e.to_string())?;
    let utils_sha = utils_upstream_sha();
    let mut out = String::from(MANIFEST_HEADER);
    for (ts, loc, rust, status, conf) in rows {
        // Rows under the utils crate track a different upstream repo (@better-auth/utils).
        let row_sha = if ts.starts_with(UTILS_PREFIX) {
            utils_sha.as_deref().unwrap_or(sha)
        } else {
            sha
        };
        out.push_str(&format!(
            "{ts}\t{loc}\t{rust}\t{status}\t{conf}\t{row_sha}\n"
        ));
    }
    fs::write(MANIFEST_PATH, out).map_err(|e| e.to_string())
}

/// status/confidence keyed by ts_path, so regen never clobbers porting progress.
fn load_prior_status() -> BTreeMap<String, (String, String)> {
    let mut map = BTreeMap::new();
    let Ok(content) = fs::read_to_string(MANIFEST_PATH) else {
        return map;
    };
    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 5 {
            map.insert(
                cols[0].to_string(),
                (cols[3].to_string(), cols[4].to_string()),
            );
        }
    }
    map
}

fn rel_under(root: &str, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|e| e.to_string())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// Collect portable (non-test, non-decl) `.ts` sources from an upstream `packages/` tree.
fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if matches!(
                name.as_str(),
                "node_modules" | "dist" | "__snapshots__" | "test" | "tests"
            ) {
                continue;
            }
            if dir.ends_with("packages") && SKIP_PACKAGES.contains(&name.as_str()) {
                continue;
            }
            // Drop the JS/TS frontend client SDK.
            if name == "client" && path.to_string_lossy().contains("/better-auth/src/") {
                continue;
            }
            collect_sources(&path, out)?;
        } else if is_portable_source(&name) {
            out.push(path);
        }
    }
    Ok(())
}

/// Collect already-co-located `.ts` references from a crate `src/`.
fn collect_vendored(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            collect_vendored(&path, out)?;
        } else if is_portable_source(&name) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_portable_source(name: &str) -> bool {
    let is_ts = name.ends_with(".ts") || name.ends_with(".tsx");
    let excluded = name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".spec.ts")
        || name.ends_with(".d.ts")
        || name.ends_with(".config.ts");
    is_ts && !excluded
}

fn count_loc(path: &Path) -> usize {
    fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// The `.rs` sibling for an already-co-located `.ts` (same dir; snake stem; index.ts→mod.rs,
/// or lib.rs at a crate `src/` root).
fn sibling_rust_path(ts_path: &str) -> String {
    let p = Path::new(ts_path);
    let dir = p
        .parent()
        .map(|d| d.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let base = p.file_name().and_then(|s| s.to_str()).unwrap_or("mod.ts");
    let file = rust_file_name(base, dir.ends_with("/src"));
    if dir.is_empty() {
        file
    } else {
        format!("{dir}/{file}")
    }
}

/// Propose the initial Rust path for an upstream `.ts` (relative path like
/// "packages/<pkg>/src/<rest>.ts").
fn derive_rust_path(rel: &str) -> String {
    let stripped = rel.strip_prefix("packages/").unwrap_or(rel);
    let Some((pkg, after)) = stripped.split_once('/') else {
        return format!("crates/better-auth-rs/src/{}", to_rust_subpath("", rel));
    };
    let inner = after.strip_prefix("src/").unwrap_or(after);
    let (crate_root, sub) = match pkg {
        "core" => ("crates/better-auth-rs-core/src", inner.to_string()),
        "better-auth" => ("crates/better-auth-rs/src", inner.to_string()),
        other => (
            "crates/better-auth-rs/src",
            format!("{}/{}", snake(other), inner),
        ),
    };
    format!("{crate_root}/{}", to_rust_subpath(crate_root, &sub))
}

/// Map an inner subpath ("crypto/password.ts", "index.ts", ...) to its Rust file path.
fn to_rust_subpath(crate_root: &str, sub: &str) -> String {
    let mut parts: Vec<String> = sub.split('/').map(snake).collect();
    let at_root = parts.len() == 1;
    if let Some(last) = parts.last_mut() {
        // index.ts at the crate src root → lib.rs; elsewhere → mod.rs.
        let root_index = at_root && !crate_root.is_empty();
        *last = rust_file_name(last, root_index);
    }
    parts.join("/")
}

/// `index.ts` → `mod.rs` (or `lib.rs` if `root_index`); `<stem>.ts` → `<snake-stem>.rs`.
fn rust_file_name(base: &str, root_index: bool) -> String {
    let stem = base
        .strip_suffix(".ts")
        .or_else(|| base.strip_suffix(".tsx"))
        .unwrap_or(base);
    if stem == "index" {
        if root_index {
            "lib.rs".into()
        } else {
            "mod.rs".into()
        }
    } else {
        let stem = snake(stem);
        // `type` is a Rust reserved keyword (`mod type;` is illegal, `r#type` is avoided), so a
        // `type.ts` source becomes `types.rs` — pluralize the keyword. This is the one
        // reserved-keyword filename in the upstream tree; both `db/type.ts` and
        // `@better-auth/utils`' `type.ts` map to `types.rs` this way.
        let stem = if stem == "type" {
            "types".to_string()
        } else {
            stem
        };
        format!("{stem}.rs")
    }
}

/// kebab-case / camelCase / dotted → snake_case (drops any `.ts`/`.tsx` first).
fn snake(s: &str) -> String {
    let stem = s
        .strip_suffix(".ts")
        .or_else(|| s.strip_suffix(".tsx"))
        .unwrap_or(s);
    let mut out = String::with_capacity(stem.len() + 4);
    let mut prev_lower_or_digit = false;
    for ch in stem.chars() {
        if ch == '-' || ch == ' ' || ch == '.' {
            out.push('_');
            prev_lower_or_digit = false;
        } else if ch.is_ascii_uppercase() {
            if prev_lower_or_digit {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = false;
        } else {
            out.push(ch);
            prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }
    out
}

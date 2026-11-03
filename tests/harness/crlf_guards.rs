//! CRLF guards (ZB-TD41). No new function is added here, so there is no
//! A-Z edge-case list: these tests guard the line-ending behaviour of the
//! parsers and golden files that already exist.
//!
//! The real entry paths are CR-safe on main, so these tests PASS on main
//! and are guards, not bug reproductions (see the PR body for the evidence):
//! every zb test parser is fed through `str::lines()` (which strips a
//! trailing `\r`), `trim()`, or `split_whitespace()` (which never yields a
//! `\r` token). The one genuinely failing red check for this PR is
//! `tools/gitattributes_selftest.sh`, which fails while the repo has no
//! `.gitattributes`. One honest limit, stated plainly:
//! `common::ballrows::parse_row` itself does not trim a raw line — every
//! caller feeds it `lines()` output, so no change is made to it here.

use crate::common::ballrows::{format_row, parse_row};
use crate::common::expect_eq;
use crate::common::goldens::{golden_dir, GOLDEN_FILES};

fn first_data_row(text: &str) -> Result<&str, String> {
    text.lines()
        .find(|l| !l.starts_with('#'))
        .ok_or_else(|| String::from("no data row"))
}

#[test]
fn crlf_ball_row_parses_identically_through_lines() -> Result<(), String> {
    let path = golden_dir().join("ball_exact_ops.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
    let row = first_data_row(&text)?;
    let lf = format_row(&parse_row(row)?)?;
    let crlf_text = format!("{}\r\n", row);
    let crlf_row = first_data_row(&crlf_text)?;
    let crlf = format_row(&parse_row(crlf_row)?)?;
    expect_eq("CRLF row through lines() parses identically to LF row", &crlf, &lf)
}

#[test]
fn crlf_mag_row_tokens_have_no_cr() -> Result<(), String> {
    let path = golden_dir().join("mag_ops.txt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
    let row = first_data_row(&text)?;
    let crlf_text = format!("{}\r\n", row);
    let crlf_row = first_data_row(&crlf_text)?;
    let lf_toks: Vec<&str> = row.split_whitespace().collect();
    let crlf_toks: Vec<&str> = crlf_row.split_whitespace().collect();
    expect_eq(
        "CRLF row split_whitespace tokens equal LF tokens",
        &crlf_toks,
        &lf_toks,
    )?;
    if crlf_toks.iter().any(|t| t.contains('\r')) {
        return Err(String::from("CRLF row token still contains CR"));
    }
    Ok(())
}

fn check_no_cr_in_dir(dir: &std::path::Path) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{:?}: {}", dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {}", e))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| format!("{:?}: {}", path, e))?;
        if kind.is_dir() {
            check_no_cr_in_dir(&path)?;
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".txt") || name == "SHA256SUMS" {
                let text =
                    std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
                if text.contains('\r') {
                    return Err(format!("{:?} contains CR", path));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn golden_files_and_slice_sums_contain_no_cr() -> Result<(), String> {
    for g in GOLDEN_FILES.iter() {
        let path = golden_dir().join(g.name);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{:?}: {}", path, e))?;
        if text.contains('\r') {
            return Err(format!("{:?} contains CR", path));
        }
    }
    check_no_cr_in_dir(&golden_dir())
}

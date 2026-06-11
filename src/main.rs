//! kobold-docgate CLI.
//!   kobold-docgate check   --root <repo>            doc-freshness gate (exit 1 on Fail)
//!   kobold-docgate receipt --root <repo> --out <f>  write the JSON freshness receipt
use kobold_docgate::{check_root, DocVerdict};
use std::process::exit;

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: kobold-docgate <check|receipt> --root <repo> [--out <file>]";
    if args.len() < 2 {
        eprintln!("{usage}");
        exit(2);
    }
    let root = arg_after(&args, "--root").unwrap_or_else(|| ".".into());
    let report = match check_root(&root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("load error: {e}");
            exit(2);
        }
    };
    match args[1].as_str() {
        "check" => {
            println!(
                "kobold-docgate: {:?}  {} courts  docs={:?}",
                report.verdict, report.court_count, report.docs_checked
            );
            for f in &report.findings {
                println!("  - [{}] {} :: {} ({})", f.severity, f.kind, f.doc, f.detail);
            }
            if report.verdict == DocVerdict::Fail {
                println!("GATE: FAIL");
                exit(1);
            }
            println!("GATE: {:?}", report.verdict);
        }
        "receipt" => {
            let json = serde_json::to_string_pretty(&report).unwrap_or_default();
            match arg_after(&args, "--out") {
                Some(out) => {
                    if let Some(p) = std::path::Path::new(&out).parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    if let Err(e) = std::fs::write(&out, &json) {
                        eprintln!("write {out}: {e}");
                        exit(2);
                    }
                    println!("wrote {out}  verdict={:?}", report.verdict);
                }
                None => println!("{json}"),
            }
        }
        _ => {
            eprintln!("{usage}");
            exit(2);
        }
    }
}

#![forbid(unsafe_code)]
//! # kobold-docgate
//!
//! A generated-doc **freshness gate** that emits a receipt. It checks committed documentation against the
//! authoritative claim-ladder, so docs cannot silently drift from the evidence:
//!
//! - **count drift** — a live doc that states "<N> courts" must match the real ladder size (this is the
//!   class of bug a hand-maintained gate misses: you bump the ladder, forget a count in STATUS.md).
//! - **court references** — every non-atlas court should be named somewhere in the docs.
//!
//! Composes [`kobold_courts`] for the claim-ladder model. Part of the KOBOLD ecosystem (Apache-2.0).
//! Dependency rule: kobold-* MAY depend on gnucobol-rs; never the reverse.

use kobold_courts::CourtSet;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocVerdict {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocFinding {
    pub doc: String,
    pub kind: String,
    pub severity: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocGateReport {
    pub schema: String,
    pub court_count: usize,
    pub docs_checked: Vec<String>,
    pub verdict: DocVerdict,
    pub findings: Vec<DocFinding>,
    pub non_claims: Vec<String>,
}

/// A court is excluded from the "must be referenced" requirement if its id marks an observed atlas / meta
/// court (these are not required to appear in the headline docs).
fn reference_excluded(id: &str) -> bool {
    id.contains("ATLAS") || id.contains("DOCGATE") || id.contains("TRUST") || id.contains("BUILD.PROFILE")
}

/// Find every integer that immediately precedes the word `court`/`courts` in `text` (regex-free).
pub fn stated_court_counts(text: &str) -> Vec<u64> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = Vec::new();
    let mut search = 0;
    while let Some(rel) = lower[search..].find("court") {
        let idx = search + rel;
        // walk back over spaces, then collect a preceding integer
        let mut i = idx;
        while i > 0 && bytes[i - 1] == b' ' {
            i -= 1;
        }
        let end = i;
        while i > 0 && bytes[i - 1].is_ascii_digit() {
            i -= 1;
        }
        if i < end {
            if let Ok(n) = lower[i..end].parse::<u64>() {
                out.push(n);
            }
        }
        search = idx + 5;
    }
    out
}

/// Check a set of `(doc_name, text)` against the claim-ladder. `live_docs` (e.g. README, STATUS) are held
/// to the strict count rule; references are checked across all docs.
pub fn check(set: &CourtSet, docs: &[(String, String)], live_docs: &[&str]) -> DocGateReport {
    let count = set.ladder.courts.len();
    let mut findings = Vec::new();

    // count drift (live docs only, to avoid flagging historical changelog mentions)
    for (name, text) in docs {
        if live_docs.iter().any(|d| name.contains(d)) {
            for n in stated_court_counts(text) {
                if n as usize != count {
                    findings.push(DocFinding {
                        doc: name.clone(),
                        kind: "stale_court_count".into(),
                        severity: "high".into(),
                        detail: format!("states {n} courts; claim-ladder has {count}"),
                    });
                }
            }
        }
    }

    // court references (across all docs)
    let all: String = docs.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join("\n");
    for c in &set.ladder.courts {
        if !reference_excluded(&c.id) && !all.contains(&c.id) {
            findings.push(DocFinding {
                doc: "(any)".into(),
                kind: "unreferenced_court".into(),
                severity: "medium".into(),
                detail: format!("court {} is not named in any doc", c.id),
            });
        }
    }

    let verdict = if findings.iter().any(|f| f.severity == "high") {
        DocVerdict::Fail
    } else if findings.is_empty() {
        DocVerdict::Pass
    } else {
        DocVerdict::Warn
    };

    DocGateReport {
        schema: "kobold-docgate-receipt-v1".into(),
        court_count: count,
        docs_checked: docs.iter().map(|(n, _)| n.clone()).collect(),
        verdict,
        findings,
        non_claims: [
            "checks declared docs only, not all prose",
            "reference match is substring-based, not semantic",
            "not a substitute for human review of doc accuracy",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
    }
}

/// Convenience: load `<root>/reports/claim-ladder.json` + the standard live docs and check.
pub fn check_root(root: impl AsRef<Path>) -> Result<DocGateReport, Box<dyn std::error::Error>> {
    let root = root.as_ref();
    let set = CourtSet::load_root(root)?;
    let mut docs = Vec::new();
    for name in ["README.md", "STATUS.md", "CHANGELOG.md"] {
        if let Ok(t) = std::fs::read_to_string(root.join(name)) {
            docs.push((name.to_string(), t));
        }
    }
    Ok(check(&set, &docs, &["README.md", "STATUS.md"]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder(n: usize) -> CourtSet {
        let courts: Vec<String> = (0..n)
            .map(|i| format!(r#"{{"id":"GNURUST.{i}","proven":"p","not_proven":"a; b"}}"#))
            .collect();
        CourtSet::from_json(&format!(r#"{{"courts":[{}]}}"#, courts.join(","))).unwrap()
    }

    #[test]
    fn counts_are_extracted() {
        assert_eq!(stated_court_counts("now 98 courts sealed"), vec![98]);
        assert_eq!(stated_court_counts("1 court; and 42 courts"), vec![1, 42]);
        assert!(stated_court_counts("no number here courts").is_empty());
    }

    #[test]
    fn stale_count_in_live_doc_fails() {
        let set = ladder(98);
        let docs = vec![("STATUS.md".into(), "the suite has 90 courts today".into())];
        let r = check(&set, &docs, &["STATUS.md"]);
        assert_eq!(r.verdict, DocVerdict::Fail);
        assert!(r.findings.iter().any(|f| f.kind == "stale_court_count"));
    }

    #[test]
    fn correct_count_and_refs_pass() {
        let set = ladder(2);
        let docs = vec![(
            "README.md".into(),
            "2 courts: GNURUST.0 and GNURUST.1 are sealed".into(),
        )];
        let r = check(&set, &docs, &["README.md"]);
        assert_eq!(r.verdict, DocVerdict::Pass, "{:?}", r.findings);
    }

    #[test]
    fn unreferenced_court_warns() {
        let set = ladder(2);
        let docs = vec![("README.md".into(), "2 courts; only GNURUST.0 named".into())];
        let r = check(&set, &docs, &["README.md"]);
        assert_eq!(r.verdict, DocVerdict::Warn);
        assert!(r.findings.iter().any(|f| f.kind == "unreferenced_court"));
    }
}

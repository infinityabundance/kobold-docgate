# kobold-docgate

A generated-doc **freshness gate** that emits a receipt. It checks committed documentation against the
authoritative claim-ladder so docs can't silently drift from the evidence.

## What it checks (v0.1)
- **count drift** — a live doc (README/STATUS) that states "*N* courts" must match the real ladder size.
  This is the bug a hand-maintained gate misses: you bump the ladder and forget a count in `STATUS.md`.
  (`Fail`.)
- **court references** — every non-atlas court should be named somewhere in the docs. (`Warn`.)

Emits a `kobold-docgate-receipt-v1` JSON receipt with an explicit non-claims list.

## CLI
```
kobold-docgate check   --root <repo>                       # exit 1 on Fail (stale count)
kobold-docgate receipt --root <repo> --out reports/docgate-receipt.json
```

It loads `<repo>/reports/claim-ladder.json` + `README.md`/`STATUS.md`/`CHANGELOG.md`.

## Composition
Built on `kobold-courts` (the claim-ladder model). Part of the KOBOLD ecosystem; `kobold-* MAY depend on
gnucobol-rs, never the reverse`. A consumer (e.g. gnucobol-rs) runs it in CI and commits the receipt as
evidence — the doc-freshness METHOD lives here, not in the consumed repo.

Roadmap: full generated-doc rendering/regeneration parity (STATUS/README/CHANGELOG models), per-doc
exclusion config, broader staleness signals (kani/fuzz counts, version strings).

## License
Apache-2.0 (see LICENSE).

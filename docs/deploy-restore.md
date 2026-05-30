# Deploy restore manifest — 2026-05-30

Snapshot of what is **about to be deployed**, and how to **roll back** to the
previous state if anything goes wrong. Read top to bottom before touching prod.

## Restore points captured

| Mechanism | Identifier | What it pins |
|---|---|---|
| Git tag | `deploy-baseline-2026-05-30` | Commit `8fa515b` — the last fully committed state (radiosity engine only, no SPD work) |
| Branch | `street-design` (current) | Will move forward when the SPD commit lands; tag stays put |

Verify:

```sh
git tag -l deploy-baseline-2026-05-30      # exists?
git show deploy-baseline-2026-05-30 --no-patch --format="%h %s"
# → 8fa515b [CALC] Patch-radiosity CU engine with normative Stockmar grid + DIAL parity
```

## What is currently ACTIVE (i.e. what will deploy)

### At HEAD = `8fa515b` (committed, will deploy as-is)

The radiosity work from the previous session:

- `crates/eulumdat/src/radiosity.rs` — patch-radiosity solver + Stockmar/CIE grid
- `crates/eulumdat/src/lib.rs` — radiosity re-exports
- `crates/eulumdat/tests/dial_parity.rs` + `tests/radiosity_cie171.rs`
- `crates/eulumdat/tests/fixtures/{alya_359lm.ldt, acrux_452lm.ldt, dial_discrepancy.csv}`
- `crates/eulumdat/examples/{dial_parity_csv.rs, radiosity_grid_demo.rs}`
- `docs/radiosity-cu.md` + `docs/dial_parity_measured.csv`

### Uncommitted on disk — the SPD work to be committed before deploy

Two existing files modified:

| File | Change |
|---|---|
| `crates/eulumdat/src/atla/mod.rs` | `+pub mod colorimetry;` + `+pub mod spd_loader;` |
| `crates/eulumdat/src/lib.rs` | `+pub use atla::colorimetry::{analyze as analyze_spd, Colorimetry};` + `+pub use atla::spd_loader::{load as load_spd, parse as parse_spd, LoadedSpd, ReferenceMetrics, SpdLoadError};` |

Four new files:

| File | Purpose |
|---|---|
| `crates/eulumdat/src/atla/spd_loader.rs` | Multi-format SPD loader (Luxeon `.spd`, named CSV, Signify lab CSV with metric prelude, two-column fallback) |
| `crates/eulumdat/src/atla/colorimetry.rs` | CIE 1931 CMF + tristimulus + 1931/1960/1976 chromaticity + Robertson CCT + Duv + dominant λ + peak/FWHM |
| `crates/eulumdat/examples/spd_colorimetry_corpus.rs` | Walks `docs/SPDs/` and prints ours vs Signify reference |
| `docs/spd-colorimetry.md` | Module documentation |

### NOT being deployed (untracked, unrelated to this work — leave on disk, don't commit)

These files are in the working tree but are not part of the SPD deploy:

- `crates/eulumdat-wasm/src/library.rs` — in-flight WASM work, separate concern
- `docs/Compare_disccrepancies.numbers` — your source spreadsheet (binary)
- `docs/SPDs/` — the real-world SPD corpus (used by tests via path; not modified)
- Modified i18n locale files (7 JSONs), `Cargo.lock`, and `crates/eulumdat-wasm/*` —
  unrelated in-progress work

## Test parity at HEAD (the gate before deploy)

These ran green just before this manifest was written:

```
cargo test -p eulumdat
  → 367 lib tests, 4 dial_parity, 3 radiosity_cie171, 9 + 3 + 3 + 1 + 16 integration suites
  → ALL PASS
```

After the SPD commit lands, the same command must still pass, plus the 5
loader + 3 colorimetry tests (already verified locally).

## Roll back: scenarios

### A. Roll back just the SPD work (keep radiosity)

Most likely scenario — SPD modules misbehave in prod, radiosity is fine.

```sh
git checkout deploy-baseline-2026-05-30 -- \
    crates/eulumdat/src/atla/mod.rs \
    crates/eulumdat/src/lib.rs

rm crates/eulumdat/src/atla/colorimetry.rs \
   crates/eulumdat/src/atla/spd_loader.rs \
   crates/eulumdat/examples/spd_colorimetry_corpus.rs \
   docs/spd-colorimetry.md

cargo test -p eulumdat   # confirm parity with baseline
```

### B. Roll back to the baseline tag entirely (nuke the SPD commit AND any later work on the branch)

Use only if everything since the baseline must go.

```sh
git reset --hard deploy-baseline-2026-05-30
```

⚠ This is destructive. Push only with team confirmation; never to `main`.

### C. Just keep the tag, don't roll back

If nothing goes wrong, the tag remains as a known-good marker for future
comparisons. Leave it. It costs nothing.

## Pre-deploy checklist

- [ ] `cargo test -p eulumdat` green (≥367 + 5 + 3 + others)
- [ ] `git tag -l deploy-baseline-2026-05-30` exists
- [ ] SPD commit made, message references this manifest
- [ ] This file (`docs/deploy-restore.md`) included in the SPD commit so the
      restore steps ship with the code that needs restoring
- [ ] No accidental staging of `eulumdat-wasm/*`, locale JSONs, or `Cargo.lock`
      (verify with `git diff --staged --stat`)

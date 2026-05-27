# CI: replace `cargo test` with `cargo llvm-cov` coverage reporting

**Date:** 2026-05-27
**Status:** Approved
**Scope:** `.github/workflows/ci.yml` only

## Goal

Switch the `test` job's bare `cargo test` step to a `cargo llvm-cov`-based
coverage run, and emit the coverage report into the GitHub Actions job
summary (plus downloadable artifacts and an optional Codecov upload). The
reference for the desired shape is the `coverage` job in `~/src/rc-voting`'s
`.github/workflows/rust.yml`; this spec adapts that pattern to
email-normalizer's simpler context (GitHub-hosted `ubuntu-latest`, single
library crate, no feature flags, no Docker, no self-hosted runner).

## Why

`cargo llvm-cov` runs the same test binaries as `cargo test` but with
LLVM source-based coverage instrumentation. We get the same correctness
signal we have today, plus per-file line/region/function coverage on every
PR. This catches:

- Untested branches added in a PR (visible in the per-file diff in the
  job summary).
- Coverage regressions across the codebase (enforced by a
  `--fail-under-lines` threshold).

Baseline measured locally on 2026-05-27: **74.29% line coverage**
(`lib.rs` 67.86%, `rules.rs` 100%). Threshold set to **71** —
~3.3-point cushion below baseline, mirroring rc-voting's 79 vs 82.52
ratio.

## Non-goals

- No restructuring of the existing `test` job into multiple jobs. rc-voting
  splits `coverage` from `clippy-ssr` / `check-hydrate` / `fmt-check` /
  `e2e` because it has ~5 parallel jobs already. email-normalizer has one;
  splitting would add ~30s of startup overhead with no benefit.
- No `--ignore-filename-regex` (`COVERAGE_IGNORE` env in rc-voting).
  Every file the library ships is intended to be covered; the test module
  itself is `#[cfg(test)]`-gated and already excluded by
  `cargo-llvm-cov`'s defaults.
- No changes to `cargo fmt --check`, `cargo clippy`, or `cargo doc`
  steps. Coverage replaces *only* the `cargo test` step.
- No Codecov account / token setup as part of this change. The upload
  step is wired but conditional — it no-ops cleanly when the secret is
  absent and starts working as soon as someone adds it.

## Design

### File modified

`.github/workflows/ci.yml` only.

### Job structure

Single job, name unchanged (`test`), `runs-on: ubuntu-latest`. Steps
execute in the order below; any failure short-circuits subsequent steps
(default GitHub Actions behavior).

1. `actions/checkout@v4`
2. `dtolnay/rust-toolchain@stable` with `rustfmt, clippy` components
3. `Swatinem/rust-cache@v2`
4. `cargo fmt --all -- --check`
5. `cargo clippy --all-targets -- -D warnings`
6. **NEW:** Install `cargo-llvm-cov` via `taiki-e/install-action@v2`
   (pre-built binary; ~3s)
7. **NEW:** `cargo llvm-cov clean --workspace` — defensive against any
   stale `.profraw` files restored by `Swatinem/rust-cache`
8. **NEW:** `cargo llvm-cov --no-report --workspace` — instrumented test
   run (replaces the bare `cargo test`)
9. **NEW:** Emit `lcov.info` via `cargo llvm-cov report --lcov`
10. **NEW:** Emit `cobertura.xml` via `cargo llvm-cov report --cobertura`
11. **NEW:** Emit `codecov.json` via `cargo llvm-cov report --codecov`
12. **NEW:** Emit `coverage-html/` via `cargo llvm-cov report --html`
13. **NEW:** `cargo llvm-cov report --fail-under-lines $COVERAGE_THRESHOLD`
14. **NEW:** Append per-file coverage summary to `$GITHUB_STEP_SUMMARY`
    (`if: always()`)
15. **NEW:** `actions/upload-artifact@v4` × 3 — HTML report (path
    `coverage-html/html/` so `index.html` is at the artifact root),
    `lcov.info`, `cobertura.xml`; all `if: always()`, retention 30 days
16. **NEW:** `codecov/codecov-action@v5` — conditional on
    `env.CODECOV_TOKEN != ''`, `fail_ci_if_error: false`,
    `files: ./codecov.json`
17. `cargo doc --no-deps` with `RUSTDOCFLAGS: -D warnings` (unchanged)

### Environment

Job-level `env:` block adds:

```yaml
env:
  COVERAGE_THRESHOLD: 71
  CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}
```

`CODECOV_TOKEN` is declared at job level so the step-level
`if: env.CODECOV_TOKEN != ''` predicate can evaluate it. This mirrors
the workaround documented in rc-voting's workflow.

### Threshold management

`COVERAGE_THRESHOLD: 71` lives in the workflow `env` block, so bumping it
in lockstep with future coverage pushes is a one-line edit. The exact
value is justified inline with a comment referencing the 2026-05-27
baseline (74.29%) so future readers know where it came from.

### Step summary format

The summary step writes a markdown fragment like:

```
## Coverage report

Threshold: ≥ 71% line coverage

```
Filename                      Regions    Missed Regions     Cover ...
lib.rs                             85                22    74.12% ...
rules.rs                           16                 0   100.00% ...
TOTAL                             101                22    78.22% ...
```
```

The fenced block contains the raw `cargo llvm-cov report` table.
`if: always()` means the summary lands even when the threshold step
fails — so a coverage regression PR shows both the failure and the
per-file numbers in one place.

### Action version pins

| Action | Version | Notes |
|---|---|---|
| `actions/checkout` | v4 | already used in current workflow |
| `dtolnay/rust-toolchain` | stable | already used |
| `Swatinem/rust-cache` | v2 | already used |
| `taiki-e/install-action` | v2 | new — for `cargo-llvm-cov` |
| `actions/upload-artifact` | v4 | new — current GHA-hosted runner stable |
| `codecov/codecov-action` | v5 | new — matches rc-voting |

(rc-voting uses `actions/checkout@v6` and `actions/upload-artifact@v7`
because it runs on a self-hosted runner with a different action allowlist;
this repo stays on v4/v4 to match what's already wired here.)

## Verification

1. `act` or push to a branch — confirm CI passes end-to-end.
2. Inspect the job summary: per-file coverage table renders, lcov /
   cobertura / HTML artifacts are downloadable.
3. Locally simulate a regression: drop a covered test, push, confirm CI
   fails at the `--fail-under-lines` step *and* the summary still renders.
4. With `CODECOV_TOKEN` absent (current state): confirm the
   `codecov/codecov-action` step is skipped, not failed.

## Risks

- **`Swatinem/rust-cache` + instrumented builds**: the cache restores
  `target/`, which can contain `.profraw` files from a previous run with
  different test binaries. `cargo llvm-cov clean --workspace` runs first
  to flush them. Low risk in practice — same pattern rc-voting uses.
- **Threshold drift**: 71 is below the 74.29% baseline, so the first CI
  run will pass. If someone adds significant new untested code in a
  single PR, the threshold won't catch it (the absolute floor is what's
  enforced, not the delta). Acceptable for a small library; we can
  layer codecov's PR-comment delta tracking on later if needed.
- **`taiki-e/install-action` availability**: this is the canonical
  pre-built-binary installer for `cargo-llvm-cov` and is well-maintained.
  If it ever breaks, the fallback is `cargo install cargo-llvm-cov`
  (~2 minutes vs ~3 seconds).

## Decisions captured from brainstorming

- Q1: "Measure first, set threshold" → baseline 74.29%, threshold 71.
- Q2: "Same as rc-voting: conditional upload" → codecov-action gated on
  `env.CODECOV_TOKEN != ''`.
- Q3: "71 (3-pt cushion under 74.29%)" → `COVERAGE_THRESHOLD: 71`.

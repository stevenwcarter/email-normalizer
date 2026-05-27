# CI Coverage Reporting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the bare `cargo test` step in `.github/workflows/ci.yml` with a `cargo llvm-cov` coverage run that emits per-file coverage to the GitHub Actions job summary, uploads HTML/lcov/cobertura artifacts, and conditionally uploads `codecov.json` to codecov.io when `CODECOV_TOKEN` is configured.

**Architecture:** Single-job edit. The existing `test` job stays on `ubuntu-latest` with `Swatinem/rust-cache@v2`. fmt/clippy/doc steps stay unchanged. Only the `cargo test` step gets swapped for an instrumented `cargo llvm-cov` sequence, plus a job-level `env` block for `COVERAGE_THRESHOLD` and `CODECOV_TOKEN`.

**Tech Stack:** GitHub Actions, `cargo-llvm-cov` (installed via `taiki-e/install-action@v2`), `codecov/codecov-action@v5`, `actions/upload-artifact@v4`.

**Spec:** [`docs/superpowers/specs/2026-05-27-ci-coverage-design.md`](../specs/2026-05-27-ci-coverage-design.md)

---

## Task 1: Rewrite `.github/workflows/ci.yml` with coverage steps

**Files:**
- Modify: `.github/workflows/ci.yml` (full rewrite of the `test` job's body)

CI workflow changes can't be unit-tested. The verification loop is:
1. Validate YAML syntax with `yq`.
2. Eyeball-check action references and step order against the spec.
3. Cross-check the local `cargo llvm-cov` baseline matches the threshold reasoning (71 vs measured 74.29%).

The full file content is given verbatim below — paste it in one edit.

- [ ] **Step 1: Confirm current workflow content matches the baseline this plan was written against**

Run: `cat .github/workflows/ci.yml`

Expected: matches the "Current state" block below. If it doesn't, STOP and re-read the spec — someone else has modified the file since the plan was written.

Current state (37 lines):

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: cargo fmt --check
        run: cargo fmt --all -- --check

      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: cargo test
        run: cargo test

      - name: cargo doc
        env:
          RUSTDOCFLAGS: -D warnings
        run: cargo doc --no-deps
```

- [ ] **Step 2: Re-measure local coverage baseline (sanity check the threshold)**

Run: `cargo llvm-cov clean --workspace && cargo llvm-cov --workspace 2>&1 | tail -10`

Expected: the `TOTAL` line shows line coverage at or near **74.29%**. If it's now below 71%, STOP — the threshold needs to drop before this lands or the first CI run will fail. (As of 2026-05-27 the baseline is 74.29%.)

- [ ] **Step 3: Replace `.github/workflows/ci.yml` with the new content**

Overwrite the file with exactly this content:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

jobs:
  test:
    runs-on: ubuntu-latest
    env:
      # Baseline 74.29% line coverage on 2026-05-27 (see
      # docs/superpowers/specs/2026-05-27-ci-coverage-design.md).
      # 71 gives ~3.3-point cushion; bump in lockstep with future
      # coverage pushes.
      COVERAGE_THRESHOLD: 71
      # Job-level so step-level `if: env.CODECOV_TOKEN != ''` evaluates.
      CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy, llvm-tools-preview

      - uses: Swatinem/rust-cache@v2

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-llvm-cov

      - name: cargo fmt --check
        run: cargo fmt --all -- --check

      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings

      - name: Clean stale coverage data
        run: cargo llvm-cov clean --workspace

      - name: Collect coverage (no report yet)
        run: cargo llvm-cov --no-report --workspace

      - name: Emit lcov
        run: cargo llvm-cov report --lcov --output-path lcov.info

      - name: Emit cobertura
        run: cargo llvm-cov report --cobertura --output-path cobertura.xml

      - name: Emit codecov json
        run: cargo llvm-cov report --codecov --output-path codecov.json

      - name: Emit HTML report
        run: cargo llvm-cov report --html --output-dir coverage-html

      - name: Enforce threshold
        run: cargo llvm-cov report --fail-under-lines "$COVERAGE_THRESHOLD"

      - name: Write coverage summary to GHA step summary
        if: always()
        run: |
          {
            echo '## Coverage report'
            echo ''
            echo 'Threshold: ≥ '"${COVERAGE_THRESHOLD}"'% line coverage'
            echo ''
            echo '```'
            cargo llvm-cov report
            echo '```'
          } >> "$GITHUB_STEP_SUMMARY"

      - name: Upload HTML report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: coverage-html-report
          # cargo-llvm-cov puts the report at <output-dir>/html/, so
          # upload the inner dir to keep index.html at the artifact root.
          path: coverage-html/html/
          retention-days: 30

      - name: Upload lcov
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: coverage-lcov
          path: lcov.info
          retention-days: 30

      - name: Upload cobertura
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: coverage-cobertura
          path: cobertura.xml
          retention-days: 30

      - name: Upload to codecov.io
        if: env.CODECOV_TOKEN != ''
        uses: codecov/codecov-action@v5
        with:
          files: ./codecov.json
          token: ${{ secrets.CODECOV_TOKEN }}
          fail_ci_if_error: false

      - name: cargo doc
        env:
          RUSTDOCFLAGS: -D warnings
        run: cargo doc --no-deps
```

Notes on diff vs. current file:
- Added `env:` block at job level (`COVERAGE_THRESHOLD`, `CODECOV_TOKEN`).
- Added `llvm-tools-preview` to the toolchain components (required by `cargo llvm-cov`).
- Added `Install cargo-llvm-cov` step before fmt (placed early so the install runs in parallel with the rust-cache warm-up cost; order doesn't otherwise matter).
- Replaced `cargo test` step with 11 new steps (clean → collect → 4× emit → threshold → summary → 3× upload artifact → conditional codecov).
- `cargo doc` step unchanged, still last.

- [ ] **Step 4: Validate YAML syntax**

Run: `yq '.' .github/workflows/ci.yml > /dev/null && echo OK`

Expected: prints `OK`. Any parse error means a typo in the rewrite — re-read Step 3 and fix.

- [ ] **Step 5: Cross-check step count and key step names**

Run:
```bash
yq '.jobs.test.steps | length' .github/workflows/ci.yml
yq '.jobs.test.steps[] | .name // .uses' .github/workflows/ci.yml
```

Expected:
- Step count: **19**
- Step list (in order — `.name` if present, else `.uses`):
  ```
  actions/checkout@v4
  dtolnay/rust-toolchain@stable
  Swatinem/rust-cache@v2
  Install cargo-llvm-cov
  cargo fmt --check
  cargo clippy
  Clean stale coverage data
  Collect coverage (no report yet)
  Emit lcov
  Emit cobertura
  Emit codecov json
  Emit HTML report
  Enforce threshold
  Write coverage summary to GHA step summary
  Upload HTML report
  Upload lcov
  Upload cobertura
  Upload to codecov.io
  cargo doc
  ```

- [ ] **Step 6: Verify env block is at job level, not workflow level**

Run: `yq '.jobs.test.env' .github/workflows/ci.yml`

Expected:
```yaml
COVERAGE_THRESHOLD: 71
CODECOV_TOKEN: ${{ secrets.CODECOV_TOKEN }}
```

Run: `yq '.env' .github/workflows/ci.yml`

Expected: `null` (workflow-level env block must not exist — the spec requires job-level so the step `if:` predicate evaluates).

- [ ] **Step 7: Verify the threshold value is wired into both the `env` block and not accidentally hardcoded elsewhere**

Run: `grep -nE '\b(71|COVERAGE_THRESHOLD)\b' .github/workflows/ci.yml`

Expected: every match of `71` is a comment ("Baseline 74.29%" / "3.3-point cushion") or the `COVERAGE_THRESHOLD: 71` env definition. The `Enforce threshold` step must reference `"$COVERAGE_THRESHOLD"`, NOT a hardcoded `71`.

- [ ] **Step 8: Verify the codecov upload step is conditional on the token, not unconditional**

Run: `yq '.jobs.test.steps[] | select(.name == "Upload to codecov.io")' .github/workflows/ci.yml`

Expected: the step's `if:` is `env.CODECOV_TOKEN != ''`. If `if:` is missing, the codecov step will run on every build and fail when the secret is absent.

- [ ] **Step 9: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
Replace cargo test with cargo-llvm-cov coverage run in CI

Adapts the rc-voting coverage job pattern to email-normalizer's
GitHub-hosted runner. The instrumented run emits a per-file coverage
table to the GitHub Actions job summary, uploads HTML/lcov/cobertura
artifacts (30-day retention), and forwards codecov.json to codecov.io
when CODECOV_TOKEN is configured.

Threshold COVERAGE_THRESHOLD=71 gives a ~3.3-point cushion below the
measured baseline of 74.29% line coverage on 2026-05-27.

Refs: docs/superpowers/specs/2026-05-27-ci-coverage-design.md
EOF
)"
```

Expected: clean commit on `main`.

---

## Spec coverage self-check

| Spec requirement | Task / Step |
|---|---|
| Replace `cargo test` with `cargo llvm-cov` | Task 1, Step 3 (lines for `Collect coverage (no report yet)`) |
| Job-level `env` for `COVERAGE_THRESHOLD` and `CODECOV_TOKEN` | Task 1, Step 3 + Step 6 verification |
| `llvm-tools-preview` toolchain component | Task 1, Step 3 (added to `dtolnay/rust-toolchain` `components`) |
| `taiki-e/install-action@v2` for cargo-llvm-cov | Task 1, Step 3 |
| `cargo llvm-cov clean --workspace` before collect | Task 1, Step 3 |
| Emit lcov / cobertura / codecov json / HTML | Task 1, Step 3 |
| `--fail-under-lines $COVERAGE_THRESHOLD` | Task 1, Step 3 + Step 7 verification |
| Step summary with `if: always()` and per-file table | Task 1, Step 3 |
| 3× `actions/upload-artifact@v4` with `if: always()`, 30d retention | Task 1, Step 3 |
| Conditional codecov upload with `if: env.CODECOV_TOKEN != ''` | Task 1, Step 3 + Step 8 verification |
| `fail_ci_if_error: false` on codecov step | Task 1, Step 3 |
| `cargo fmt --check`, `cargo clippy`, `cargo doc` unchanged | Task 1, Step 3 (verified against current state in Step 1) |
| No `--ignore-filename-regex` (no `COVERAGE_IGNORE`) | Task 1, Step 3 (absence) |
| No feature flags on `cargo llvm-cov --workspace` | Task 1, Step 3 |

All spec requirements covered.

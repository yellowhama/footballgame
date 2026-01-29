# CI Testing Guide (5-Pack Contract Enforcement)

**Last Updated**: 2025-12-23
**Status**: P1.0-C Implementation

---

## Overview

The CI 5-Pack is a comprehensive test suite that enforces all critical contracts:

1. **PACK 1**: Rust Unit Tests (UID + Attributes)
2. **PACK 2**: Property-Based Tests
3. **PACK 3**: Snapshot Tests
4. **PACK 4**: Integration Tests
5. **PACK 5**: Contract Verification (CI Gates)

---

## Quick Start

### Run All Tests

```bash
./scripts/run_tests.sh
```

### Run with Strict Attributes Mode

```bash
STRICT_ATTRIBUTES=1 ./scripts/run_tests.sh
```

### Run with Godot Tests (if Godot installed)

```bash
GODOT_BIN=/path/to/godot ./scripts/run_tests.sh
```

### Run with Benchmarks

```bash
./scripts/run_tests.sh --bench
```

### Run with Coverage Report

```bash
./scripts/run_tests.sh --coverage
```

---

## PACK 1: Rust Unit Tests

### 1.1. UID Normalization (P0.75-1)

**Test**: `person_cache::tests::parse_person_uid_accepts_all_supported_forms`

**Contract**:
- `csv:123` → `123` (u32)
- `csv_123` → `123` (u32)
- `123` → `123` (u32)
- Invalid formats → `Err`

**Run**:
```bash
cargo test -p of_core --lib person_cache::tests
```

### 1.2. Attributes Guard (P0.5 + P1.0-B)

**Test**: `match_setup::tests::match_setup_debug_missing_attributes_zero_when_all_have_attrs`

**Contract**:
- All players MUST have `attributes = Some(...)`
- `missing_attributes_count == 0` required
- Violation → CI FAIL

**Run**:
```bash
cargo test -p of_core --lib match_setup::tests
```

### 1.3. Strict Mode (P1.0-B)

**Test**: `match_setup::tests::strict_mode_panics_on_missing_attributes`

**Contract**:
- With `strict_attributes` feature: `expect()` panic on `None`
- Without feature: `unwrap_or_default()` + warning

**Run**:
```bash
cargo test -p of_core --lib --features strict_attributes match_setup::tests::strict_mode_panics_on_missing_attributes
```

---

## PACK 2: Property-Based Tests

**Purpose**: Fuzzing/generative testing for edge cases

**Run**:
```bash
cargo test -p of_core story_property_tests
```

**Environment**:
```bash
export PROPTEST_CASES=100  # Adjust for more thorough testing
```

---

## PACK 3: Snapshot Tests

**Purpose**: Regression detection via output snapshots

**Run**:
```bash
cargo test -p of_core story_snapshot_tests
```

**Update Snapshots**:
```bash
cargo insta review
```

---

## PACK 4: Integration Tests

**Purpose**: End-to-end workflows (match simulation, etc.)

**Run**:
```bash
cargo test -p of_core --test '*'
```

---

## PACK 5: Contract Verification

### 5.1. UID Resolve Contract (ENGINE_CONTRACT 6.2)

**Contract**:
- All roster UIDs MUST resolve before simulation
- Preflight validation required
- Unresolvable UID → simulation REJECTED

**Status**: ✅ Enforced by `_preflight_match_setup_roster()`
**TODO**: Add dedicated CI test

### 5.2. Attributes None Ratio

**Contract**:
- `attributes_none_ratio == 0` (no silent default(50) fallback)
- Enforced by PACK 1.2 test

**Status**: ✅ Verified by `missing_attributes_count == 0`

### 5.3. Simulation Lock Release (P0.75-3)

**Contract**:
- Lock MUST be released on ALL exit paths (success, error, abort)
- 10 exit paths verified (MatchManager + MatchSimulationManager)

**Tests**: `tests/test_simulation_lock_release.gd` (6 test cases)

**Status**: ✅ Implemented (GUT tests)
**TODO**: Integrate GUT runner into CI script

**Manual Run** (if Godot available):
```bash
godot --headless --script tests/test_simulation_lock_release.gd
```

---

## CI Integration

### GitHub Actions Example

```yaml
name: CI 5-Pack

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run CI 5-Pack
        run: |
          STRICT_ATTRIBUTES=1 ./scripts/run_tests.sh

      - name: Upload Coverage
        if: matrix.coverage
        uses: codecov/codecov-action@v3
```

### Local Pre-Commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

echo "Running CI 5-Pack (fast mode)..."
cargo test -p of_core --lib person_cache::tests --quiet
cargo test -p of_core --lib match_setup::tests --quiet

if [ $? -ne 0 ]; then
    echo "❌ Tests failed - commit rejected"
    exit 1
fi

echo "✅ Tests passed"
```

---

## Troubleshooting

### Test Failures

**UID Parser Test Fails**:
```
Check: crates/of_core/src/data/person_cache.rs:109-118
Ensure: parse_person_uid() handles all 3 formats
```

**Attributes Guard Test Fails**:
```
Check: scripts/core/PlayerLibrary.gd:_derive_player_attributes()
Ensure: All players get attributes injected (P0.75-2)
```

**Lock Release Test Fails**:
```
Check: autoload/domain/MatchManager.gd lines 226, 238, 247, 258, 270
Check: autoload/domain/MatchSimulationManager.gd lines 993, 1046, 1060, 1092, 1108
Ensure: SimulationLock.release() called on ALL exit paths
```

### Environment Setup

**Godot Not Found**:
```bash
export GODOT_BIN=/path/to/godot
# Or install system-wide: sudo apt install godot3 (Linux)
```

**Rust Toolchain**:
```bash
rustup update stable
cargo --version  # Should be 1.70+
```

---

## Contract Enforcement Summary

| Contract | Test | Status | CI Gate |
|----------|------|--------|---------|
| UID Normalization | PACK 1.1 | ✅ | PASS required |
| Attributes None Ratio | PACK 1.2 | ✅ | PASS required |
| Strict Attributes | PACK 1.3 | ✅ | Optional (env flag) |
| Lock Release | PACK 5.3 | ✅ | TODO: GUT integration |
| Property Invariants | PACK 2 | ✅ | PASS required |
| Snapshot Regression | PACK 3 | ✅ | PASS required |
| Integration E2E | PACK 4 | ✅ | PASS required |

---

## Next Steps (P2.0+)

- [ ] Integrate GUT runner into `run_tests.sh`
- [ ] Add dedicated UID resolve preflight test
- [ ] Add coordinate sanity check (position bounds)
- [ ] Add actor/probability sanity check
- [ ] Add tactical watchdog tests (ENGINE_CONTRACT 6.x)
- [ ] Enable strict_attributes by default in CI

---

## References

- Contract Spec: `docs/specs/FIX_2512/1223/UID_NORMALIZATION_AND_ATTRIBUTE_INJECTION_CONTRACT.md`
- Engine Contract: `docs/specs/SSOT/ENGINE_SSOT_AND_CI_CONTRACT.md`
- Test Implementation: `tests/test_simulation_lock_release.gd`

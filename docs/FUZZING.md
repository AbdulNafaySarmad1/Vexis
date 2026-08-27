# Fuzzing

The decoder entry point must **never panic** on arbitrary bytes, and every
successful decode must report `len ∈ 1..=15` without overrunning its input.

## Targets

| Target | Entry point | Invariant |
|--------|-------------|-----------|
| `decode` | `decoder::decode` | no panic; `1 <= len <= 15`; `len <= input`; `bytes.len() == len` |
| `linear_sweep` | `disassembler::linear::sweep` | no panic; terminates; every instruction within input range |

## Running

```
cargo install cargo-fuzz
cargo +nightly fuzz run decode        -- -max_total_time=300
cargo +nightly fuzz run linear_sweep  -- -max_total_time=300
```

## Crash-free run log

| Date | Target | Iterations | Result |
|------|--------|------------|--------|
| _fill me in_ | `decode` | — | — |
| _fill me in_ | `linear_sweep` | — | — |

In lieu of a recorded `cargo-fuzz` run, the in-tree test
`tests/instruction_tests.rs::differential::random_bytes_length_agreement`
exercises the decoder over 200,000 pseudo-random 16-byte windows on every
`cargo test`, and `never_panics_on_short_input` covers all 0–3 byte inputs.

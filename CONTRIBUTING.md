# Contributing

Thanks for helping make physics simulation accessible and correct.

## Principles

- Correctness first. A new solver lands with a test against a closed-form solution or a convergence study.
- Clear names over comments. Comments explain physics or a non-obvious numerical choice, nothing else.
- One concept per module. Keep the `SemiLinear` trait small; put equation-specific work in `equations/`.

## Workflow

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --release
```

Open a pull request with a short description of the physics, the method, and how you validated it.

## Adding an equation

1. Create `crates/soliton-core/src/equations/<name>.rs` implementing `SemiLinear`.
2. Add any exact solutions to `exact.rs`.
3. Add a test in `crates/soliton-core/tests/` that checks accuracy and invariant drift.
4. Expose the model string in `soliton-wasm` and `soliton-py`.

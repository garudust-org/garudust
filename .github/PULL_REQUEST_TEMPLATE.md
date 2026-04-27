## Summary

<!-- What does this PR do and why? 1-3 bullet points. -->

-

## Type of change

<!-- Check all that apply -->

- [ ] Bug fix (non-breaking)
- [ ] New feature (non-breaking)
- [ ] Breaking change (alters existing API or behaviour)
- [ ] Documentation / comments only
- [ ] CI / tooling / chore

## Checklist

- [ ] `cargo check --workspace --all-targets` passes
- [ ] `cargo clippy --workspace --all-targets -- -W clippy::all -W clippy::pedantic` passes (same flags as CI)
- [ ] `cargo fmt --all -- --check` passes
- [ ] No API keys, tokens, or secrets committed (`git diff HEAD | grep -iE '(api.?key|secret|token|bearer|sk-)'`)
- [ ] New tools registered in `bin/garudust/src/main.rs` and `bin/garudust-server/src/main.rs`
- [ ] New platform adapters added behind a feature flag in `garudust-platforms/Cargo.toml`
- [ ] README updated if user-facing behaviour changed

## Testing

<!-- How did you verify this works? Manual steps, test output, etc. -->

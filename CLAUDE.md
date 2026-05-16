# Garudust Agent — Claude Instructions

## Workflow Rules

- Before every `git commit` and `git push`, run `cargo test --workspace` and verify CI passes locally. Do not commit or push if tests fail.

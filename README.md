# decider-adr

Manage architecture decision records through their lifecycle. The binary is `decider`.

## Install

Not published yet. Install locally:

```bash
cargo install --path .
```

## Usage

```bash
decider --version
```

No other commands exist yet.

## Development

```bash
just install
just check
```

Run `just --list` for every task. Unit tests live inline (`#[cfg(test)]`),
integration tests in `tests/integration/`. Coverage must stay at or above 80%
lines.

## Documentation

Design decisions are recorded as ADRs in
[`docs/explanations/decisions/`](docs/explanations/decisions/README.md).

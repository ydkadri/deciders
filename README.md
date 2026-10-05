# decider-adr

Manage architecture decision records through their lifecycle. The binary is `decider`.

## Install

Not published yet. Install locally:

```bash
cargo install --path .
```

## Usage

```bash
decider init                 # once per clone, at the repository root
decider propose "Use Postgres"
decider accept 1
decider implement 1 --pr '#12'
decider propose "Use Redis"
decider reject 2 --reason "Too costly"
```

The lifecycle is `proposed`, then `accepted` and `implemented`, or `rejected`. See the [tutorial](docs/tutorials/getting-started.md) and the [command reference](docs/reference/commands.md).

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

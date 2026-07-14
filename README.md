# Rehearsal Planner

A Rust CLI that collects user availability and finds the best rehearsal time slots based on attendance and timing.

## Quick start

```bash
cargo run
```

```bash
cargo test
cargo clippy -- -D warnings
```

## Usage

```
<username> <YYYY-MM-DD HH:MM> <YYYY-MM-DD HH:MM>   # add availability
LIST                                                # list users
BEST <n>                                            # top N slots
BEST <n> AFTER|BEFORE|BETWEEN <date> ...            # filtered slots
DONE                                                # exit
```

Availability is saved to `availability.db` as you add slots.

## Documentation

- [Requirements](docs/requirements.md) — functional spec and testable criteria
- [Roadmap](docs/roadmap.md) — planned features

## Architecture

Layered Rust crate with zero external dependencies:

- `src/main.rs` — CLI interface
- `src/service.rs` — application service (coordinates domain + persistence)
- `src/domain.rs` — pure domain logic (overlap detection, ranking)
- `src/persistence.rs` — file-based repository (`AvailabilityRepository` trait)

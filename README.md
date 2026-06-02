# MIRA-RS - A Broadcast Box Discord Bot

Mira is a Discord bot for use with the fantastic [Broadcast Box](https://github.com/glimesh/broadcast-box). It allows you to specify which hosts to monitor and then subscribe to different stream keys.

This repo is for the rewrite of Mira in Rust. For the original project click [here](https://github.com/richardtatum/mira).

## Development Setup

### Prerequisites

- Rust (stable)
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli): `cargo install sqlx-cli --no-default-features --features sqlite`

### Environment Variables

Export these in your shell profile or set them for the session:

```bash
export DATABASE_URL="sqlite:/absolute/path/to/mira.db"
export DISCORD_TOKEN="your-token"
```

| Variable | Description |
|---|---|
| `DATABASE_URL` | SQLite connection string — must be an absolute path |
| `DISCORD_TOKEN` | Your Discord bot token |

### Database

Run migrations from the workspace root:

```bash
cargo sqlx migrate run --source crates/adapters/storage/migrations
```

### Running

```bash
cargo run -p mira-discord-bot
```

### After Changing Queries

If you modify any `sqlx::query!` macros in `crates/adapters/storage`, regenerate the query cache from within that crate:

```bash
pushd crates/adapters/storage && cargo sqlx prepare && popd
```

The `.sqlx` cache directory lives inside `crates/adapters/storage/` and should be checked into version control.

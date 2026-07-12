# MIRA-RS - A Broadcast Box Discord Bot

Mira is a Discord bot for use with the fantastic [Broadcast Box](https://github.com/glimesh/broadcast-box). It allows you to specify which hosts to monitor and then subscribe to different stream keys.

This repo is for the rewrite of Mira in Rust. For the original project click [here](https://github.com/richardtatum/mira).

## Development Setup — Discord Bot

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

## Development Setup — CLI

The CLI can be used to check stream status or watch a stream key directly, without running the bot.

### Environment Variables

Export these or pass them as flags:

```bash
export BROADCAST_BOX_URL="https://your-broadcast-box-host"
export BROADCAST_BOX_AUTH_TOKEN="your-token"  # optional
```

### Commands

**Check if a stream key is currently online:**

```bash
cargo run -p mira-cli -- status <key>
```

**Watch a stream key and print status changes to stdout:**

```bash
cargo run -p mira-cli -- watch <key>
```

Optionally override the polling interval (in seconds):

```bash
cargo run -p mira-cli -- watch <key> --polling-interval 10
```

Press `Ctrl+C` to stop watching.

## TODO

### Slash Commands

- [x] `/add_host` — register a Broadcast Box host with the guild
- [x] `/remove_host` — remove a registered host from the guild
- [x] `/subscribe` — subscribe to a stream key on a registered host
- [x] `/unsubscribe` — unsubscribe from a stream key
- [x] `/playing` — set what is currently playing on a host
- [x] `/list_subscriptions` — list all subscriptions across all registered guild hosts

### Functionality

- [ ] Full logging — replace `println!` calls with a structured logging framework (e.g. `tracing`)
- [ ] Tests — unit and integration test coverage across crates
- [ ] Stream screenshots — capture a thumbnail from the stream and display it in the online/offline embed
- [ ] IGDB integration — enrich "currently playing" metadata (cover art, genre, release date) via the [IGDB API](https://api-docs.igdb.com/)
- [x] Guild cleanup — remove all hosts, subscriptions, and related data when the bot is removed from a guild

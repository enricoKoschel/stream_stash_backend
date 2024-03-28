# Stream Stash backend

The backend for the Stream Stash website, the repository for the frontend can be
found [here](https://github.com/enricoKoschel/stream-stash).

## How to run locally

Run all the following commands in the root directory of the project

### Set environment variable

Set the `TIDE_SECRET` environment variable equal to a random string with a length of at least 32 bytes

### Build and run the app

```bash
cargo run # debug build
# or
cargo run --release # production build with optimizations
```

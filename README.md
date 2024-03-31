# Stream Stash backend

The backend for the Stream Stash website, the repository for the frontend can be
found [here](https://github.com/enricoKoschel/stream-stash).

## How to run locally

Run all the following commands in the root directory of the project

### Setup environment variables

Set the `ROCKET_SECRET_KEY` environment variable with a random 256-bit base64 string.\
This can be done with openssl like so:

```bash
openssl rand -base64 32
```

### Build and run the app

```bash
cargo run # debug build
# or
cargo run --release # production build with optimizations
```

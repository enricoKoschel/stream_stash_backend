# Stream Stash backend

The backend for the Stream Stash website, the repository for the frontend can be
found [here](https://github.com/enricoKoschel/stream-stash).

## How to run locally

Run all the following commands in the root directory of the project

### Create a Google Cloud application

TODO

### Setup environment variables

- Set the `ROCKET_SECRET_KEY` environment variable with a random 256-bit base64 string.\
  This can be done with openssl like so:

```bash
openssl rand -base64 32
```

- Set the `ROCKET_ADDRESS` environment variable to the address you want the backend to listen on\
  (recommended for use with frontend: 127.0.0.1)
- Set the `ROCKET_PORT` environment variable to the port you want the backend to listen on\
  (recommended for use with frontend: 8080)
- Set the `GOOGLE_CLIENT_ID` environment variable with your Google Cloud application's client id
- Set the `GOOGLE_CLIENT_SECRET` environment variable with your Google Cloud application's client secret

### Build and run the app

```bash
cargo run # debug build
# or
cargo run --release # production build with optimizations
```

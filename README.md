# Stream Stash backend

The backend for the Stream Stash website, the repository for the frontend can be
found [here](https://github.com/enricoKoschel/stream-stash).

## How to run locally

Run all the following commands in the root directory of the project

### Create a Google Cloud application

TODO

### Request API access to [The Movie Database](https://www.themoviedb.org)

Click [here](https://developer.themoviedb.org/docs/getting-started) for instructions on how to request access

### Setup environment variables

- Set the `ROCKET_SECRET_KEY` environment variable with a random 256-bit base64 string.\
  This can be done with openssl like so:

```bash
openssl rand -base64 32
```

- Set the `ROCKET_ADDRESS` environment variable to the address you want the backend to listen on\
  (recommended for use with frontend: `127.0.0.1`)
- Set the `ROCKET_PORT` environment variable to the port you want the backend to listen on\
  (recommended for use with frontend: `8080`)
- Set the `GOOGLE_CLIENT_ID` environment variable with your Google Cloud application's client id
- Set the `GOOGLE_CLIENT_SECRET` environment variable with your Google Cloud application's client secret
- Set the `TMDB_READ_ACCESS_TOKEN` environment variable with your TMDB API read access token
  - Make sure to use the API read access token, not the API key (both can be found [here](https://www.themoviedb.org/settings/api) after your request to access the TMDB API has been granted)

### Build and run the app

```bash
cargo run # debug build
# or
cargo run --release # production build with optimizations
```

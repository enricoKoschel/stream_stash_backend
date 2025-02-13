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

- Create a `.env` file in the same directory as the `Cargo.toml` file with the following contents:

```toml
ROCKET_SECRET_KEY="" # A random 256-bit base64 string
ROCKET_ADDRESS="" # The address you want the backend to listen on
ROCKET_PORT="" # The port you want the backend to listen on
GOOGLE_CLIENT_ID="" # Your Google Cloud application's Client ID
GOOGLE_CLIENT_SECRET="" # Your Google Cloud application's Client Secret
TMDB_READ_ACCESS_TOKEN="" # Your TMDB API Read Access Token
```

- You can create the `ROCKET_SECRET_KEY` with OpenSSL like so: `openssl rand -base64 32`
- The recommended address and port for use with the Stream Stash frontend are `127.0.0.1:8080`
- Make sure to use the TMDB API Read Access Token, not the API Key (both can be found [here](https://www.themoviedb.org/settings/api) after your request to access the TMDB API has been granted)

### Build and run the app

```bash
cargo run # debug build
# or
cargo run --release # production build with optimizations
```

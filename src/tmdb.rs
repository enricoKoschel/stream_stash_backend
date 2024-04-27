use crate::{
    macros::{get_json_body, internal_server_error, parse_url, serde_struct},
    ApiResult,
};

const POSTER_BASE_URL: &str = "https://image.tmdb.org/t/p/w600_and_h900_bestv2";
const BACKDROP_BASE_URL: &str = "https://image.tmdb.org/t/p/w1920_and_h1080_bestv2";

pub struct ReadAccessToken(pub String);

serde_struct!(
    pub Movie,
    backdrop_path: Option<String>,
    id: u32,
    original_title: String,
    overview: String,
    poster_path: Option<String>,
    title: String,
    release_date: String,
);
serde_struct!(
    pub Tv,
    backdrop_path: Option<String>,
    id: u32,
    original_name: String,
    overview: String,
    poster_path: Option<String>,
    name: String,
    first_air_date: String,
);

fn map_path(path: &mut Option<String>, base_url: &str) {
    *path = path.as_mut().map(|path| format!("{base_url}{path}"));
}

serde_struct!(pub MovieSearchResult, page: u32, results: Vec<Movie>, total_pages: u32, total_results: u32);

pub async fn movie_search(
    tmdb_read_access_token: &ReadAccessToken,
    http_client: &reqwest::Client,
    query: &str,
    page: u32,
) -> ApiResult<MovieSearchResult> {
    let mut api_url = parse_url!("https://api.themoviedb.org/3/search/movie")?;
    api_url
        .query_pairs_mut()
        .append_pair("include_adult", "false")
        .append_pair("language", "en-US")
        .append_pair("query", query)
        .append_pair("page", &page.to_string());
    let request = http_client
        .get(api_url)
        .bearer_auth(&tmdb_read_access_token.0);
    let response = get_json_body!(request, MovieSearchResult)?;

    match response {
        Ok(MovieSearchResult {
            page,
            mut results,
            total_pages,
            total_results,
        }) => {
            for movie in &mut results {
                map_path(&mut movie.poster_path, POSTER_BASE_URL);
                map_path(&mut movie.backdrop_path, BACKDROP_BASE_URL);
            }

            Ok(MovieSearchResult {
                page,
                results,
                total_pages,
                total_results,
            })
        }
        Err(err) => Err(internal_server_error!("Could not movie search TMDB: {err}")),
    }
}

serde_struct!(pub TvSearchResult, page: u32, results: Vec<Tv>, total_pages: u32, total_results: u32);

pub async fn tv_search(
    tmdb_read_access_token: &ReadAccessToken,
    http_client: &reqwest::Client,
    query: &str,
    page: u32,
) -> ApiResult<TvSearchResult> {
    let mut api_url = parse_url!("https://api.themoviedb.org/3/search/tv")?;
    api_url
        .query_pairs_mut()
        .append_pair("include_adult", "false")
        .append_pair("language", "en-US")
        .append_pair("query", query)
        .append_pair("page", &page.to_string());
    let request = http_client
        .get(api_url)
        .bearer_auth(&tmdb_read_access_token.0);
    let response = get_json_body!(request, TvSearchResult)?;

    match response {
        Ok(TvSearchResult {
            page,
            mut results,
            total_pages,
            total_results,
        }) => {
            for tv in &mut results {
                map_path(&mut tv.poster_path, POSTER_BASE_URL);
                map_path(&mut tv.backdrop_path, BACKDROP_BASE_URL);
            }

            Ok(TvSearchResult {
                page,
                results,
                total_pages,
                total_results,
            })
        }
        Err(err) => Err(internal_server_error!("Could not tv search TMDB: {err}")),
    }
}

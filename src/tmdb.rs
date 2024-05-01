use crate::{
    macros::{get_json_body, internal_server_error, parse_url, serde_struct},
    ApiResult,
};

const POSTER_BASE_URL: &str = "https://image.tmdb.org/t/p/w600_and_h900_bestv2";
const BACKDROP_BASE_URL: &str = "https://image.tmdb.org/t/p/w1920_and_h1080_bestv2";

pub struct ReadAccessToken(pub String);

serde_struct!(
    Media,
    backdrop_url: Option<String>,
    id: u32,
    media_type: String,
    key: String,
    original_title: String,
    overview: String,
    poster_url: Option<String>,
    title: String,
    date: String,
);

serde_struct!(pub SearchResult, page: u32, results: Vec<Media>, total_pages: u32, total_results: u32);

fn map_path(path: &Option<String>, base_url: &str) -> Option<String> {
    path.as_ref().map(|path| format!("{base_url}{path}"))
}

pub async fn movie_search(
    tmdb_read_access_token: &ReadAccessToken,
    http_client: &reqwest::Client,
    query: &str,
    page: u32,
) -> ApiResult<SearchResult> {
    serde_struct!(
        Movie,
        backdrop_path: Option<String>,
        id: u32,
        original_title: String,
        overview: String,
        poster_path: Option<String>,
        title: String,
        release_date: String,
    );
    serde_struct!(Response, page: u32, results: Vec<Movie>, total_pages: u32, total_results: u32);

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
    let response = get_json_body!(request, Response)?;

    match response {
        Ok(Response {
            page,
            results,
            total_pages,
            total_results,
        }) => {
            let media = results
                .into_iter()
                .map(|movie| Media {
                    backdrop_url: map_path(&movie.backdrop_path, BACKDROP_BASE_URL),
                    id: movie.id,
                    media_type: "movie".to_string(),
                    key: format!("movie:{}", movie.id),
                    original_title: movie.original_title,
                    overview: movie.overview,
                    poster_url: map_path(&movie.poster_path, POSTER_BASE_URL),
                    title: movie.title,
                    date: movie.release_date,
                })
                .collect::<Vec<_>>();

            Ok(SearchResult {
                page,
                results: media,
                total_pages,
                total_results,
            })
        }
        Err(err) => Err(internal_server_error!("Could not movie search TMDB: {err}")),
    }
}

pub async fn tv_search(
    tmdb_read_access_token: &ReadAccessToken,
    http_client: &reqwest::Client,
    query: &str,
    page: u32,
) -> ApiResult<SearchResult> {
    serde_struct!(
        Tv,
        backdrop_path: Option<String>,
        id: u32,
        original_name: String,
        overview: String,
        poster_path: Option<String>,
        name: String,
        first_air_date: String,
    );
    serde_struct!(Response, page: u32, results: Vec<Tv>, total_pages: u32, total_results: u32);

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
    let response = get_json_body!(request, Response)?;

    match response {
        Ok(Response {
            page,
            results,
            total_pages,
            total_results,
        }) => {
            let media = results
                .into_iter()
                .map(|tv| Media {
                    backdrop_url: map_path(&tv.backdrop_path, BACKDROP_BASE_URL),
                    id: tv.id,
                    media_type: "tv".to_string(),
                    key: format!("tv:{}", tv.id),
                    original_title: tv.original_name,
                    overview: tv.overview,
                    poster_url: map_path(&tv.poster_path, POSTER_BASE_URL),
                    title: tv.name,
                    date: tv.first_air_date,
                })
                .collect::<Vec<_>>();

            Ok(SearchResult {
                page,
                results: media,
                total_pages,
                total_results,
            })
        }
        Err(err) => Err(internal_server_error!("Could not tv search TMDB: {err}")),
    }
}

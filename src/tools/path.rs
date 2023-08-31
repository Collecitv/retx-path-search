use compact_str::CompactString;
use futures::future;

use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use serde_json;

use crate::build_fuzzy_regex_filter::build_fuzzy_regex_filter;
use crate::case_permutations::case_permutations;
use crate::trigrams::trigrams;

#[derive(Debug, Serialize, Deserialize)]
struct BodyRes {
    query: String,
    max_hits: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    num_hits: i32,            // Change the type to i32 or another appropriate numeric type
    elapsed_time_micros: i64, // Change the type to i32 or another appropriate numeric type
    hits: Vec<ResultItem>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResultItem {
    relative_path: String,
    repo_name: String,
    lang: String,
    content: String,
    symbols: String,
    avg_line_length: f64,
    is_directory: bool,
    last_commit: String,
    repo_ref: String,
    repo_disk_path: String,
    unique_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseObject {
    relative_path: String,
    repo_name: String,
    lang: String,
    repo_ref: String,
}

pub async fn path_search(
    index_name: &str,
    search_field: &str,
    search_query: &str,
) -> Result<String, Error> {
    let response_array = search_api(index_name, search_field, search_query).await?;

    let d = fuzzy_path_match(search_query, 50).await;

    let paths: Vec<_> = response_array
        .into_iter()
        .map(|c| c.relative_path)
        .collect::<HashSet<_>>() // Removes duplicates
        .into_iter()
        .collect::<Vec<_>>();

    println!("paths array length: {:?}", paths.len());

    // let is_semantic = paths.is_empty();

    // If there are no lexical results, perform a semantic search.

    // if path is empty do the part
    if paths.is_empty() {}

    let response = paths
        .iter()
        .map(|path| path.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(response)
}

async fn search_api(
    index_name: &str,
    search_field: &str,
    search_query: &str,
) -> Result<Vec<ResponseObject>, Error> {
    let client = Client::new();
    let base_url = "http://13.234.204.108:7280";

    println!("search_query {}", search_query);

    let query = if !search_field.is_empty() {
        format!("{}:{}", search_field, search_query)
    } else {
        search_query.to_owned()
    };

    let json_data = BodyRes {
        query,
        max_hits: 10,
    };

    let json_string = serde_json::to_string(&json_data).expect("Failed to serialize object");

    let url = format!("{}/api/v1/{}/search", base_url, index_name);

    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(json_string)
        .send()
        .await?;

    let mut response_array: Vec<ResponseObject> = Vec::new();

    if response.status().is_success() {
        let response_text = response.text().await?;

        let parsed_response: Result<ApiResponse, serde_json::Error> =
            serde_json::from_str(&response_text);

        match parsed_response {
            Ok(api_response) => {
                for result_item in api_response.hits {
                    response_array.push(ResponseObject {
                        relative_path: result_item.relative_path,
                        repo_name: result_item.repo_name,
                        lang: result_item.lang,
                        repo_ref: result_item.repo_ref,
                    });
                }
            }
            Err(err) => {
                println!("Failed to parse JSON response: {}", err);
            }
        }
    } else {
        println!("Request was not successful: {}", response.status());
    }

    Ok(response_array)
}

async fn fuzzy_path_match(query_str: &str, limit: usize) {
    let hits = future::try_join_all(
        trigrams(query_str)
            .flat_map(|s| case_permutations(s.as_str()))
            .map(|token| search_with_async(token.clone())),
    )
    .await; // Pass token as a reference

    let regex_filter = build_fuzzy_regex_filter(query_str);

    println!("{:?}", hits);
    println!("regex_filter {:?}", regex_filter);

    // if the regex filter fails to build for some reason, the filter defaults to returning
    // false and zero results are produced
    // hits.into_iter()
    //     .map(|(doc, _)| doc)
    //     .filter(move |doc| {
    //         regex_filter
    //             .as_ref()
    //             .map(|f| f.is_match(&doc.relative_path))
    //             .unwrap_or_default()
    //     })
    //     .filter(|doc| !doc.relative_path.ends_with('/')) // omit directories
    //     .take(limit);
}

async fn search_with_async(token: CompactString) -> Result<Vec<ResponseObject>, Error> {
    search_api("bloop", "relative_path", token.as_str()).await
}

// declare a variable

use compact_str::CompactString;
use futures::future;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};

use serde_json;

use crate::build_fuzzy_regex_filter::build_fuzzy_regex_filter;
use crate::case_permutations::case_permutations;
use crate::trigrams::trigrams;

#[derive(Debug, Serialize, Deserialize)]
struct BodyRes {
    query: String,
    max_hits: i32,
}

struct FileResDoc {
    res_obj: ResponseObject,
    val: usize,
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

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Clone)]
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

    println!("search_query_ls {}", search_query);

    // Trim leading and trailing whitespace from search_query
    let search_query = search_query.trim();

    // Perform a case-insensitive comparison
    // if search_query.trim() == ".rs" {
    //     return Ok(Vec::new());
    // }

    let query = if !search_field.is_empty() {
        format!("{}:{}", search_field, search_query)
    } else {
        search_query.to_owned()
    };

    let json_data = BodyRes {
        query,
        max_hits: 100,
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
    let mut counts: HashMap<ResponseObject, usize> = HashMap::new();

    let hits = trigrams(query_str)
        .flat_map(|s| case_permutations(s.as_str()))
        .chain(std::iter::once(query_str.to_owned().into())); // Pass token as a reference

    // Iterate over counts and populate file_documents
    for hit in hits {
        println!("hit: {:?}\n", hit.clone());
        let result = search_with_async(hit.clone().into()).await;
        println!("res: {:?}\n", result);
        for res in result.unwrap() {
            // Check if the key exists in the HashMap
            if let Some(entry) = counts.get_mut(&res.clone()) {
                // The key exists, increment its value
                *entry += 1;
            } else {
                // The key doesn't exist, insert it with an initial value of 0
                counts.insert(res.clone(), 0);
            }
        }
    }

    // Convert the HashMap into a Vec<(ResponseObject, usize)>
    let mut new_hit: Vec<(ResponseObject, usize)> = counts.into_iter().collect();

    new_hit.sort_by(|(this_doc, this_count), (other_doc, other_count)| {
        let order_count_desc = other_count.cmp(this_count);
        let order_path_asc = this_doc
            .relative_path
            .as_str()
            .cmp(other_doc.relative_path.as_str());

        order_count_desc.then(order_path_asc)
    });

    let regex_filter = build_fuzzy_regex_filter(query_str);

    // if the regex filter fails to build for some reason, the filter defaults to returning
    // false and zero results are produced
    // let result = new_hit
    //     .into_iter()
    //     .map(|(doc, _)| doc)
    //     .filter(move |doc| {
    //         regex_filter
    //             .as_ref()
    //             .map(|f| f.is_match(&doc.relative_path))
    //             .unwrap_or_default()
    //     })
    //     .filter(|doc| !doc.relative_path.ends_with('/')) // omit directories
    //     .take(limit);

    let mut filterd_hits = Vec::new();

    match regex_filter {
        Some(f) => {
            for res in new_hit {
                if f.is_match(&res.0.relative_path) {
                    filterd_hits.push(res.0.relative_path);
                }
            }
        }
        None => {}
    }

    let result = filterd_hits;

    // let filterd_hits_by_slash = filterd_hits
    //     .into_iter()
    //     .filter(|doc| !doc.ends_with('/'))
    //     .take(limit);

    println!("result: {:?}", result);
}

async fn search_with_async(token: CompactString) -> Result<Vec<ResponseObject>, Error> {
    search_api("bloop", "relative_path", token.as_str()).await
}

// declare a variable

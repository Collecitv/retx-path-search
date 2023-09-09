mod helpers;
mod tools;

use helpers::build_fuzzy_regex_filter;
use helpers::case_permutations;
use helpers::trigrams;
use tools::path;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    let result = path::path_search("bloop", "relative_path", "webserver.rs").await;
    println!("{:?}", result);

    Ok(())
}

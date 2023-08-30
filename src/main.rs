mod tools;

use tools::path;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");


    let result = path::path_search("bloop", "", "path").await;
    println!("{:?}", result);

    Ok(())
}

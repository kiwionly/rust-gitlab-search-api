use std::error::Error;
use std::sync::Arc;
use std::time::SystemTime;
use clap::{Parser};
use colored::Colorize;

mod gitlab_client;

#[derive(Parser, Debug)]
#[command(name = "rust_gitlab_search")]
#[command(author = "kiwionly <kiwionly@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "searching gitlab repositories", long_about = None)]
struct Cli {

    #[arg(short, long)]
    url: String,

    #[arg(short, long)]
    token: String,

    #[arg(short= 'o', long, default_value_t = 30)]
    time_out: u64,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    group_ids: Vec<u64>,

    #[arg(short, long)]
    project_ids: Vec<u64>,

    #[arg(short, long)]
    query: String,

}
//run -- -u <url> -t <token> -v -g <group_id_1> -g <group_id_2> -q <search_term>
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let cli = Cli::parse();

    let url = cli.url;
    let token = cli.token;
    let group_ids = cli.group_ids;
    let query = cli.query;
    let time_out = cli.time_out;
    let verbose = cli.verbose;

    let mut client = gitlab_client::GitlabClient::new_unsecure(&url, &token, time_out);
    client.verbose = verbose;

    let instance = Arc::new(client);

    let start = SystemTime::now();

    let search_result = instance.search_by_group_ids(&instance, group_ids, &query).await?;
    // let projects = instance.search_by_ids(&instance, vec![123, 456], "search_term").await?;
    // let projects = instance.search_by_name(&instance, "project_name", "search_term").await?;

    let mut count = 0;

    for sr in &search_result {

        count += sr.count;

        for r in &sr.result_list {

            println!("Project: {}", r.name.magenta());
            println!("URL: {}", r.url.bright_blue());
            println!("Data: {}", r.data.bright_green()); // need difference library for highlighted text within text
            println!("-------");
        }
    }

    println!("search result(s) = {}", count);
    println!("total time used = {:?}", SystemTime::now().duration_since(start)?);

    Ok(())
}


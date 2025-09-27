use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use url::{Url, ParseError};
use clap::Parser;
use futures::stream::{self, StreamExt};

/// A concurrent web crawler
#[derive(Parser, Debug)]
struct Cli {
    /// The starting URL to crawl
    #[arg(short, long)]
    url: String,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let start_url = args.url;

    // A queue of URLs to visit. Arc<Mutex<...>> allows safe concurrent access.
    let to_visit = Arc::new(Mutex::new(VecDeque::from([start_url.clone()])));
    // A set of URLs that have already been visited.
    let visited = Arc::new(Mutex::new(HashSet::new()));
    
    // Create a single reqwest client to be reused, which is more efficient.
    let client = Arc::new(Client::new());

    println!("Starting crawl from: {}", start_url);

    // Create a stream that never ends, allowing our loop to control its own pace.
    stream::iter(0..200) // The number here defines max concurrency
        .for_each_concurrent(None, |_| {
            let to_visit = Arc::clone(&to_visit);
            let visited = Arc::clone(&visited);
            let client = Arc::clone(&client);
            
            async move {
                let mut locked_to_visit = to_visit.lock().await;
                if let Some(url) = locked_to_visit.pop_front() {
                    drop(locked_to_visit); // Release lock before long-running task

                    let mut locked_visited = visited.lock().await;
                    if !locked_visited.contains(&url) {
                        locked_visited.insert(url.clone());
                        drop(locked_visited); // Release lock

                        if let Ok(new_links) = crawl_url(&client, &url).await {
                            let mut locked_to_visit_again = to_visit.lock().await;
                            for link in new_links {
                                locked_to_visit_again.push_back(link);
                            }
                        }
                    }
                }
            }
        })
        .await;
}

async fn crawl_url(client: &Client, url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    println!("Crawling: {}", url);
    
    let base_url = Url::parse(url)?;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("a[href]").unwrap();

    let mut links = Vec::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            match base_url.join(href) {
                Ok(mut new_url) => {
                    new_url.set_fragment(None); // Remove fragments like #section
                    if new_url.domain() == base_url.domain() {
                        links.push(new_url.to_string());
                    }
                },
                Err(_) => continue, // Ignore malformed links
            }
        }
    }
    Ok(links)
}
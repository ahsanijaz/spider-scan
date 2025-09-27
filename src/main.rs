use reqwest::header::HeaderMap;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;
use clap::Parser;
use futures::stream::{self, StreamExt};

/// A concurrent web crawler and vulnerability scanner
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

    let to_visit = Arc::new(Mutex::new(VecDeque::from([start_url.clone()])));
    let visited = Arc::new(Mutex::new(HashSet::new()));
    let client = Arc::new(Client::new());

    println!("Starting crawl from: {}", start_url);

    stream::iter(0..200) // Max concurrency
        .for_each_concurrent(None, |_| {
            let to_visit = Arc::clone(&to_visit);
            let visited = Arc::clone(&visited);
            let client = Arc::clone(&client);
            
            async move {
                let mut locked_to_visit = to_visit.lock().await;
                if let Some(url) = locked_to_visit.pop_front() {
                    drop(locked_to_visit);

                    let mut locked_visited = visited.lock().await;
                    if !locked_visited.contains(&url) {
                        locked_visited.insert(url.clone());
                        drop(locked_visited);

                        if let Ok((new_links, headers, body)) = crawl_url(&client, &url).await {
                            // Run our scanner functions on the response
                            check_security_headers(&headers, &url);
                            check_server_banner(&headers, &url);
                            check_sensitive_content(&body, &url);

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

// This function now returns the links, headers, and body
async fn crawl_url(client: &Client, url: &str) -> Result<(Vec<String>, HeaderMap, String), Box<dyn std::error::Error>> {
    println!("Crawling: {}", url);
    
    let base_url = Url::parse(url)?;
    let response = client.get(url).send().await?;
    
    // Clone headers before consuming the body
    let headers = response.headers().clone();
    let body = response.text().await?;
    
    let document = Html::parse_document(&body);
    let selector = Selector::parse("a[href]").unwrap();

    let mut links = Vec::new();
    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if let Ok(mut new_url) = base_url.join(href) {
                new_url.set_fragment(None);
                if new_url.domain() == base_url.domain() {
                    links.push(new_url.to_string());
                }
            }
        }
    }
    Ok((links, headers, body))
}

// --- SCANNER FUNCTIONS ---

fn check_security_headers(headers: &HeaderMap, url: &str) {
    if !headers.contains_key("Content-Security-Policy") {
        println!("[VULN] Missing Content-Security-Policy header on: {}", url);
    }
    if !headers.contains_key("X-Frame-Options") {
         println!("[VULN] Missing X-Frame-Options header on: {}", url);
    }
}

fn check_server_banner(headers: &HeaderMap, url: &str) {
    if let Some(server_header) = headers.get("Server") {
        let server_str = server_header.to_str().unwrap_or("");
        // A simple check for any version number is a good start
        if server_str.chars().any(|c| c.is_digit(10)) {
            println!("[INFO] Verbose Server Banner: '{}' on: {}", server_str, url);
        }
    }
}

fn check_sensitive_content(body: &str, url: &str) {
    // Check for exposed directory listings
    if body.contains("<title>Index of /") {
        println!("[VULN] Directory listing enabled on: {}", url);
    }
    // Check for common error message patterns that might leak info
    if body.contains("SQL syntax error") || body.contains("Fatal error:") {
        println!("[VULN] Possible error message exposure on: {}", url);
    }
}
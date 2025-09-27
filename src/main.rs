use reqwest;
// Import the Html and Selector structs from the scraper crate
use scraper::{Html, Selector};

#[tokio::main]
async fn main() {
    let url = "http://info.cern.ch"; // The world's first website, great for testing
    println!("Fetching URL: {}", url);

    match reqwest::get(url).await {
        Ok(response) => {
            println!("Status: {}", response.status());
            
            if response.status().is_success() {
                match response.text().await {
                    Ok(body) => {
                        // --- NEW CODE: PARSING AND EXTRACTING LINKS ---
                        
                        // Parse the HTML body into a traversable document
                        let document = Html::parse_document(&body);
                        
                        // Create a CSS selector to find all anchor tags with an `href` attribute.
                        // "a[href]" means "find all <a> elements that have an `href` attribute".
                        let selector = Selector::parse("a[href]").unwrap();

                        println!("\n--- Found Links ---");
                        // Iterate over all elements in the document that match our selector
                        for element in document.select(&selector) {
                            // For each element, get the value of its `href` attribute.
                            if let Some(href) = element.value().attr("href") {
                                println!("{}", href);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading response body: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error making request: {}", e);
        }
    }
}
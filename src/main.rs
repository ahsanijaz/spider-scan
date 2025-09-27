use reqwest;

#[tokio::main]
async fn main() {
    let url = "http://info.cern.ch"; // The world's first website, great for testing
    println!("Fetching URL: {}", url);

    match reqwest::get(url).await {
        Ok(response) => {
            println!("Status: {}", response.status());

            // Check if the request was successful
            if response.status().is_success() {
                match response.text().await {
                    Ok(body) => {
                        println!("\n--- Response Body ---");
                        println!("{}", body);
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
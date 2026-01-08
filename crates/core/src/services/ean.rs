use crate::error::{Error, Result};
use serde::Deserialize;
use tracing::{debug, warn};

/// Service for looking up product information from EAN/barcode
pub struct EanService {
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct EanLookupResult {
    pub title: String,
    pub original_barcode: String,
    pub vendor: Option<String>,
    pub category: Option<String>,
}

// UPCitemdb API response structures
#[derive(Debug, Deserialize)]
struct UpcItemDbResponse {
    code: String,
    items: Option<Vec<UpcItem>>,
}

#[derive(Debug, Deserialize)]
struct UpcItem {
    title: Option<String>,
    brand: Option<String>,
    category: Option<String>,
}

impl EanService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Look up a product by EAN/barcode
    pub async fn lookup(&self, barcode: &str) -> Result<Option<EanLookupResult>> {
        // Clean the barcode (remove spaces, dashes)
        let clean_barcode: String = barcode.chars().filter(|c| c.is_ascii_digit()).collect();

        if clean_barcode.is_empty() {
            return Err(Error::Validation("Invalid barcode format".into()));
        }

        debug!("Looking up barcode: {}", clean_barcode);

        // Try UPCitemdb first (better for movies/media)
        if let Some(result) = self.lookup_upcitemdb(&clean_barcode).await? {
            debug!("Found in UPCitemdb: {:?}", result.title);
            return Ok(Some(result));
        }

        // Fallback to OpenGTINDB
        if let Some(result) = self.lookup_opengtindb(&clean_barcode).await? {
            debug!("Found in OpenGTINDB: {:?}", result.title);
            return Ok(Some(result));
        }

        debug!("Barcode not found in any database: {}", clean_barcode);
        Ok(None)
    }

    /// Look up using UPCitemdb API (good for movies/media)
    /// Free tier: 100 requests/day
    async fn lookup_upcitemdb(&self, barcode: &str) -> Result<Option<EanLookupResult>> {
        let url = format!(
            "https://api.upcitemdb.com/prod/trial/lookup?upc={}",
            barcode
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "MyMovies/1.0")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                warn!("UPCitemdb request failed: {}", e);
                Error::ExternalApi(format!("UPCitemdb request failed: {}", e))
            })?;

        if !response.status().is_success() {
            debug!("UPCitemdb returned status: {}", response.status());
            return Ok(None);
        }

        let data: UpcItemDbResponse = response.json().await.map_err(|e| {
            warn!("UPCitemdb parse error: {}", e);
            Error::ExternalApi(e.to_string())
        })?;

        if data.code != "OK" {
            return Ok(None);
        }

        let item = match data.items.and_then(|items| items.into_iter().next()) {
            Some(item) => item,
            None => return Ok(None),
        };

        let title = match item.title {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(None),
        };

        Ok(Some(EanLookupResult {
            title: Self::clean_title(&title),
            original_barcode: barcode.to_string(),
            vendor: item.brand,
            category: item.category,
        }))
    }

    /// Fallback to OpenGTINDB
    async fn lookup_opengtindb(&self, barcode: &str) -> Result<Option<EanLookupResult>> {
        let url = format!(
            "https://opengtindb.org/api/v1/?ean={}&cmd=query&queryid=400000000",
            barcode
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "MyMovies/1.0")
            .send()
            .await
            .map_err(|e| Error::ExternalApi(format!("OpenGTINDB request failed: {}", e)))?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let text = response
            .text()
            .await
            .map_err(|e| Error::ExternalApi(e.to_string()))?;

        // Parse the response (OpenGTINDB returns a custom format)
        let mut title: Option<String> = None;
        let mut vendor: Option<String> = None;
        let mut category: Option<String> = None;

        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "error" => {
                        if value != "0" {
                            return Ok(None);
                        }
                    }
                    "detailname" => title = Some(value.trim().to_string()),
                    "mainname" if title.is_none() => title = Some(value.trim().to_string()),
                    "vendor" => vendor = Some(value.trim().to_string()),
                    "subcat" => category = Some(value.trim().to_string()),
                    _ => {}
                }
            }
        }

        match title {
            Some(t) if !t.is_empty() => Ok(Some(EanLookupResult {
                title: Self::clean_title(&t),
                original_barcode: barcode.to_string(),
                vendor,
                category,
            })),
            _ => Ok(None),
        }
    }

    /// Clean up the title for TMDB search
    /// Removes common suffixes like [Blu-ray], (DVD), import info, actor names, etc.
    fn clean_title(title: &str) -> String {
        let mut result = title.to_string();

        // First: Remove everything after " - " that looks like actor names or extra info
        // This catches patterns like "Movie Title - Actor1, Actor2, Actor3"
        if let Some(idx) = result.find(" - ") {
            let after_dash = &result[idx + 3..];
            // If what's after the dash contains commas (likely actors) or is longer text, remove it
            if after_dash.contains(',') || after_dash.len() > 30 {
                result = result[..idx].to_string();
            }
        }

        // Remove content in square brackets (region info, format, etc.)
        // e.g., "[Blu-ray]", "[regio Free (0)]", "[Region 2]"
        if let Ok(re) = regex::Regex::new(r"\[[^\]]*\]") {
            result = re.replace_all(&result, "").to_string();
        }

        // Remove content in parentheses that looks like format/region/import info
        if let Ok(re) = regex::Regex::new(
            r"(?i)\([^)]*(?:import|region|pal|ntsc|blu-ray|dvd|uhd|4k|regio)[^)]*\)",
        ) {
            result = re.replace_all(&result, "").to_string();
        }

        let patterns = [
            // Format indicators
            "Blu-ray",
            "Blu-Ray",
            "Dvd",
            "DVD",
            "4K UHD",
            "4k Uhd",
            "UHD",
            "Bd",
            "BD",
            "HD DVD",
            // Edition info
            "Steelbook",
            "Limited Edition",
            "Special Edition",
            "Collector's Edition",
            "Director's Cut",
            "Extended Edition",
            "Ultimate Edition",
            "Anniversary Edition",
            "Remastered",
            // Import/region info
            "Import",
            "Region Free",
            "Region 2",
            "Region 1",
            "Region B",
            "Region A",
            "regio Free",
        ];

        // Remove patterns
        for pattern in patterns {
            result = result.replace(pattern, "");
        }

        // Remove "Dc:" or "DC:" prefix (common for DC Comics movies)
        let lower = result.to_lowercase();
        if lower.starts_with("dc: ") {
            result = result[4..].to_string();
        }

        // Remove extra whitespace
        result = result.split_whitespace().collect::<Vec<_>>().join(" ");

        // Remove trailing/leading dashes and artifacts
        result = result
            .trim_end_matches(" -")
            .trim_end_matches("-")
            .to_string();
        result = result
            .trim_start_matches("- ")
            .trim_start_matches("-")
            .to_string();

        // Final trim
        result.trim().to_string()
    }

    /// Validate EAN-13 checksum
    pub fn validate_ean13(barcode: &str) -> bool {
        let digits: Vec<u32> = barcode.chars().filter_map(|c| c.to_digit(10)).collect();

        if digits.len() != 13 {
            return false;
        }

        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
            .sum();

        // is_multiple_of is still unstable, use modulo instead
        #[allow(clippy::manual_is_multiple_of)]
        let valid = sum % 10 == 0;
        valid
    }
}

impl Default for EanService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ean13() {
        assert!(EanService::validate_ean13("5050582721478")); // Valid EAN
        assert!(!EanService::validate_ean13("5050582721479")); // Invalid checksum
        assert!(!EanService::validate_ean13("123")); // Too short
    }

    #[test]
    fn test_clean_title() {
        assert_eq!(
            EanService::clean_title("The Matrix [Blu-ray]"),
            "The Matrix"
        );
        assert_eq!(EanService::clean_title("Inception (4K UHD)"), "Inception");
        assert_eq!(
            EanService::clean_title("Star Wars Steelbook Limited Edition"),
            "Star Wars"
        );
        // UPCitemdb format
        assert_eq!(
            EanService::clean_title(
                "Dc: Constantine: City Of Demons - (german Import) (us Import) Dvd"
            ),
            "Constantine: City Of Demons"
        );
        assert_eq!(
            EanService::clean_title("The Dark Knight (Blu-ray) (UK Import)"),
            "The Dark Knight"
        );
        // With actor names after dash
        assert_eq!(
            EanService::clean_title(
                "Fast And Furious 2 [regio Free (0)] - Paul Walker, Tyrese Gibson, Eva Mende"
            ),
            "Fast And Furious 2"
        );
        // Region info in brackets
        assert_eq!(
            EanService::clean_title("Gladiator [Region 2] [UK Import]"),
            "Gladiator"
        );
    }
}

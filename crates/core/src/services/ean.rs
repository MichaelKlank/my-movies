use serde::Deserialize;
use crate::error::{Error, Result};

/// Service for looking up product information from EAN/barcode
pub struct EanService {
    client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct OpenGtinResponse {
    error: Option<String>,
    #[serde(default)]
    products: Vec<OpenGtinProduct>,
}

#[derive(Debug, Deserialize)]
struct OpenGtinProduct {
    #[serde(rename = "detailname")]
    detail_name: Option<String>,
    #[serde(rename = "mainname")]
    main_name: Option<String>,
    #[serde(rename = "vendor")]
    vendor: Option<String>,
    #[serde(rename = "subcat")]
    category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EanLookupResult {
    pub title: String,
    pub original_barcode: String,
    pub vendor: Option<String>,
    pub category: Option<String>,
}

impl EanService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Look up a product by EAN/barcode using Open EAN Database
    pub async fn lookup(&self, barcode: &str) -> Result<Option<EanLookupResult>> {
        // Clean the barcode (remove spaces, dashes)
        let clean_barcode: String = barcode.chars().filter(|c| c.is_ascii_digit()).collect();

        if clean_barcode.is_empty() {
            return Err(Error::Validation("Invalid barcode format".into()));
        }

        // Try OpenGTINDB first
        if let Some(result) = self.lookup_opengtindb(&clean_barcode).await? {
            return Ok(Some(result));
        }

        // Could add fallback to other services here (UPCitemdb, etc.)

        Ok(None)
    }

    async fn lookup_opengtindb(&self, barcode: &str) -> Result<Option<EanLookupResult>> {
        // OpenGTINDB API
        // Note: This requires registration for heavy usage
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
        // Format: error=0\n---\nname=value\n...
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
    /// Removes common suffixes like [Blu-ray], (DVD), etc.
    fn clean_title(title: &str) -> String {
        let patterns = [
            "[Blu-ray]",
            "[DVD]",
            "[4K UHD]",
            "[4K Ultra HD]",
            "[Blu-ray + DVD]",
            "(Blu-ray)",
            "(DVD)",
            "(4K UHD)",
            "Blu-ray",
            "DVD",
            "4K UHD",
            "Steelbook",
            "Limited Edition",
            "Special Edition",
            "Collector's Edition",
            "Director's Cut",
        ];

        let mut result = title.to_string();
        for pattern in patterns {
            result = result.replace(pattern, "");
        }

        // Remove extra whitespace
        result.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Validate EAN-13 checksum
    pub fn validate_ean13(barcode: &str) -> bool {
        let digits: Vec<u32> = barcode
            .chars()
            .filter_map(|c| c.to_digit(10))
            .collect();

        if digits.len() != 13 {
            return false;
        }

        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
            .sum();

        sum % 10 == 0
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
        assert_eq!(
            EanService::clean_title("Inception (4K UHD)"),
            "Inception"
        );
        assert_eq!(
            EanService::clean_title("Star Wars Steelbook Limited Edition"),
            "Star Wars"
        );
    }
}

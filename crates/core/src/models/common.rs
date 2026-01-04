use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[derive(Default)]
pub enum DiscType {
    Dvd,
    #[default]
    BluRay,
    UhdBluRay,
    Hddvd,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Series,
    Collection,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[derive(Default)]
pub enum Condition {
    Mint,
    Excellent,
    #[default]
    Good,
    Fair,
    Poor,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum VideoStandard {
    Ntsc,
    Pal,
    Secam,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LentInfo {
    pub lent_to: Option<String>,
    pub lent_due: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseInfo {
    pub purchase_date: Option<chrono::NaiveDate>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub purchase_place: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueInfo {
    pub value_date: Option<chrono::NaiveDate>,
    pub value_price: Option<f64>,
    pub value_currency: Option<String>,
}

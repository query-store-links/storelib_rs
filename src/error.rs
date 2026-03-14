/// All errors that can be produced by storelib_rs.
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XML parse error: {0}")]
    Xml(String),

    #[error("Product not found")]
    NotFound,

    #[error("Request timed out")]
    TimedOut,

    #[error("{0}")]
    Other(String),
}

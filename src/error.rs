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

    #[error("Operation cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl StoreError {
    /// Walk `self` and every `source()` underneath it, returning each layer's
    /// `Display` text. Index 0 is `self`. Useful when you want to show the
    /// full chain (e.g. `reqwest::Error → hyper::Error → io::Error`) instead
    /// of just the top-level wrapper.
    pub fn causes(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(self);
        while let Some(e) = cur {
            out.push(e.to_string());
            cur = e.source();
        }
        out
    }

    /// Render `self` plus its full source chain as one multi-line string:
    /// `"top message\ncaused by: <layer1>\ncaused by: <layer2>"`.
    pub fn full_chain(&self) -> String {
        self.causes().join("\ncaused by: ")
    }
}

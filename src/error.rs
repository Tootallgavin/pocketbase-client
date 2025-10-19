use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecordViewError {
    /// The record (by collection + identifier) was not found (HTTP 404).
    #[error("record not found: collection='{collection}', id='{identifier}'")]
    NotFound {
        collection: String,
        identifier: String,
        /// Optional truncated body to aid debugging.
        body_snippet: String,
    },

    /// Other non-2xx HTTP error.
    #[error("http error {status} for {url}: {body_snippet}")]
    Http {
        status: u16,
        url: String,
        body_snippet: String,
    },

    /// JSON decode error with precise path from serde_path_to_error.
    #[error("json decode error at `{path}`: {source}")]
    Decode {
        path: String,
        #[source]
        source: serde_path_to_error::Error<serde_json::Error>,
        body_snippet: String,
    },

    /// Transport or unexpected lower-level error.
    #[error("transport error: {0}")]
    Transport(#[from] anyhow::Error),
}

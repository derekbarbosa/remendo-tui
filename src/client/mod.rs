//! Sashiko API client: trait, HTTP implementation, error types.
//!
//! Provides a type-safe, async interface for accessing Sashiko
//! instances. The [`SashikoApi`] trait abstracts the API surface;
//! [`HttpClient`] is the production implementation using `reqwest`.

pub mod api;
pub mod error;
pub mod http;
pub mod mock;
pub mod types;

pub use api::SashikoApi;
pub use error::ApiError;
pub use http::HttpClient;
pub use mock::MockClient;
pub use types::{ListParams, RetryConfig, ReviewQuery};

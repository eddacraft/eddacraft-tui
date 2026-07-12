use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

use crate::{PlanReadError, WorkspaceReadError};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("dashboard listener must bind to a loopback address")]
    NonLoopback,
    #[error("workspace boundary could not be initialised: {0}")]
    Workspace(#[from] WorkspaceReadError),
    #[error("dashboard server failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("requested plan was not found")]
    PlanNotFound,
    #[error("{0}")]
    Plan(#[from] PlanReadError),
    #[error("dashboard worker failed")]
    Worker,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::PlanNotFound => (StatusCode::NOT_FOUND, "plan-not-found"),
            Self::Plan(PlanReadError::InvalidId) => (StatusCode::BAD_REQUEST, "invalid-plan-id"),
            Self::Plan(_) => (StatusCode::SERVICE_UNAVAILABLE, "plan-data-unavailable"),
            Self::Worker => (StatusCode::INTERNAL_SERVER_ERROR, "dashboard-worker-failed"),
        };
        let body = ErrorBody {
            code,
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

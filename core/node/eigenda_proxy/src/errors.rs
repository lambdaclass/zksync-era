use axum::response::{IntoResponse, Response};

pub(crate) enum RequestProcessorError {}

impl IntoResponse for RequestProcessorError {
    fn into_response(self) -> Response {
        unimplemented!("EigenDA request error into response")
    }
}

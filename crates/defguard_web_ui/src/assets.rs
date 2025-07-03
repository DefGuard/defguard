use axum::{
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

pub async fn web_asset(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    // Rewrite the path to match the structure of the embedded files
    path.insert_str(0, "dist/");
    StaticFile(path)
}

pub async fn index() -> impl IntoResponse {
    web_asset(Uri::from_static("/index.html")).await
}

pub async fn svg(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    // Rewrite the path to match the structure of the embedded files
    path.insert_str(0, "src/shared/images/");
    StaticFile(path)
}

#[derive(Embed)]
#[folder = "../../web/"]
#[include = "dist/*"]
#[include = "src/shared/images/*"]
struct WebAsset;

pub(crate) struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match WebAsset::get(path.as_str()) {
            Some(content) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
            }
            None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
        }
    }
}

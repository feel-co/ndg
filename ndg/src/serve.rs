use std::path::Path;

use axum::Router;
use misstep::{Result, ResultExt};
use tower_http::services::ServeDir;

/// Start a local web server to serve the generated documentation.
///
/// # Arguments
///
/// * `output_dir` - The directory containing the generated HTML files
/// * `port` - The port to serve on
pub async fn serve_docs(output_dir: &Path, port: u16) -> Result<()> {
  let output_dir = output_dir.to_path_buf();

  let serve_dir =
    ServeDir::new(&output_dir).append_index_html_on_directories(true);

  let app = Router::new().fallback_service(serve_dir);

  let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
    .await
    .with_context(|| format!("Failed to bind to port {}", port))?;

  println!("Starting web server on http://127.0.0.1:{}", port);
  println!("Serving files from: {}", output_dir.display());
  println!("Press Ctrl+C to stop");

  axum::serve(listener, app).await.context("Server error")?;

  Ok(())
}

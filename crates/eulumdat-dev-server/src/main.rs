//! Development server with Brotli/Gzip pre-compressed file support.
//!
//! Serves static files from a directory, automatically serving `.br` or `.gz`
//! versions when available and the client supports them.
//!
//! Usage:
//!   dev-server \[OPTIONS\] \[DIR\]
//!
//! Example:
//!   dev-server -p 8042 ../eulumdat-wasm/dist

use actix_files::NamedFile;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "dev-server")]
#[command(about = "Development server with Brotli/Gzip support for WASM files")]
struct Args {
    /// Directory to serve
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8042")]
    port: u16,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,
}

/// Serve static files with pre-compressed variant support.
///
/// If client accepts `br` encoding and `file.br` exists, serve it with Content-Encoding: br.
/// If client accepts `gzip` encoding and `file.gz` exists, serve it with Content-Encoding: gzip.
/// Otherwise serve the original file.
async fn serve_file(
    req: HttpRequest,
    path: web::Path<String>,
    root: web::Data<PathBuf>,
) -> actix_web::Result<HttpResponse> {
    let path_str = path.into_inner();
    let file_path = if path_str.is_empty() {
        root.join("index.html")
    } else {
        root.join(&path_str)
    };

    // Security: prevent path traversal
    let canonical = file_path
        .canonicalize()
        .map_err(|_| actix_web::error::ErrorNotFound("File not found"))?;
    let root_canonical = root
        .canonicalize()
        .map_err(|_| actix_web::error::ErrorInternalServerError("Invalid root"))?;
    if !canonical.starts_with(&root_canonical) {
        return Err(actix_web::error::ErrorForbidden("Access denied"));
    }

    // Check Accept-Encoding header
    let accept_encoding = req
        .headers()
        .get("Accept-Encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let accepts_br = accept_encoding.contains("br");
    let accepts_gzip = accept_encoding.contains("gzip");

    // Try pre-compressed versions
    let (serve_path, encoding) = if accepts_br {
        let br_path = PathBuf::from(format!("{}.br", canonical.display()));
        if br_path.exists() {
            (br_path, Some("br"))
        } else if accepts_gzip {
            let gz_path = PathBuf::from(format!("{}.gz", canonical.display()));
            if gz_path.exists() {
                (gz_path, Some("gzip"))
            } else {
                (canonical.clone(), None)
            }
        } else {
            (canonical.clone(), None)
        }
    } else if accepts_gzip {
        let gz_path = PathBuf::from(format!("{}.gz", canonical.display()));
        if gz_path.exists() {
            (gz_path, Some("gzip"))
        } else {
            (canonical.clone(), None)
        }
    } else {
        (canonical.clone(), None)
    };

    // Determine content type from original file (not .br/.gz)
    let content_type = mime_guess::from_path(&canonical)
        .first_or_octet_stream()
        .to_string();

    // Open and serve the file
    let file = NamedFile::open(&serve_path)?;
    let mut response = file.into_response(&req);

    // Set correct content type (based on original file, not .br)
    response.headers_mut().insert(
        actix_web::http::header::CONTENT_TYPE,
        content_type.parse().unwrap(),
    );

    // Set content encoding if serving compressed version
    if let Some(enc) = encoding {
        response.headers_mut().insert(
            actix_web::http::header::CONTENT_ENCODING,
            enc.parse().unwrap(),
        );
    }

    // CORS for local development
    response.headers_mut().insert(
        actix_web::http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );

    Ok(response)
}

/// Serve index.html for root path
async fn index(req: HttpRequest, root: web::Data<PathBuf>) -> actix_web::Result<HttpResponse> {
    serve_file(req, web::Path::from("index.html".to_string()), root).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let root = args.dir.canonicalize().unwrap_or_else(|_| {
        eprintln!("Error: Directory '{}' not found", args.dir.display());
        std::process::exit(1);
    });

    println!("╭─────────────────────────────────────────────────────────╮");
    println!("│  Eulumdat Dev Server                                    │");
    println!("├─────────────────────────────────────────────────────────┤");
    println!(
        "│  Serving: {:<45} │",
        root.display()
            .to_string()
            .chars()
            .take(45)
            .collect::<String>()
    );
    println!("│  URL:     http://{}:{:<36} │", args.host, args.port);
    println!("│                                                         │");
    println!("│  Features:                                              │");
    println!("│    ✓ Brotli pre-compressed files (.br)                  │");
    println!("│    ✓ Gzip pre-compressed files (.gz)                    │");
    println!("│    ✓ Correct MIME types for WASM                        │");
    println!("│    ✓ CORS headers for local development                 │");
    println!("╰─────────────────────────────────────────────────────────╯");
    println!();
    println!("Press Ctrl+C to stop");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(root.clone()))
            .wrap(middleware::Logger::new("%s %r (%Ts)"))
            .route("/", web::get().to(index))
            .route("/{path:.*}", web::get().to(serve_file))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}

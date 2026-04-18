//! Eulumdat SSR Server
//!
//! A unified server that provides:
//! - Server-side rendering with Leptos
//! - Brotli/Gzip compression for all responses
//! - Pre-compressed static file serving (.br, .gz)
//! - Usage statistics and analytics
//! - API endpoints for future features
//!
//! Usage:
//! ```text
//!   eulumdat-server [OPTIONS]
//!
//! Options:
//!   -p, --port <PORT>    Port to listen on [default: 8042]
//!   -H, --host <HOST>    Host to bind to [default: 0.0.0.0]
//!   --dist <DIR>         Static files directory [default: ./dist]
//! ```

mod stats;

use actix_files::NamedFile;
use actix_web::{http::header, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use clap::Parser;
use std::path::{Path, PathBuf};

use stats::{Stats, StatsEvent};

// Embedded 404 SVG
const NOT_FOUND_SVG: &str = include_str!("../../../assets/404.svg");

#[derive(Parser, Debug)]
#[command(name = "eulumdat-server")]
#[command(about = "Eulumdat SSR server with Brotli compression and analytics")]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8042")]
    port: u16,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    host: String,

    /// Static files directory (for WASM, JS, CSS)
    #[arg(long, default_value = "./dist")]
    dist: PathBuf,
}

/// Generate a nice 404 page with the embedded SVG
fn not_found_page() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>404 - Page Not Found | Eulumdat</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            background: #070810;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
            color: #c7f8ff;
            padding: 2rem;
        }}
        .container {{
            max-width: 1400px;
            width: 100%;
            text-align: center;
        }}
        .svg-container {{
            width: 100%;
            max-width: 1000px;
            margin: 0 auto 2rem;
        }}
        .svg-container svg {{
            width: 100%;
            height: auto;
        }}
        .message {{
            opacity: 0.8;
            margin-bottom: 2rem;
        }}
        .home-link {{
            display: inline-block;
            padding: 0.75rem 2rem;
            background: linear-gradient(135deg, #22d8ff 0%, #9ff7ff 100%);
            color: #070810;
            text-decoration: none;
            border-radius: 8px;
            font-weight: 600;
            transition: transform 0.2s, box-shadow 0.2s;
        }}
        .home-link:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 20px rgba(34, 216, 255, 0.4);
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="svg-container">
            {svg}
        </div>
        <p class="message">The process you're looking for seems to be in the dark.</p>
        <a href="/" class="home-link">← Back to Editor</a>
    </div>
</body>
</html>"#,
        svg = NOT_FOUND_SVG
    )
}

/// Serve static files with pre-compressed variant support
async fn serve_static_file(
    req: &HttpRequest,
    path_str: &str,
    dist: &Path,
    stats: &Stats,
) -> Option<HttpResponse> {
    let file_path = dist.join(path_str);

    // Security: prevent path traversal (reject ".." components before resolving symlinks)
    for component in Path::new(path_str).components() {
        if matches!(component, std::path::Component::ParentDir) {
            return None;
        }
    }

    // Resolve symlinks for the actual file
    let canonical = file_path.canonicalize().ok()?;

    // Check Accept-Encoding header
    let accept_encoding = req
        .headers()
        .get(header::ACCEPT_ENCODING)
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

    // Track WASM downloads
    if path_str.ends_with(".wasm") {
        let size = std::fs::metadata(&serve_path).map(|m| m.len()).unwrap_or(0);
        stats.record(StatsEvent::WasmDownload { size });
    }

    // Determine content type from original file
    let content_type = mime_guess::from_path(&canonical)
        .first_or_octet_stream()
        .to_string();

    // Open and serve the file
    let file = NamedFile::open(&serve_path).ok()?;
    let mut response = file.into_response(req);

    // Set correct content type
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type.parse().unwrap());

    // Set content encoding if serving compressed version
    if let Some(enc) = encoding {
        response
            .headers_mut()
            .insert(header::CONTENT_ENCODING, enc.parse().unwrap());
    }

    // Cache headers for hashed files (immutable)
    if path_str.contains('-')
        && (path_str.ends_with(".wasm") || path_str.ends_with(".js") || path_str.ends_with(".css"))
    {
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            "public, max-age=31536000, immutable".parse().unwrap(),
        );
    }

    Some(response)
}

/// Serve static files, return 404 page if not found
async fn serve_static(
    req: HttpRequest,
    path: web::Path<String>,
    dist: web::Data<PathBuf>,
    stats: web::Data<Stats>,
) -> HttpResponse {
    let path_str = path.into_inner();

    if let Some(response) = serve_static_file(&req, &path_str, &dist, &stats).await {
        response
    } else {
        HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(not_found_page())
    }
}

/// Serve index.html (the main entry point)
async fn index(
    req: HttpRequest,
    dist: web::Data<PathBuf>,
    stats: web::Data<Stats>,
) -> HttpResponse {
    stats.record(StatsEvent::PageView {
        path: "/".to_string(),
    });

    if let Some(response) = serve_static_file(&req, "index.html", &dist, &stats).await {
        response
    } else {
        HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(not_found_page())
    }
}

/// Serve index.html for secret export path (enables PDF/Typst export)
async fn index_with_export(
    req: HttpRequest,
    dist: web::Data<PathBuf>,
    stats: web::Data<Stats>,
) -> HttpResponse {
    stats.record(StatsEvent::PageView {
        path: "/htr_pdf_typ_test_almost_secret".to_string(),
    });

    if let Some(response) = serve_static_file(&req, "index.html", &dist, &stats).await {
        response
    } else {
        HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(not_found_page())
    }
}

/// API: Get current statistics
async fn api_stats(stats: web::Data<Stats>) -> HttpResponse {
    HttpResponse::Ok().json(stats.summary())
}

/// API: Track client-side events
#[derive(serde::Deserialize)]
struct ClientEvent {
    event: String,
    data: Option<serde_json::Value>,
}

async fn api_track(event: web::Json<ClientEvent>, stats: web::Data<Stats>) -> HttpResponse {
    stats.record(StatsEvent::ClientEvent {
        name: event.event.clone(),
        data: event.data.clone(),
    });
    HttpResponse::Ok().json(serde_json::json!({"ok": true}))
}

/// Health check endpoint
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Default 404 handler
async fn not_found() -> HttpResponse {
    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(not_found_page())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("eulumdat_server=info".parse().unwrap())
                .add_directive("actix_web=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    let dist = args.dist.canonicalize().unwrap_or_else(|_| {
        eprintln!(
            "Warning: dist directory '{}' not found, using current dir",
            args.dist.display()
        );
        std::env::current_dir().unwrap()
    });

    let stats = web::Data::new(Stats::new());

    println!("╭─────────────────────────────────────────────────────────╮");
    println!(
        "│  Eulumdat Server v{}                              │",
        env!("CARGO_PKG_VERSION")
    );
    println!("├─────────────────────────────────────────────────────────┤");
    println!("│  URL:     http://{}:{:<36} │", args.host, args.port);
    println!(
        "│  Static:  {:<47} │",
        dist.display()
            .to_string()
            .chars()
            .take(47)
            .collect::<String>()
    );
    println!("│                                                         │");
    println!("│  Features:                                              │");
    println!("│    ✓ Brotli/Gzip pre-compressed files                   │");
    println!("│    ✓ Usage statistics & analytics                       │");
    println!("│    ✓ Immutable caching for hashed files                 │");
    println!("│    ✓ Custom 404 page                                    │");
    println!("│                                                         │");
    println!("│  API Endpoints:                                         │");
    println!("│    GET  /api/health  - Health check                     │");
    println!("│    GET  /api/stats   - Usage statistics                 │");
    println!("│    POST /api/track   - Track client events              │");
    println!("╰─────────────────────────────────────────────────────────╯");
    println!();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(dist.clone()))
            .app_data(stats.clone())
            .wrap(middleware::Logger::new("%s %r (%Ts) %{Content-Encoding}o"))
            .wrap(middleware::Compress::default())
            // API routes (must be before wildcard)
            .route("/api/health", web::get().to(health))
            .route("/api/stats", web::get().to(api_stats))
            .route("/api/track", web::post().to(api_track))
            // App routes
            .route("/", web::get().to(index))
            .route(
                "/htr_pdf_typ_test_almost_secret",
                web::get().to(index_with_export),
            )
            // Static files (catches remaining paths)
            .route("/{path:.*}", web::get().to(serve_static))
            // 404 fallback
            .default_service(web::route().to(not_found))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await
}

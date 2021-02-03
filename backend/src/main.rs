use anyhow::Context;
use backend::utils::single_page_application;
use backend::{
    balanced_or_tree, debug_boxed, exists, setup_assets_directory, setup_database, setup_logger,
};
use std::env;
use tokio::fs;
use warp::Filter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = "logs/backend.log";
    if exists(path).await? {
        fs::remove_file(path)
            .await
            .context("failed to remove previous log file")?
    }
    fs::File::create(path)
        .await
        .context("failed to create log file")?;
    let file = tokio::task::spawn_blocking(move || {
        std::fs::OpenOptions::new()
            .append(true)
            .write(true)
            .read(true)
            .open(path)
    })
    .await?
    .context("failed to open log file")?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(file);

    setup_logger(non_blocking)?;

    setup_assets_directory()
        .await
        .context("failed to setup assets directory")?;

    let pool = setup_database().await.context("failed to setup database")?;

    let dist_dir = env::var("DIST_DIR").context("environment variable `DIST_DIR` not defined")?;

    let api = backend::api(pool.clone());
    let spa = single_page_application(dist_dir);

    let routes = balanced_or_tree!(api, spa);

    #[cfg(debug_assertions)]
    let routes = routes.with(
        warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["GET", "POST", "OPTIONS"])
            .allow_headers(vec!["authorization", "content-type"]),
    );

    let routes = routes.with(warp::trace::request());

    let port = env::var("PORT")
        .map(|it| it.parse().expect("invalid port"))
        .unwrap_or(9090);

    let (addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], port), async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C signal handler");
        });

    tracing::info!("running server at http://{}/", addr);
    server.await;
    Ok(())
}

use std::error::Error;
use std::net::SocketAddr;

use oauth2_mock::{Config, app};
use tokio::net::TcpListener;
use tokio::signal::unix::{Signal, SignalKind, signal};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    let address = SocketAddr::from(([0, 0, 0, 0], config.port));
    let listener = TcpListener::bind(address).await?;
    let terminate = signal(SignalKind::terminate())?;
    println!("oauth2-mock serving issuer {} on {address}", config.issuer);

    axum::serve(listener, app(config)?)
        .with_graceful_shutdown(shutdown_signal(terminate))
        .await?;
    Ok(())
}

async fn shutdown_signal(mut terminate: Signal) {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

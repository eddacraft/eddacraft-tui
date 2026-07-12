use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anvil_dashboard_server::serve;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4217);
    let listener = TcpListener::bind(address).await?;
    eprintln!("Anvil dashboard API listening on http://{address}");
    serve(listener, workspace).await?;
    Ok(())
}

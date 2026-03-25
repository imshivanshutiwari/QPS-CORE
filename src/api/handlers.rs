use tonic::transport::Server;
use tracing::info;

use crate::api::grpc_server::{proto::qps_server::QpsServer, QpsService};

/// Bind and run the gRPC server on `addr` (e.g. `"[::]:50051"`).
pub async fn run_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr = addr.parse()?;
    info!("gRPC server listening on {addr}");

    Server::builder()
        .add_service(QpsServer::new(QpsService))
        .serve(addr)
        .await?;

    Ok(())
}

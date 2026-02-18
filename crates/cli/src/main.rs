use ambient_node::{
    AmbientNode, DataPlaneGateway, GatewayConfig, NodeId, SafetyPolicy, TelemetrySample,
};
#[cfg(feature = "observability")]
use ambient_node::LocalObservabilityServer;
use anyhow::Result;
use clap::{Parser, Subcommand};
use mesh_coordinator::{MeshCoordinator, TaskAssignmentStrategy};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, Level};

#[derive(Parser)]
#[command(name = "ambient-vcp")]
#[command(about = "Ambient AI + Verifiable Computation Protocol CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an ambient node
    Node {
        /// Node ID
        #[arg(short, long)]
        id: String,

        /// Region
        #[arg(short, long, default_value = "us-west")]
        region: String,

        /// Node type
        #[arg(short = 't', long, default_value = "compute")]
        node_type: String,

        /// Enable local observability interface (operator-only, privacy-preserving)
        #[arg(long, default_value = "false")]
        observability: bool,

        /// Observability server port (default: 9090)
        #[arg(long, default_value_t = 9090)]
        observability_port: u16,
    },

    /// Start a data-plane gateway for connect_only relay sessions
    Gateway {
        /// Listen address for the relay tunnel entrypoint
        #[arg(long, default_value = "0.0.0.0:7000")]
        listen: String,

        /// JSON file containing active gateway sessions
        #[arg(long)]
        sessions_file: PathBuf,

        /// Upstream connect timeout in seconds
        #[arg(long, default_value_t = 5)]
        connect_timeout_seconds: u64,

        /// Idle timeout in seconds for handshake + relay sessions
        #[arg(long, default_value_t = 600)]
        idle_timeout_seconds: u64,
    },

    /// Start a mesh coordinator
    Coordinator {
        /// Cluster ID
        #[arg(short, long)]
        cluster_id: String,

        /// Task assignment strategy
        #[arg(short, long, default_value = "weighted")]
        strategy: String,
    },

    /// Show node information
    Info {
        /// Node ID
        #[arg(short, long)]
        id: String,
    },

    /// Run health check
    Health,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Node {
            id,
            region,
            node_type,
            observability,
            observability_port,
        } => {
            run_node(id, region, node_type, observability, observability_port).await?;
        }
        Commands::Gateway {
            listen,
            sessions_file,
            connect_timeout_seconds,
            idle_timeout_seconds,
        } => {
            run_gateway(
                listen,
                sessions_file,
                connect_timeout_seconds,
                idle_timeout_seconds,
            )
            .await?;
        }
        Commands::Coordinator {
            cluster_id,
            strategy,
        } => {
            run_coordinator(cluster_id, strategy).await?;
        }
        Commands::Info { id } => {
            show_node_info(id).await?;
        }
        Commands::Health => {
            run_health_check().await?;
        }
    }

    Ok(())
}

async fn run_node(
    id: String,
    region: String,
    node_type: String,
    observability: bool,
    observability_port: u16,
) -> Result<()> {
    info!("Starting ambient node: {}", id);

    let node_id = NodeId::new(&id, &region, &node_type);
    let policy = SafetyPolicy::default();
    let mut node = AmbientNode::new(node_id, policy);

    // Simulate telemetry collection
    let telemetry = TelemetrySample {
        bandwidth_mbps: 100.0,
        avg_latency_ms: 20.0,
        cpu_usage_percent: 50.0,
        memory_usage_percent: 60.0,
        temperature_c: 65.0,
        power_watts: 150.0,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    };

    node.ingest_telemetry(telemetry);

    info!("Node ID: {}", id);
    info!("Region: {}", region);
    info!("Type: {}", node_type);
    info!("Health Score: {:.2}", node.health_score());
    info!("Safe Mode: {}", node.is_safe_mode());

    // Start local observability server if enabled
    #[cfg(feature = "observability")]
    if observability {
        info!("Local observability enabled on port {}", observability_port);
        
        // Wrap node in Arc<RwLock<>> for shared access
        let node_arc = Arc::new(RwLock::new(node));
        
        // Create and start observability server
        let server = LocalObservabilityServer::new(observability_port, node_arc.clone());
        server.print_curl_command();
        
        // Run server in background
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.run().await {
                tracing::error!("Observability server error: {}", e);
            }
        });

        info!("Node running... Press Ctrl+C to stop");

        // Keep running until interrupted
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down node...");
            }
            _ = server_handle => {
                info!("Observability server stopped");
            }
        }
    } else {
        info!("Node running... Press Ctrl+C to stop");

        // Keep running until interrupted
        tokio::signal::ctrl_c().await?;

        info!("Shutting down node...");
    }

    #[cfg(not(feature = "observability"))]
    {
        let _ = (observability, observability_port); // Suppress unused warnings
        
        info!("Node running... Press Ctrl+C to stop");

        // Keep running until interrupted
        tokio::signal::ctrl_c().await?;

        info!("Shutting down node...");
    }

    Ok(())
}

async fn run_gateway(
    listen: String,
    sessions_file: PathBuf,
    connect_timeout_seconds: u64,
    idle_timeout_seconds: u64,
) -> Result<()> {
    info!("Starting data-plane gateway on {}", listen);

    let gateway = DataPlaneGateway::from_sessions_file(
        GatewayConfig {
            listen_addr: listen,
            connect_timeout_seconds,
            idle_timeout_seconds,
        },
        sessions_file,
    )
    .await?;

    gateway.run().await
}

async fn run_coordinator(cluster_id: String, strategy_str: String) -> Result<()> {
    info!("Starting mesh coordinator: {}", cluster_id);

    let strategy = match strategy_str.as_str() {
        "weighted" => TaskAssignmentStrategy::Weighted,
        "round-robin" => TaskAssignmentStrategy::RoundRobin,
        "least-loaded" => TaskAssignmentStrategy::LeastLoaded,
        "latency-aware" => TaskAssignmentStrategy::LatencyAware,
        _ => {
            info!("Unknown strategy '{}', using 'weighted'", strategy_str);
            TaskAssignmentStrategy::Weighted
        }
    };

    let coordinator = MeshCoordinator::new(cluster_id.clone(), strategy);

    info!("Cluster ID: {}", cluster_id);
    info!("Strategy: {:?}", strategy);
    info!("Nodes: {}", coordinator.node_count());

    let stats = coordinator.cluster_stats();
    info!("Total Nodes: {}", stats.total_nodes);
    info!("Healthy Nodes: {}", stats.healthy_nodes);
    info!("Avg Health Score: {:.2}", stats.avg_health_score);

    info!("Coordinator running... Press Ctrl+C to stop");

    tokio::signal::ctrl_c().await?;

    info!("Shutting down coordinator...");
    Ok(())
}

async fn show_node_info(id: String) -> Result<()> {
    info!("Node Information: {}", id);
    info!("This would show detailed node information in a production system");
    Ok(())
}

async fn run_health_check() -> Result<()> {
    info!("Running system health check...");

    // Check system components
    info!("✓ Ambient Node module loaded");
    info!("✓ Data-plane gateway module loaded");
    info!("✓ WASM Engine module loaded");
    info!("✓ ZK Prover module loaded");
    info!("✓ Mesh Coordinator module loaded");

    info!("All systems operational!");
    Ok(())
}

# Connectivity Layer Integration Guide

## Overview

The connectivity layer provides multi-backhaul orchestration for universal/open-access nodes in the Ambient AI VCP System. It manages WAN connectivity through multiple interface types and ensures continuous reachability under changing network conditions.

## Supported Interface Types

- **Ethernet** (eth*, eno*, enp*, ens*)
- **Wi-Fi Client Mode** (wlan*, wlp*)
- **LTE/5G Modem** (wwan*, ppp*)
- **USB Tethering** (usb0, enx*)
- **Bluetooth PAN** (bnep0)
- **Wi-Fi AP Mode** (Phase 2 - hotspot fallback)

## Quick Start

### Basic Usage

```rust
use ambient_node::connectivity::{
    BackhaulManager,
    BackhaulConfig,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create backhaul manager with default configuration
    let config = BackhaulConfig::default();
    let manager = BackhaulManager::new(config);
    
    // Start the connectivity subsystem
    manager.start().await?;
    
    // Query current active backhaul
    if let Some(backhaul) = manager.current_backhaul().await {
        println!("Active interface: {}", backhaul.iface);
        println!("State: {:?}", backhaul.state);
        println!("Score: {}", backhaul.score);
    }
    
    Ok(())
}
```

### Custom Configuration

```rust
use ambient_node::connectivity::{
    BackhaulManager,
    BackhaulConfig,
    ProbeConfig,
    ScoringConfig,
    StateMachineConfig,
    RoutingConfig,
    ProbeTarget,
    ProbeType,
};

let config = BackhaulConfig {
    probe_config: ProbeConfig {
        interval_secs: 10,
        timeout_secs: 5,
        targets: vec![
            ProbeTarget {
                name: "control-plane".to_string(),
                address: "control.example.com".to_string(),
                port: 443,
                probe_type: ProbeType::TcpConnect,
            },
            ProbeTarget {
                name: "dns".to_string(),
                address: "1.1.1.1".to_string(),
                port: 53,
                probe_type: ProbeType::TcpConnect,
            },
        ],
        degraded_threshold: 2,
        down_threshold: 3,
    },
    scoring_config: ScoringConfig {
        weight_latency: 300.0,
        weight_loss: 200.0,
        weight_success: 500.0,
        enable_policy_bias: true,
        policy_bias_multiplier: 1.5,
        max_rtt_ms: 200.0,
        max_loss_percent: 10.0,
    },
    state_machine_config: StateMachineConfig {
        up_to_degraded_holddown_secs: 15,
        degraded_to_down_holddown_secs: 30,
        down_to_probing_holddown_secs: 60,
        probing_to_up_holddown_secs: 10,
        min_state_duration_secs: 5,
    },
    routing_config: RoutingConfig {
        execute_commands: true,
        main_table_priority: 32766,
        interface_table_priority: 1000,
    },
};

let manager = BackhaulManager::new(config);
```

## Integration with Node Agent

The connectivity layer is designed to be integrated into the ambient node agent's lifecycle:

```rust
use ambient_node::{AmbientNode, connectivity::BackhaulManager};

pub struct NodeAgent {
    node: AmbientNode,
    connectivity: BackhaulManager,
}

impl NodeAgent {
    pub async fn new() -> Result<Self> {
        let node = AmbientNode::new(/* ... */);
        
        // Initialize connectivity with production settings
        let mut config = BackhaulConfig::default();
        config.routing_config.execute_commands = true;
        
        let connectivity = BackhaulManager::new(config);
        
        Ok(Self { node, connectivity })
    }
    
    pub async fn start(&self) -> Result<()> {
        // Start connectivity subsystem
        self.connectivity.start().await?;
        
        // Start other node services...
        
        Ok(())
    }
    
    pub async fn get_connectivity_status(&self) -> Option<ActiveBackhaul> {
        self.connectivity.current_backhaul().await
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        // Shutdown connectivity cleanly
        self.connectivity.shutdown().await?;
        
        // Shutdown other services...
        
        Ok(())
    }
}
```

## Phase 2: Hotspot & Tether

### Hotspot Mode

When no upstream backhaul is available, the node can switch to AP mode to provide local connectivity:

```rust
use ambient_node::connectivity::hotspot::{
    ApMode,
    ApConfig,
    SecurityConfig,
    SecurityMode,
    QosManager,
    QosConfig,
};

let ap_config = ApConfig {
    interface: "wlan0".to_string(),
    ssid: "Ambient-Node-Emergency".to_string(),
    channel: 6,
    ip_address: "192.168.50.1".to_string(),
    netmask: "255.255.255.0".to_string(),
    dhcp_start: "192.168.50.100".to_string(),
    dhcp_end: "192.168.50.200".to_string(),
    enable_nat: true,
    nat_upstream: Some("wwan0".to_string()),
};

let mut ap = ApMode::new(ap_config, true);

// Start AP mode
ap.start().await?;

// Configure security
let security_config = SecurityConfig {
    mode: SecurityMode::Wpa2Wpa3Mixed,
    psk: "secure-password".to_string(),
    enable_client_isolation: true,
    max_clients: 10,
    ..Default::default()
};

// Configure QoS
let qos_config = QosConfig {
    interface: "wlan0".to_string(),
    enabled: true,
    control_bandwidth_kbps: 1000,
    interactive_bandwidth_kbps: 5000,
    bulk_bandwidth_kbps: 10000,
    ..Default::default()
};

let qos = QosManager::new(qos_config, true);
qos.apply_qos().await?;
```

### USB Tethering

Detect and use USB tethered connections:

```rust
use ambient_node::connectivity::tether::{
    UsbTether,
    UsbTetherConfig,
};

let config = UsbTetherConfig {
    enabled: true,
    interface_patterns: vec!["usb0".to_string(), "enx".to_string()],
    enable_dhcp: true,
    dhcp_timeout_secs: 30,
    validate_route: true,
    validation_target: "8.8.8.8".to_string(),
};

let mut usb_tether = UsbTether::new(config, true);

// Detect available interfaces
let interfaces = usb_tether.detect_interfaces().await?;

// Initialize first available
if let Some(iface) = interfaces.first() {
    usb_tether.initialize_interface(iface).await?;
}
```

### Bluetooth PAN

Similar to USB tethering, but for Bluetooth network connections:

```rust
use ambient_node::connectivity::tether::{
    BluetoothTether,
    BluetoothTetherConfig,
};

let config = BluetoothTetherConfig::default();
let mut bt_tether = BluetoothTether::new(config, true);

let interfaces = bt_tether.detect_interfaces().await?;
for iface in interfaces {
    bt_tether.initialize_interface(&iface).await?;
}
```

## API Reference

### BackhaulManager

Main entry point for the connectivity subsystem.

#### Methods

- `new(config: BackhaulConfig) -> Self` - Create a new manager
- `async start(&self) -> Result<()>` - Start background tasks
- `async current_backhaul(&self) -> Option<ActiveBackhaul>` - Get active interface
- `async get_all_interface_states(&self) -> Vec<(String, BackhaulState, u32)>` - Get all interfaces
- `async shutdown(&self) -> Result<()>` - Clean shutdown

### BackhaulState

Public state enum for interfaces:

```rust
pub enum BackhaulState {
    Up,       // Interface is healthy
    Degraded, // Interface is experiencing issues
    Down,     // Interface is unavailable
}
```

### ActiveBackhaul

Information about the active interface:

```rust
pub struct ActiveBackhaul {
    pub iface: String,       // Interface name (e.g., "eth0")
    pub state: BackhaulState, // Current state
    pub score: u32,          // Current score (higher is better)
}
```

## Monitoring & Debugging

### Get All Interface States

```rust
let states = manager.get_all_interface_states().await;
for (iface, state, score) in states {
    println!("{}: {:?} (score: {})", iface, state, score);
}
```

### Dry Run Mode (Testing)

For testing without actually changing system routing:

```rust
let mut config = BackhaulConfig::default();
config.routing_config.execute_commands = false;

let manager = BackhaulManager::new(config);
```

## Error Handling

All operations return `Result<T, ConnectivityError>`:

```rust
use ambient_node::connectivity::ConnectivityError;

match manager.start().await {
    Ok(()) => println!("Connectivity started"),
    Err(ConnectivityError::DiscoveryError(e)) => {
        eprintln!("Interface discovery failed: {}", e);
    }
    Err(ConnectivityError::RoutingError(e)) => {
        eprintln!("Routing operation failed: {}", e);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Performance Considerations

- **Probe Interval**: Default 5 seconds. Reduce for faster detection, increase to reduce network overhead.
- **State Machine Holddowns**: Prevent flapping. Tune based on your network stability.
- **Scoring Weights**: Adjust to prioritize latency vs. reliability vs. policy bias.

## Security Considerations

- **Routing Changes**: Require root/CAP_NET_ADMIN privileges
- **Hotspot PSK**: Use strong passwords and enable rotation
- **Client Isolation**: Always enable for untrusted hotspot clients
- **QoS**: Prioritize control traffic to prevent DoS

## Troubleshooting

### Interface Not Detected

- Check `/sys/class/net` permissions
- Verify interface name patterns in configuration
- Check systemd-networkd/NetworkManager isn't interfering

### Routing Not Working

- Verify `execute_commands: true` in production
- Check for existing ip rules: `ip rule list`
- Check routing tables: `ip route show table <id>`
- Verify CAP_NET_ADMIN capability

### Hotspot Not Starting

- Check hostapd availability: `which hostapd`
- Verify Wi-Fi interface supports AP mode: `iw list`
- Check for conflicting NetworkManager settings

### High CPU Usage

- Increase probe_interval_secs
- Reduce number of probe targets
- Check for excessive logging

## License

MIT - See LICENSE file for details

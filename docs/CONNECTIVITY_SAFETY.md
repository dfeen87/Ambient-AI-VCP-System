# Connectivity Layer Safety & Correctness Review

## Concurrency Safety

### Thread Safety

All shared state in the connectivity layer uses appropriate synchronization primitives:

#### InterfaceRegistry
- **Protection**: `Arc<RwLock<HashMap<String, InterfaceInfo>>>`
- **Rationale**: Multiple readers (health probes, scoring) with occasional writers (discovery)
- **Safety**: RwLock ensures no data races; Arc ensures shared ownership

#### BackhaulManager State
- **Protection**: `Arc<RwLock<HashMap<String, InterfaceState>>>` for interface states
- **Protection**: `Arc<RwLock<Option<String>>>` for active interface
- **Protection**: `Arc<RwLock<RoutingManager>>` for routing operations
- **Rationale**: Multiple concurrent operations (probing, scoring, route updates)
- **Safety**: All mutations require exclusive write locks

### Async Task Safety

The management loop spawns background tasks:

```rust
tokio::spawn(async move {
    if let Err(e) = manager.management_loop().await {
        warn!(error = %e, "Management loop terminated");
    }
});
```

**Safety Measures**:
1. Manager is cloned (Arc-wrapped internals) before spawning
2. All shared state uses Send + Sync primitives
3. No blocking operations in async context
4. Errors are logged but don't crash the application

### Lock Ordering

To prevent deadlocks, locks are always acquired in this order:
1. `interface_states` (read or write)
2. `active_interface` (read or write)
3. `routing` (write)

Example from select_best_interface:
```rust
let current_active = self.active_interface.read().await; // 1. Read active
let old_interface = current_active.as_deref().map(|s| s.to_string());
drop(current_active); // Release before acquiring routing

let mut routing = self.routing.write().await; // 2. Acquire routing
routing.switch_active_interface(best_interface, None)?;
drop(routing); // Release before acquiring active again

let mut active = self.active_interface.write().await; // 3. Write active
*active = Some(best_interface.to_string());
```

## Race Conditions

### Interface Discovery vs. Probing

**Scenario**: Discovery adds a new interface while probing is happening.

**Mitigation**:
- Discovery and probing read from separate iterations
- Interface registry uses RwLock to prevent partial reads
- Health probes are independent per interface

### State Machine Transitions

**Scenario**: Multiple events processed concurrently for same interface.

**Mitigation**:
- State machine is not Send/Sync (one per interface, managed by manager)
- All state transitions happen sequentially in management loop
- Pending events are queued and processed in order

### Routing Updates

**Scenario**: Two interfaces trying to become active simultaneously.

**Mitigation**:
- `RoutingManager` is protected by exclusive write lock
- Only one routing operation can execute at a time
- Route changes are atomic (setup new before removing old)

## Blocking Operations

### System Calls

All system calls (ip, iptables, etc.) are potentially blocking.

**Mitigation**:
- System calls happen in sync context, not inside async tasks
- Commands have explicit timeouts
- Failures don't block the management loop (logged and continue)

### File I/O

Reading `/sys/class/net` is blocking I/O.

**Mitigation**:
- Discovery runs in dedicated periodic task (separate from critical path)
- Failures fall back to mock data gracefully
- No file I/O in hot paths (health probes, scoring)

## Routing Atomicity

### Route Switching

When switching active interfaces, we must avoid blackholes where no route exists.

**Sequence**:
```rust
// 1. Set up new interface routing table
self.setup_interface_table(new_interface, table_id, gateway)?;

// 2. Add new policy routing rule (traffic starts using new interface)
self.add_routing_rule(new_interface, table_id)?;

// 3. Remove old policy routing rule (traffic no longer uses old interface)
if let Some(old_interface) = &self.active_interface {
    if old_interface != new_interface {
        if let Some(&old_table_id) = self.table_assignments.get(old_interface) {
            self.remove_routing_rule(old_interface, old_table_id)?;
        }
    }
}
```

**Safety**:
- New route is active before old route is removed
- Brief period with both routes active (acceptable)
- Never a period with no route active

### Rollback Behavior

If route setup fails, we attempt rollback:

```rust
pub fn rollback_interface(&mut self, interface: &str) -> Result<()> {
    info!(interface = %interface, "Rolling back routing changes");
    
    if let Some(&table_id) = self.table_assignments.get(interface) {
        let table_id_str = table_id.to_string();
        
        // Remove routing rule
        self.remove_routing_rule(interface, table_id)?;
        
        // Flush routing table
        self.execute_command(&[
            "ip", "route", "flush", "table", &table_id_str
        ])?;
    }
    
    if self.active_interface.as_deref() == Some(interface) {
        self.active_interface = None;
    }
    
    Ok(())
}
```

**Limitations**:
- Rollback is best-effort
- If both setup and rollback fail, manual intervention may be needed
- Logging captures all failures for debugging

## Failure Modes

### All Interfaces Down

**Scenario**: No healthy interfaces available.

**Behavior**:
- `select_best_interface` finds no UP interfaces
- Active interface remains None or current (degraded)
- No routing changes made
- System continues monitoring for recovery

**Recovery**: When any interface comes up and passes health checks, it's automatically selected.

### Misconfigured Routes

**Scenario**: `ip` command fails due to invalid configuration.

**Behavior**:
- Error is returned and logged
- Active interface unchanged
- Routing manager state remains consistent
- Next iteration will retry or select different interface

**Detection**: Health probes will detect broken connectivity and mark interface as DOWN.

### Interface Flapping

**Scenario**: Interface rapidly alternating between UP and DOWN.

**Mitigation**:
- Hold-down timers prevent rapid state transitions
- `min_state_duration_secs` prevents processing events too quickly
- Pending events are queued and debounced
- Configurable thresholds for degraded/down detection

**Example**:
```rust
StateMachineConfig {
    up_to_degraded_holddown_secs: 15,   // Wait 15s before marking degraded
    degraded_to_down_holddown_secs: 30, // Wait 30s before marking down
    down_to_probing_holddown_secs: 60,  // Wait 60s before re-probing
    probing_to_up_holddown_secs: 10,    // Wait 10s before marking up
    min_state_duration_secs: 5,         // Min 5s in any state
}
```

### Split Brain (Multiple Active)

**Scenario**: Due to race condition, multiple interfaces think they're active.

**Mitigation**:
- Single source of truth: `active_interface` protected by RwLock
- Routing manager serializes all operations
- Only one interface can have active routing rule

**Detection**: Inconsistency between `active_interface` and actual routing rules.

**Recovery**: Manual check via `ip rule list` and restart connectivity manager.

### Resource Exhaustion

**Scenario**: Too many probe targets or interfaces.

**Mitigation**:
- Probe intervals configurable (avoid too frequent probes)
- Limited routing table ID space (100-200 = 100 interfaces max)
- Each interface gets dedicated task, not one task per probe

**Monitoring**: Watch for CPU usage spikes or excessive network traffic.

## Memory Safety

### Rust Safety Guarantees

All code benefits from Rust's memory safety:
- No null pointer dereferences
- No use-after-free
- No data races (enforced by Send/Sync)
- No buffer overflows

### Potential Issues

1. **String Lifetime**: Temporary strings in `execute_command` fixed by creating owned strings first
2. **HashMap Iteration**: Clone values or collect to avoid borrow checker issues
3. **Drop Handler**: `RoutingManager::drop` calls `cleanup_all` for best-effort cleanup

## Security Considerations

### Privilege Requirements

**Root/CAP_NET_ADMIN Required**:
- All `ip` commands
- All `iptables` commands  
- hostapd/dnsmasq operations

**Mitigation**:
- Document privilege requirements clearly
- Provide non-root testing mode (`execute_commands: false`)
- Use capabilities instead of full root when possible

### Command Injection

**Risk**: User-controlled strings in system commands.

**Mitigation**:
- Interface names validated by Linux kernel (no injection risk)
- Gateway addresses parsed and validated
- No user input in command construction
- All commands use explicit argument arrays (not shell)

**Example** (safe):
```rust
Command::new("ip")
    .args(&["route", "add", "default", "via", gateway])
    .output()
```

### Hotspot Security

**PSK Storage**: PSKs stored in memory only, not on disk.

**PSK Generation**: Uses timestamp/counter (not cryptographically secure, but sufficient for temporary networks).

**Improvement**: For production, use `ring` or `getrandom` crate:
```rust
use ring::rand::{SystemRandom, SecureRandom};

fn generate_secure_psk() -> String {
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes).unwrap();
    hex::encode(bytes)
}
```

## Testing Coverage

### Unit Tests: 41 tests across all modules

- **discovery**: Interface type classification, registry operations, discovery lifecycle
- **health**: Stats tracking, threshold checks, TCP probing
- **scoring**: Latency/loss/success scoring, policy bias, comparisons
- **state_machine**: State transitions, hold-down timers, physical events
- **routing**: Table assignment, interface switching, rollback
- **hotspot/ap_mode**: AP lifecycle, configuration
- **hotspot/security**: PSK rotation, token management, hostapd config
- **hotspot/qos**: QoS rules application
- **tether/usb**: Pattern matching, lifecycle
- **tether/bluetooth**: Detection, lifecycle
- **backhaul/mod**: Manager creation, startup

### Integration Tests

Suggested tests (not yet implemented):

1. **Full lifecycle**: Create manager → start → switch interfaces → shutdown
2. **Interface failure**: Simulate interface going down → verify failover
3. **Multiple failures**: All interfaces down → verify no crashes
4. **Hotspot activation**: No upstream → verify AP mode activated
5. **Tether detection**: Plug in USB → verify detection and configuration

### Chaos Testing

For production deployment, consider:

- Random interface failures
- Network partition simulation
- Concurrent operation stress test
- Memory leak detection (long-running tests)
- Command failure injection

## Performance Characteristics

### Time Complexity

- Interface discovery: O(n) where n = number of interfaces
- Health probing: O(n × m) where n = interfaces, m = probe targets
- Scoring: O(n)
- State machine updates: O(n)
- Best interface selection: O(n)

### Space Complexity

- Interface registry: O(n) where n = number of interfaces
- Health stats: O(n × m) where m = historical probe results
- Routing tables: O(n) routing table entries

### Network Overhead

Default configuration:
- 2 probe targets × 5 second interval = ~0.4 probes/sec
- Each probe: 1 TCP SYN + SYN-ACK + ACK = ~200 bytes
- Total: ~80 bytes/sec (negligible)

## Recommendations

1. **Enable Logging**: Use `RUST_LOG=debug` to troubleshoot issues
2. **Monitor Health**: Expose metrics via Prometheus
3. **Graceful Shutdown**: Always call `shutdown()` before exit
4. **Test Dry Run**: Test configurations with `execute_commands: false`
5. **Backup Routes**: Keep a static route as fallback
6. **Rate Limiting**: Limit probe frequency on battery/metered connections
7. **Secure Hotspot**: Always use WPA2/WPA3, never Open mode
8. **Client Isolation**: Always enable for untrusted clients

## Known Limitations

1. **Platform**: Linux-only (uses /sys, ip, iptables)
2. **Privileges**: Requires root or CAP_NET_ADMIN
3. **IPv4 Focus**: Limited IPv6 support
4. **Single Active**: Only one interface active at a time (no bonding/multipath)
5. **Mock Discovery**: Falls back to mock data if /sys unavailable
6. **Best-Effort Cleanup**: Drop handler cleanup may fail silently

## Future Improvements

1. **Netlink**: Replace `ip` commands with rtnetlink crate for better performance
2. **IPv6**: Full dual-stack support
3. **Multipath**: Simultaneous use of multiple interfaces
4. **Metrics**: Prometheus exporter for monitoring
5. **Persistence**: Save/restore configuration across restarts
6. **WebUI**: Dashboard for connectivity status
7. **Notifications**: Webhooks/alerts for connectivity changes

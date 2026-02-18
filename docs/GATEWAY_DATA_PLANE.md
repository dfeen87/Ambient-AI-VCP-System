# Data-Plane Gateway Service (`connect_only`)

This service runs on `open_internet` nodes and performs actual end-to-end relay for `connect_only` sessions.

## What it enforces

- Session authentication (`session_id` + `session_token`)
- Session expiration checks
- Destination policy checks against an allowlist (`allowed_destinations`)
- Live TCP relay (`copy_bidirectional`) between client and upstream destination

## Start gateway

```bash
ambient-vcp gateway \
  --listen 0.0.0.0:7000 \
  --sessions-file ./gateway-sessions.json \
  --connect-timeout-seconds 5 \
  --idle-timeout-seconds 600
```

## Session file

```json
[
  {
    "session_id": "sess_123",
    "session_token": "cs_token_from_api",
    "egress_profile": "allowlist_domains",
    "destination_policy_id": "policy_web_basic_v1",
    "allowed_destinations": ["*.example.com", "1.1.1.1"],
    "expires_at_epoch_seconds": 1735689600
  }
]
```

## Tunnel handshake

Clients open a TCP connection to the gateway and send one newline-delimited JSON object:

```json
{"session_id":"sess_123","session_token":"cs_token_from_api","destination":"example.com:443"}
```

If accepted, the gateway returns:

```text
OK
```

After `OK`, traffic is fully relayed bidirectionally until close/timeout.

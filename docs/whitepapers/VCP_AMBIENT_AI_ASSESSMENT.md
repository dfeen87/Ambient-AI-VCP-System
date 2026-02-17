# Assessment: VCP + Ambient AI Whitepapers

## Scope
This assessment reviews:
- `docs/whitepapers/VCP.md`
- `docs/whitepapers/AMBIENT_AI.md`

Primary question assessed: **Can a node be used to provide internet access?**

## Executive Answer
**Yes. I agree with your direction**: introduce a dedicated internet-access node class and keep compute workers isolated.

- **Compute nodes (worker/sandbox nodes):** keep `no network egress` as default.
- **Open Internet nodes (new category):** provide policy-governed internet relay only.
- **Connection-only tasks:** use a minimal task type that requests connectivity only (no arbitrary task description payload).

This is a good security move because it reduces free-form instructions and narrows the attack surface.

## Proposed security model update

## 1) New node category
Add a distinct node role:
- `open_internet` — relay/egress access only, not arbitrary compute execution.

Keep existing roles separate:
- `compute.worker` — sandboxed computation, no open internet.
- `cache.edge` (optional) — cached content/IPFS delivery.

## 2) New minimal task type
Add a strict task type:
- `connect_only`

### `connect_only` design principle
No human task description field. No executable payload. No user code.

Allowed fields should be minimal and policy-verifiable, for example:
- `task_type`: `connect_only`
- `session_id`
- `requester_id`
- `duration_seconds` (bounded)
- `bandwidth_limit_mbps` (bounded)
- `egress_profile` (enum only)
- `destination_policy_id` (points to a predefined allowlist policy)

This is safer than free-form task descriptions because policy is machine-validated.

## 3) Required guardrails for `open_internet`
- identity/authentication required for requester and node operator,
- per-session quotas (time, bandwidth, connection count),
- destination allow/deny policy enforcement,
- protocol limits (e.g., HTTPS/DNS only in early phases),
- abuse logging, anomaly detection, and immediate revocation,
- legal/compliance controls by jurisdiction.

## 4) Why this is more secure
Your idea improves security **if implemented with strict policy fields**:
- no arbitrary compute payload,
- no free-text instruction ambiguity,
- easier auditing and deterministic enforcement,
- clear separation between internet relay and compute execution.

Important caveat: "no task description" alone is not enough; security comes from replacing it with a **validated schema + enforced policy IDs**.

## Suggested spec snippet
```json
{
  "task_type": "connect_only",
  "session_id": "sess_123",
  "requester_id": "user_abc",
  "duration_seconds": 300,
  "bandwidth_limit_mbps": 20,
  "egress_profile": "allowlist_domains",
  "destination_policy_id": "policy_web_basic_v1"
}
```

## Conclusion
I think your proposal is strong and should be adopted:
1. create `open_internet` node category,
2. add `connect_only` task type with no free-form description,
3. enforce strict schema + policy + quotas.

That keeps your VCP compute sandbox model intact while enabling controlled internet access through a separate trust boundary.

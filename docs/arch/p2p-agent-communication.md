# P2P Agent Communication

**Status**: Parked — high-level design for future implementation

## Concept

An AI agent runs as a Nexus plugin. A host extension (`p2p-comm`) provides
peer-to-peer communication between Nexus instances, allowing agents to
collaborate across machines with bilateral consent from both users.

## Components

### AI Agent Plugin
- Docker container like any other plugin
- Uses Host API for system access (filesystem, process, network)
- Calls LLM API (Anthropic, OpenAI, local) for reasoning
- Exposes MCP tools so Claude Code / other clients can interact
- Communicates with peers via the `p2p-comm` extension

### P2P Communication Extension
- Native host extension (process-based, JSON-RPC over stdin/stdout)
- Handles networking, encryption, and message signing
- Manages peer connections and message routing

## Identity

Each Nexus instance gets an Ed25519 keypair (reuse existing crypto infra).

- **Keypair** generated on first run, stored in `~/.nexus/identity/`
- **Fingerprint** derived from public key (e.g. `a3:f2:b1:...`)
- **Display name** set by user, transmitted with pubkey
- **Trusted peers** stored in `~/.nexus/trusted_peers.json` (TOFU model)

## Discovery

Phase 1: Manual key exchange (paste pubkey + IP, like SSH).
Phase 2: Optional relay/rendezvous server for NAT traversal.
Phase 3: mDNS for LAN discovery (zero-config for same-network peers).

## Bilateral Consent

Both parties must approve before any data flows:

1. User A initiates: "Connect to peer `<pubkey-fingerprint>` at `<address>`"
2. User B receives connection request → runtime approval dialog:
   "Daniel wants to connect. Fingerprint: `a3:f2:...`. Accept?"
3. On accept → pubkey saved to trusted peers, connection established
4. Subsequent connections auto-approved (trusted peer), revocable anytime

## Message Signing

Every message between peers is signed (Ed25519) and verified. This is where
SigV4-style signing makes sense — messages traverse the network.

```
Message {
  sender_pubkey: [u8; 32],
  recipient_pubkey: [u8; 32],
  timestamp: u64,
  nonce: [u8; 16],
  payload: bytes,
  signature: [u8; 64],  // Ed25519 sign(sender_privkey, hash(all_above))
}
```

- Timestamp + nonce prevent replay attacks
- Recipient verifies sender signature before processing
- Messages encrypted with X25519 key exchange (derived from Ed25519 keys)

## Transport

Phase 1: Direct WebSocket connections (simple, works on LAN).
Phase 2: WebRTC for NAT traversal (true P2P, no relay needed).
Phase 3: Optional relay server for restrictive networks.

## Permission Model

Fits into existing three-layer security:

| Layer | Gate |
|-------|------|
| Permission | `ext:p2p-comm:send` / `ext:p2p-comm:receive` |
| Scope | Approved peer IDs (per-plugin, per-peer) |
| Risk | Message content review for sensitive operations |

Example runtime approval:
"Alice's AI agent wants to share a file listing with you via P2P. Allow?"

## Call Flow

```
AI Agent Plugin → POST /v1/extensions/p2p-comm/send
  → auth middleware (access token)
  → permission check (ext:p2p-comm:send)
  → scope check (is target peer approved?)
  → risk check (if high-risk → approval dialog)
  → p2p-comm extension signs + encrypts + sends
  → recipient p2p-comm extension verifies + decrypts
  → recipient Nexus runtime approval ("Peer wants to send X. Allow?")
  → delivered to recipient AI agent plugin
```

## Open Questions

- Message format: structured (JSON-RPC between agents?) vs freeform text
- Rate limiting on cross-peer messages
- Message size limits
- Persistent vs ephemeral connections
- Group communication (>2 peers)
- What capabilities should an AI agent plugin request by default?
- Should agents negotiate capabilities before collaborating?

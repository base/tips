# shadow-boost

A minimal proxy for driving a shadow builder (op-rbuilder) from a non-sequencer op-node.

## Purpose

Shadow-boost enables running a shadow builder in parallel with the canonical sequencer without causing reorgs or P2P block rejections. It sits between a non-sequencer op-node and a builder (op-rbuilder), enabling the builder to produce blocks in parallel with the canonical sequencer without interfering with L2 consensus.

## How It Works

1. **Intercepts `forkchoiceUpdated` calls**: Rewrites `no_tx_pool=true` to `no_tx_pool=false` to trigger block building
2. **Synthetically calls `getPayload`**: Fetches built blocks from the builder after a delay for analysis/logging
3. **Forwards `newPayload` calls**: Keeps the builder synced with the canonical chain via P2P blocks

The builder produces blocks in parallel with the canonical sequencer, but these blocks are never sent back to the op-node (getPayload is not supported). The op-node follows the canonical chain via P2P and L1 derivation normally, while the builder independently produces shadow blocks for comparison/analysis.

## Why Not Sequencer Mode?

Running a shadow builder with op-node in sequencer mode causes several issues:

### Sequencer Mode Problems

1. **P2P Block Rejection**: The shadow op-node builds its own blocks locally and considers them the "unsafe head". When the real sequencer's blocks arrive via P2P gossip, they are rejected with "skipping unsafe payload, since it is older than unsafe head" because they have the same block number but different hashes.

2. **Multiple L1-Triggered Reorgs**: When L1 blocks arrive containing batched L2 data, the derivation pipeline advances the "safe head" and re-derives L2 blocks from L1 data. If the locally-built blocks don't match the L1-derived attributes (random field, etc.), the shadow builder reorgs away from its own chain, discarding the queued P2P blocks in the process.

3. **Delayed Convergence**: The shadow builder may reorg the same block number multiple times as it receives:
   - Its own locally-built block
   - L1-derived blocks (from ancestor batch data)
   - The final canonical block (from the specific block's L1 batch data)

   This can take 10-20+ seconds per block to converge.

4. **Fork Persistence**: The shadow builder maintains a persistent fork from the canonical chain until L1 derivation eventually produces matching blocks.

### Shadow-Boost Solution

Shadow-boost solves these issues by:

1. **Non-Sequencer Mode**: The op-node runs as a follower, accepting canonical blocks via P2P immediately without rejection.

2. **Forced Block Building**: Intercepts `forkchoiceUpdated` calls and rewrites `no_tx_pool=true` to `no_tx_pool=false`, triggering the builder to construct blocks even though the op-node isn't sequencing.

3. **Parallel Building**: The builder produces blocks in parallel with the canonical sequencer, but these blocks are never sent back to the op-node.

4. **No Reorgs**: The op-node follows the canonical chain via P2P and L1 derivation normally, while the builder independently produces shadow blocks for comparison/analysis.

This allows the shadow builder to build blocks at the same pace as the real sequencer while staying synchronized with the canonical chain without constant reorgs.

## Usage

```bash
shadow-boost \
  --builder-url http://localhost:9551 \
  --builder-jwt-secret /path/to/jwt/secret \
  --listen-addr 127.0.0.1:8554
```

Then configure your op-node to use this proxy as its execution engine:

```bash
op-node \
  --l2.engine-rpc http://127.0.0.1:8554 \
  --l2.engine-jwt-secret /path/to/jwt/secret
```

## Environment Variables

- `BUILDER_URL`: Builder's Engine API URL
- `BUILDER_JWT_SECRET`: Path to builder's JWT secret file
- `LISTEN_ADDR`: Address to listen on (default: 127.0.0.1:8554)
- `TIMEOUT_MS`: Request timeout in milliseconds (default: 2000)

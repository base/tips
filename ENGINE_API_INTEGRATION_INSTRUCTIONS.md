# Instructions for Claude in builder-playground Repository

Copy this entire message and send it to Claude in the builder-playground repository:

---

## Task: Configure Sequencer to Use External TIPS Builder via Engine API

I need you to configure the op-geth sequencer to use an external TIPS builder for block building via the Engine API.

### Current Setup

**TIPS Builder (External)**:
- Running at `http://localhost:8561` (Engine API endpoint)
- JWT secret: `0x2053bbf613d005202d3215c33c6ead941b821fb85f695c8e87c5c5649e70974c`
- JWT file location: `/Users/williamlaw/src/opensource/tips/jwt.hex`
- HTTP RPC endpoint: `http://localhost:2222`
- Chain ID: 13
- Already configured with:
  - `--authrpc.addr=127.0.0.1`
  - `--authrpc.port=8561`
  - `--authrpc.jwtsecret=./jwt.hex`
  - `--rollup.sequencer-http=http://localhost:8547`

**Sequencer (Your Side)**:
- Currently at `http://localhost:8547`
- Needs to send Engine API calls to the TIPS builder instead of building blocks internally

### What Needs to Change

The sequencer must be configured to:

1. **Use external builder mode** - Send `engine_forkchoiceUpdatedV3` and `engine_getPayloadV3` calls to `http://localhost:8561`
2. **Use JWT authentication** - Authenticate with JWT secret `0x2053bbf613d005202d3215c33c6ead941b821fb85f695c8e87c5c5649e70974c`
3. **Not build blocks internally** - Delegate all block building to the external TIPS builder

### Expected Flow

```
Transaction → Sequencer (8547)
                ↓
     engine_forkchoiceUpdatedV3
                ↓
         TIPS Builder (8561)
                ↓
     Builds block with UserOps
                ↓
     engine_getPayloadV3
                ↓
         TIPS Builder (8561)
                ↓
      Returns payload to Sequencer
                ↓
     Sequencer proposes block
```

### Configuration Flags Needed

For op-geth sequencer, you likely need flags like:

```bash
--builder.remote_relay_url=http://localhost:8561
--builder.jwt_secret=0x2053bbf613d005202d3215c33c6ead941b821fb85f695c8e87c5c5649e70974c
```

OR for reth-based sequencer:

```bash
--builder.url=http://localhost:8561
--authrpc.jwtsecret=/path/to/jwt.hex
```

### Verification Steps

After making the changes:

1. **Start the sequencer** with the new configuration
2. **Check sequencer logs** - Should show Engine API calls being sent to `localhost:8561`
3. **Check TIPS builder logs** - Should show:
   ```
   INFO Received Engine API call method=engine_forkchoiceUpdatedV3
   INFO Received Engine API call method=engine_getPayloadV3
   ```
4. **Send a test transaction**:
   ```bash
   cast send 0x0000000000000000000000000000000000000000 \
     --value 0.01ether \
     --rpc-url http://localhost:8547 \
     --private-key 0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d
   ```
5. **Verify block was built by TIPS builder** - Check both logs confirm the flow

### Questions to Answer

1. What configuration file or script controls the sequencer startup?
2. What flags or environment variables need to be modified?
3. How do I verify the Engine API connection is working?
4. Are there any port conflicts to watch out for?

### Additional Context

- The TIPS builder is a custom Reth-based builder that inserts ERC-4337 UserOperation bundles at block midpoint
- It's already running and healthy (confirmed via logs)
- We've verified it can receive regular RPC calls on port 2222
- Now we need it to receive Engine API calls on port 8561 from the sequencer

Please show me:
1. What files you'll modify
2. What the configuration changes look like
3. How to verify it's working

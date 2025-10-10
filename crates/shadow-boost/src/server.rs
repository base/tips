use crate::proxy::ShadowBuilderProxy;
use alloy_primitives::B256;
use alloy_rpc_types_engine::PayloadId;
use jsonrpsee::{
    core::client::ClientT,
    server::Server,
    types::{ErrorObjectOwned, Params},
    RpcModule,
};
use serde_json::Value;
use tracing::{info, warn};

pub fn build_rpc_module(proxy: ShadowBuilderProxy) -> RpcModule<ShadowBuilderProxy> {
    let mut module = RpcModule::new(proxy);

    module
        .register_async_method(
            "engine_forkchoiceUpdatedV3",
            |params, context, _| async move {
                let (fork_choice_state, payload_attributes) = params.parse()?;
                context
                    .handle_fcu(fork_choice_state, payload_attributes)
                    .await
            },
        )
        .unwrap();

    module
        .register_async_method("engine_newPayloadV3", |params, context, _| async move {
            let (payload, versioned_hashes, parent_beacon_block_root) = params.parse()?;
            context
                .handle_new_payload(payload, versioned_hashes, parent_beacon_block_root)
                .await
        })
        .unwrap();

    module
        .register_async_method("engine_newPayloadV4", |params, context, _| async move {
            let (payload, versioned_hashes, parent_beacon_block_root, _blob_versioned_hashes): (_, _, _, Vec<B256>) = params.parse()?;
            context
                .handle_new_payload(payload, versioned_hashes, parent_beacon_block_root)
                .await
        })
        .unwrap();

    module
        .register_async_method("engine_getPayloadV3", |params, _context, _| async move {
            let (payload_id,): (PayloadId,) = params.parse()?;
            warn!(%payload_id, "op-node called getPayload unexpectedly (should never happen in non-sequencer mode)");
            Err::<(), _>(ErrorObjectOwned::owned(
                -32601,
                "getPayload not supported in shadow builder proxy",
                None::<()>,
            ))
        })
        .unwrap();

    add_passthrough_methods(&mut module);

    module
}

fn add_passthrough_methods(module: &mut RpcModule<ShadowBuilderProxy>) {
    let methods = [
        "eth_chainId",
        "eth_syncing",
        "eth_getBlockByNumber",
        "eth_getBlockByHash",
        "engine_exchangeCapabilities",
        "engine_forkchoiceUpdatedV1",
        "engine_forkchoiceUpdatedV2",
        "engine_forkchoiceUpdatedV4",
        "engine_newPayloadV1",
        "engine_newPayloadV2",
        "engine_getPayloadV1",
        "engine_getPayloadV2",
        "engine_getPayloadV4",
        "engine_newPayloadWithWitnessV4",
        "engine_getPayloadBodiesByHashV1",
        "engine_getPayloadBodiesByRangeV1",
    ];

    for method in methods {
        let method_name = method.to_string();
        module
            .register_async_method(method, move |params: Params<'static>, context, _| {
                let method = method_name.clone();
                async move {
                    let mut params_vec = Vec::new();
                    let mut seq = params.sequence();
                    while let Ok(Some(value)) = seq.optional_next::<Value>() {
                        params_vec.push(value);
                    }

                    info!(
                        method,
                        params_count = params_vec.len(),
                        "Proxying method to shadow builder"
                    );

                    let client = context.builder_client.read().await;
                    let result: Value = client.request(&method, params_vec).await.map_err(|e| {
                        warn!(method, error = %e, "Shadow builder method call failed");
                        ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
                    })?;

                    info!(method, "Shadow builder method call succeeded");
                    Ok::<Value, ErrorObjectOwned>(result)
                }
            })
            .unwrap();
    }
}

pub async fn start_server(
    listen_addr: &str,
    rpc_module: RpcModule<ShadowBuilderProxy>,
) -> eyre::Result<()> {
    let server = Server::builder().build(listen_addr).await?;
    let handle = server.start(rpc_module);

    tokio::signal::ctrl_c().await?;
    handle.stop()?;
    handle.stopped().await;

    Ok(())
}

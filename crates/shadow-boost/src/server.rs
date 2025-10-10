use crate::proxy::ShadowBuilderProxy;
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
            |params: Params<'static>, context, _| async move {
                let mut params_vec = Vec::new();
                let mut seq = params.sequence();
                while let Ok(Some(value)) = seq.optional_next::<Value>() {
                    params_vec.push(value);
                }
                context.handle_fcu(params_vec).await
            },
        )
        .unwrap();

    module
        .register_async_method(
            "engine_newPayloadV4",
            |params: Params<'static>, context, _| async move {
                let mut params_vec = Vec::new();
                let mut seq = params.sequence();
                while let Ok(Some(value)) = seq.optional_next::<Value>() {
                    params_vec.push(value);
                }
                context.handle_new_payload(params_vec).await
            },
        )
        .unwrap();

    let methods = [
        "eth_chainId",
        "eth_syncing",
        "eth_getBlockByNumber",
        "eth_getBlockByHash",
        "eth_sendRawTransaction",
        "eth_sendRawTransactionConditional",
        "miner_setExtra",
        "miner_setGasPrice",
        "miner_setGasLimit",
        "miner_setMaxDASize",
        "engine_exchangeCapabilities",
        "engine_forkchoiceUpdatedV1",
        "engine_forkchoiceUpdatedV2",
        "engine_forkchoiceUpdatedV4",
        "engine_newPayloadV1",
        "engine_newPayloadV2",
        "engine_newPayloadV3",
        "engine_getPayloadV1",
        "engine_getPayloadV2",
        "engine_getPayloadV3",
        "engine_getPayloadV4",
        "engine_newPayloadWithWitnessV4",
        "engine_getPayloadBodiesByHashV1",
        "engine_getPayloadBodiesByRangeV1",
    ];

    for method in methods {
        register_passthrough_method(&mut module, method);
    }

    module
}

fn register_passthrough_method(
    module: &mut RpcModule<ShadowBuilderProxy>,
    method_name: &'static str,
) {
    let method_owned = method_name.to_string();
    module
        .register_async_method(method_name, move |params: Params<'static>, context, _| {
            let method = method_owned.clone();
            async move {
                let mut params_vec = Vec::new();
                let mut seq = params.sequence();
                while let Ok(Some(value)) = seq.optional_next::<Value>() {
                    params_vec.push(value);
                }

                info!(
                    method,
                    params_count = params_vec.len(),
                    params = ?params_vec,
                    "JSON-RPC request"
                );

                let client = context.builder_client.read().await;
                let result: Value = client.request(&method, params_vec).await.map_err(|e| {
                    warn!(method, error = %e, "JSON-RPC request failed");
                    ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>)
                })?;

                info!(method, response = ?result, "JSON-RPC response");
                Ok::<Value, ErrorObjectOwned>(result)
            }
        })
        .unwrap();
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

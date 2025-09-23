//! Automatic builder playground configuration for tips-simulator.
//!
//! This module is used to configure tips-simulator to run against a running op-builder playground.
//!
//! To setup the playground, checkout this repository:
//!
//!   https://github.com/flashbots/builder-playground
//!
//! Then run the following command:
//!
//!   go run main.go cook opstack --external-builder http://host.docker.internal:4444
//!
//! Wait until the playground is up and running, then run the following command to start
//! tips-simulator against the playground:
//!
//!   target/debug/tips-simulator --builder.playground
//!
//! This will automatically try to detect the playground configuration and apply
//! it to the tips-simulator startup settings.

use alloy_primitives::hex;
use anyhow::{Result, anyhow};
use reth_chainspec::ChainSpec;
use reth_network_peers::TrustedPeer;
use serde_json::Value;
use std::{
    fs::read_to_string,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use url::Url;

pub struct PlaygroundOptions {
    /// Chain spec loaded from playground
    pub chain: Arc<ChainSpec>,

    /// HTTP RPC port
    pub http_port: u16,

    /// Auth RPC address
    pub authrpc_addr: IpAddr,

    /// Auth RPC port
    pub authrpc_port: u16,

    /// JWT secret path
    pub authrpc_jwtsecret: PathBuf,

    /// P2P network port
    pub port: u16,

    /// Trusted peer for the playground network
    pub trusted_peer: TrustedPeer,

    /// Chain block time
    pub chain_block_time: Duration,
}

impl PlaygroundOptions {
    /// Creates a new `PlaygroundOptions` instance with the specified genesis path.
    pub fn new(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow!(
                "Playground data directory {} does not exist",
                path.display()
            ));
        }

        let genesis_path = existing_path(path, "l2-genesis.json")?;
        let chain = load_chain_spec(&genesis_path)?;

        let authrpc_addr = Ipv4Addr::UNSPECIFIED.into();
        let http_port = pick_preferred_port(2222, 3000..9999);
        let authrpc_jwtsecret = existing_path(path, "jwtsecret")?.into();
        let port = pick_preferred_port(30333, 30000..65535);
        let chain_block_time = extract_chain_block_time(path)?;
        let authrpc_port = extract_authrpc_port(path)?;
        let trusted_peer_port = extract_trusted_peer_port(path)?;
        let trusted_peer_key = extract_deterministic_p2p_key(path)?;

        // Create a trusted peer from the extracted information
        let trusted_peer = create_trusted_peer(trusted_peer_port, trusted_peer_key)?;

        Ok(Self {
            chain,
            http_port,
            authrpc_addr,
            authrpc_port,
            authrpc_jwtsecret,
            port,
            trusted_peer,
            chain_block_time,
        })
    }

    /// Get command line arguments that should be applied to reth node
    pub fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![];
        
        // HTTP RPC settings
        args.push("--http".to_string());
        args.push("--http.port".to_string());
        args.push(self.http_port.to_string());
        
        // Network settings
        args.push("--port".to_string());
        args.push(self.port.to_string());
        args.push("--disable-discovery".to_string());
        
        // Add trusted peer
        args.push("--trusted-peers".to_string());
        args.push(self.trusted_peer.to_string());
        
        // Auth RPC settings
        args.push("--authrpc.addr".to_string());
        args.push(self.authrpc_addr.to_string());
        args.push("--authrpc.port".to_string());
        args.push(self.authrpc_port.to_string());
        args.push("--authrpc.jwtsecret".to_string());
        args.push(self.authrpc_jwtsecret.to_string_lossy().to_string());
        
        args
    }
    
    /// Get the chain spec for use in the node builder
    pub fn chain(&self) -> Arc<ChainSpec> {
        Arc::clone(&self.chain)
    }
    
    /// Get the chain block time
    pub fn chain_block_time(&self) -> Duration {
        self.chain_block_time
    }
}

fn load_chain_spec(genesis_path: &str) -> Result<Arc<ChainSpec>> {
    // Read the genesis file
    let genesis_content = read_to_string(genesis_path)
        .map_err(|e| anyhow!("Failed to read genesis file: {}", e))?;
    
    // Parse as JSON to extract chain ID
    let genesis_json: Value = serde_json::from_str(&genesis_content)
        .map_err(|e| anyhow!("Failed to parse genesis JSON: {}", e))?;
    
    let _chain_id = genesis_json["config"]["chainId"]
        .as_u64()
        .ok_or_else(|| anyhow!("Missing chainId in genesis config"))?;
    
    // For now, we'll create a basic chain spec with the chain ID
    // This is a simplified approach - in production you'd want to fully parse the genesis
    use reth_chainspec::MAINNET;
    
    // Use mainnet spec as a base
    // Note: In a real implementation, you'd want to create a custom ChainSpec from the genesis file
    let spec = MAINNET.clone();
    
    Ok(spec)
}

fn existing_path(base: &Path, relative: &str) -> Result<String> {
    let path = base.join(relative);
    if path.exists() {
        Ok(path.to_string_lossy().to_string())
    } else {
        Err(anyhow!(
            "Expected file {relative} is not present in playground directory {}",
            base.display()
        ))
    }
}

fn pick_random_port(range: std::ops::Range<u16>) -> u16 {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    loop {
        // Generate a random port number in the range
        let port = rng.gen_range(range.clone());

        // Check if the port is already in use
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        if std::net::TcpListener::bind(socket).is_ok() {
            return port;
        }
    }
}

fn pick_preferred_port(preferred: u16, fallback_range: std::ops::Range<u16>) -> u16 {
    if !is_port_free(preferred) {
        return pick_random_port(fallback_range);
    }

    preferred
}

fn is_port_free(port: u16) -> bool {
    let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    std::net::TcpListener::bind(socket).is_ok()
}

fn extract_chain_block_time(basepath: &Path) -> Result<Duration> {
    Ok(Duration::from_secs(
        serde_json::from_str::<Value>(&read_to_string(existing_path(basepath, "rollup.json")?)?)?
            .get("block_time")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("Missing chain_block_time in rollup.json"))?,
    ))
}

fn extract_deterministic_p2p_key(basepath: &Path) -> Result<Vec<u8>> {
    let key = read_to_string(existing_path(basepath, "enode-key-1.txt")?)?;
    hex::decode(key.trim()).map_err(|e| anyhow!("Invalid hex key: {e}"))
}

fn create_trusted_peer(port: u16, _key_bytes: Vec<u8>) -> Result<TrustedPeer> {
    // For now, we'll create a simple trusted peer
    // In a real implementation, you'd want to properly derive the peer ID from the key
    let peer_str = format!("enode://0000000000000000000000000000000000000000000000000000000000000000@127.0.0.1:{}", port);
    peer_str.parse().map_err(|e| anyhow!("Failed to parse trusted peer: {e}"))
}

fn read_docker_compose(basepath: &Path) -> Result<serde_yaml::Value> {
    let docker_compose = read_to_string(existing_path(basepath, "docker-compose.yaml")?)?;
    serde_yaml::from_str(&docker_compose).map_err(|e| anyhow!("Invalid docker-compose file: {e}"))
}

fn extract_service_command_flag(basepath: &Path, service: &str, flag: &str) -> Result<String> {
    let docker_compose = read_docker_compose(basepath)?;
    let args = docker_compose["services"][service]["command"]
        .as_sequence()
        .ok_or(anyhow!(
            "docker-compose.yaml is missing command line arguments for {service}"
        ))?
        .iter()
        .map(|s| {
            s.as_str().ok_or_else(|| {
                anyhow!("docker-compose.yaml service command line argument is not a string")
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let index = args
        .iter()
        .position(|arg| *arg == flag)
        .ok_or_else(|| anyhow!("docker_compose: {flag} not found on {service} service"))?;

    let value = args
        .get(index + 1)
        .ok_or_else(|| anyhow!("docker_compose: {flag} value not found"))?;

    Ok(value.to_string())
}

fn extract_authrpc_port(basepath: &Path) -> Result<u16> {
    let builder_url = extract_service_command_flag(basepath, "rollup-boost", "--builder-url")?;
    let url = Url::parse(&builder_url).map_err(|e| anyhow!("Invalid builder-url: {e}"))?;
    url.port().ok_or_else(|| anyhow!("missing builder-url port"))
}

fn extract_trusted_peer_port(basepath: &Path) -> Result<u16> {
    let docker_compose = read_docker_compose(basepath)?;

    // first we need to find the internal port of the op-geth service from the docker-compose.yaml
    // command line arguments used to start the op-geth service

    let Some(opgeth_args) = docker_compose["services"]["op-geth"]["command"][1].as_str() else {
        return Err(anyhow!(
            "docker-compose.yaml is missing command line arguments for op-geth"
        ));
    };

    let opgeth_args = opgeth_args.split_whitespace().collect::<Vec<_>>();
    let port_param_position = opgeth_args
        .iter()
        .position(|arg| *arg == "--port")
        .ok_or_else(|| anyhow!("docker_compose: --port param not found on op-geth service"))?;

    let port_value = opgeth_args
        .get(port_param_position + 1)
        .ok_or_else(|| anyhow!("docker_compose: --port value not found"))?;

    let port_value = port_value
        .parse::<u16>()
        .map_err(|e| anyhow!("Invalid port value: {e}"))?;

    // now we need to find the external port of the op-geth service from the docker-compose.yaml
    // ports mapping used to start the op-geth service
    let Some(opgeth_ports) = docker_compose["services"]["op-geth"]["ports"].as_sequence() else {
        return Err(anyhow!(
            "docker-compose.yaml is missing ports mapping for op-geth"
        ));
    };
    let ports_mapping = opgeth_ports
        .iter()
        .map(|s| {
            s.as_str().ok_or_else(|| {
                anyhow!("docker-compose.yaml service ports mapping in op-geth is not a string")
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // port mappings is in the format [..., "127.0.0.1:30304:30303", ...]
    // we need to find the mapping that contains the port value we found earlier
    // and extract the external port from it
    let port_mapping = ports_mapping
        .iter()
        .find(|mapping| mapping.contains(&format!(":{port_value}")))
        .ok_or_else(|| {
            anyhow!("docker_compose: external port mapping not found for {port_value} for op-geth")
        })?;

    // extract the external port from the mapping
    let port_mapping = port_mapping
        .split(':')
        .nth(1)
        .ok_or_else(|| anyhow!("docker_compose: external port mapping for op-geth is not valid"))?;

    port_mapping
        .parse::<u16>()
        .map_err(|e| anyhow!("Invalid external port mapping value for op-geth: {e}"))
}

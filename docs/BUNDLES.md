// Limits: 25m gas total
pub txs: Vec<Bytes>,
// Can only be within next 24 hours
pub block_number: u64,
pub min_timestamp: Option<u64>,
pub max_timestamp: Option<u64>,
// If set, will only update, if not set will insert
pub replacement_uuid: Option<String>,

/*** Not supported: ***/
/// Must be set to all txn hashes -- could potentially allow this to have txn hashes
pub reverting_tx_hashes: Vec<TxHash>,
// Must be empty -- let us update a bundle when txn is included, or just ignore it on procesing
pub dropping_tx_hashes: Vec<TxHash>,
// Must be none
pub refund_percent: Option<u8>,
// Must be none
pub refund_recipient: Option<Address>,
// Must be none
pub refund_tx_hashes: Vec<TxHash>,
// Must be empty
pub extra_fields: OtherFields,
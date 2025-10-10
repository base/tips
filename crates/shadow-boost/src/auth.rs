use alloy_rpc_types_engine::JwtSecret;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn generate_jwt_token(secret: &JwtSecret) -> String {
    use base64::Engine;
    use hmac::Mac;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let header = r#"{"alg":"HS256","typ":"JWT"}"#;
    let payload = format!(r#"{{"iat":{}}}"#, now);

    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);

    let message = format!("{}.{}", header_b64, payload_b64);

    let signature = {
        use sha2::Sha256;
        let mut mac =
            hmac::Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC creation failed");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(result.into_bytes())
    };

    format!("{}.{}", message, signature)
}

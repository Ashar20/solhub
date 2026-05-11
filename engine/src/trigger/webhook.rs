//! Webhook trigger.
//!
//! The webhook trigger is fully handled by the API layer: when
//! `POST /v1/webhooks/:workflow_id` is received, the API validates the
//! HMAC-SHA256 `X-Hub-Signature` header and calls `db.create_run()` directly.
//! The engine never sees raw HTTP — it only processes the resulting `Pending`
//! run through the `Scheduler`.

pub struct WebhookTrigger;

/// Validate an `X-Hub-Signature` header against `payload` and `secret`.
///
/// Returns `true` if the HMAC-SHA256 digest matches.
pub fn verify_signature(secret: &[u8], payload: &[u8], signature_hex: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let Ok(expected_bytes) = hex::decode(signature_hex.trim_start_matches("sha256=")) else {
        return false;
    };

    let mut mac = Hmac::<Sha256>::new_from_slice(secret)
        .expect("HMAC accepts any key length");
    mac.update(payload);
    let result = mac.finalize().into_bytes();

    result.as_slice() == expected_bytes.as_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    fn make_sig(secret: &[u8], payload: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(payload);
        let bytes = mac.finalize().into_bytes();
        format!("sha256={}", hex::encode(bytes))
    }

    #[test]
    fn valid_signature_passes() {
        let secret = b"my-secret";
        let payload = b"hello world";
        let sig = make_sig(secret, payload);
        assert!(verify_signature(secret, payload, &sig));
    }

    #[test]
    fn invalid_signature_fails() {
        assert!(!verify_signature(b"secret", b"payload", "sha256=deadbeef"));
    }
}

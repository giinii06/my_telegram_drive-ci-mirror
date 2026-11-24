use tdrive_crypto::{VaultEnvelopeError, unwrap_master_key, verify_key_check, wrap_master_key, encode_key_check};

#[test]
fn master_key_and_check_roundtrip_and_reject_tampering() {
    let master = [11; 32];
    let kek = [22; 32];
    let mut wrapped = wrap_master_key(&master, &kek).unwrap();
    assert_eq!(wrapped.len(), 72);
    assert_eq!(unwrap_master_key(&wrapped, &kek).unwrap(), master);
    assert_eq!(unwrap_master_key(&wrapped, &[33; 32]), Err(VaultEnvelopeError::AuthenticationFailed));
    let check = encode_key_check(&master).unwrap();
    assert_eq!(check.len(), 59);
    assert!(verify_key_check(&master, &check).is_ok());
    assert_eq!(verify_key_check(&[33; 32], &check), Err(VaultEnvelopeError::AuthenticationFailed));
    wrapped[30] ^= 1;
    assert_eq!(unwrap_master_key(&wrapped, &kek), Err(VaultEnvelopeError::AuthenticationFailed));
}

#[test]
fn malformed_envelopes_rejected_before_decryption() {
    assert_eq!(unwrap_master_key(&[0; 71], &[0; 32]), Err(VaultEnvelopeError::MalformedEnvelope));
    assert_eq!(verify_key_check(&[0; 32], &[0; 60]), Err(VaultEnvelopeError::MalformedEnvelope));
}

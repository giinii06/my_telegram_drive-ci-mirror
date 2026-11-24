use tdrive_crypto::{VaultSession, encode_key_check, wrap_master_key};

#[test]
fn locked_sessions_deny_names_and_wrong_unlock_revokes_access() {
    let master = [7; 32];
    let kek = [8; 32];
    let wrapped = wrap_master_key(&master, &kek).unwrap();
    let check = encode_key_check(&master).unwrap();
    let mut session = VaultSession::new();
    assert!(!session.is_unlocked());
    assert!(session.encrypt_name("hello").is_err());
    session.unlock(&wrapped, &check, &kek).unwrap();
    assert!(session.is_unlocked());
    let sealed = session.encrypt_name("한글 + file").unwrap();
    assert_eq!(session.decrypt_name(&sealed).unwrap(), "한글 + file");
    assert!(session.unlock(&wrapped, &check, &[9; 32]).is_err());
    assert!(!session.is_unlocked());
    assert!(session.decrypt_name(&sealed).is_err());
    session.unlock(&wrapped, &check, &kek).unwrap();
    session.lock();
    assert!(session.decrypt_name(&sealed).is_err());
}

#[test]
fn inconsistent_key_check_never_unlocks_or_crosses_sessions() {
    let master = [1; 32];
    let kek = [2; 32];
    let wrapped = wrap_master_key(&master, &kek).unwrap();
    let check = encode_key_check(&master).unwrap();
    let wrong = encode_key_check(&[3; 32]).unwrap();
    let mut a = VaultSession::new();
    let mut b = VaultSession::new();
    assert!(a.unlock(&wrapped, &wrong, &kek).is_err());
    assert!(!a.is_unlocked());
    a.unlock(&wrapped, &check, &kek).unwrap();
    assert!(b.decrypt_name(&a.encrypt_name("name").unwrap()).is_err());
    assert!(b.unlock(&wrapped[..71], &check, &kek).is_err());
    assert!(!b.is_unlocked());
}

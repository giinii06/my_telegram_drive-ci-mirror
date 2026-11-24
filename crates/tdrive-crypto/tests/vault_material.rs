use tdrive_crypto::{VaultKdfParams, VaultMaterialError, validate_vault_material};

#[test]
fn go_default_and_legacy_empty_kdf_are_accepted() {
    let params = VaultKdfParams::default();
    assert_eq!(params.memory_kib, 65536);
    assert_eq!(
        validate_vault_material(&params, "", &[0; 16], &[0; 72], &[0; 59]),
        Ok(())
    );
    assert_eq!(
        validate_vault_material(&params, "argon2id", &[0; 16], &[0; 72], &[0; 59]),
        Ok(())
    );
}

#[test]
fn untrusted_cost_and_material_lengths_are_rejected() {
    let defaults = VaultKdfParams::default();
    for params in [
        VaultKdfParams {
            memory_kib: 262145,
            ..defaults
        },
        VaultKdfParams {
            memory_kib: 16383,
            ..defaults
        },
        VaultKdfParams {
            iterations: 11,
            ..defaults
        },
        VaultKdfParams {
            lanes: 0,
            ..defaults
        },
        VaultKdfParams {
            key_len: 16,
            ..defaults
        },
        VaultKdfParams {
            salt_len: 65,
            ..defaults
        },
    ] {
        assert_eq!(
        validate_vault_material(&params, "argon2id", &[0; 16], &[0; 72], &[0; 59]),
        Err(VaultMaterialError::InvalidParams)
    );
    }
    assert_eq!(
        validate_vault_material(&defaults, "scrypt", &[0; 16], &[0; 72], &[0; 59]),
        Err(VaultMaterialError::UnsupportedKdf)
    );
    assert!(validate_vault_material(&defaults, "", &[0; 15], &[0; 72], &[0; 59]).is_err());
    assert!(validate_vault_material(&defaults, "", &[0; 16], &[0; 71], &[0; 59]).is_err());
    assert!(validate_vault_material(&defaults, "", &[0; 16], &[0; 72], &[0; 58]).is_err());
}

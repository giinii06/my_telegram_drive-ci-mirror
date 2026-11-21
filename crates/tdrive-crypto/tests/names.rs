use tdrive_crypto::{
    NameCryptoError, NAME_KEY_VERSION, decrypt_file_name, encrypt_file_name,
};

const KEY: [u8; 32] = [11; 32];

#[test]
fn go_envelope_roundtrips_unicode_and_boundaries() {
    assert_eq!(NAME_KEY_VERSION, 1);
    for name in ["movie.mkv", "한글 파일 이름.mp4", "a", "사진🦋.jpg"] {
        let sealed = encrypt_file_name(&KEY, name).unwrap();
        assert!(sealed.starts_with("v1:"));
        assert_eq!(decrypt_file_name(&KEY, &sealed).unwrap(), name);
    }
    let max_name = "x".repeat(4096);
    let sealed = encrypt_file_name(&KEY, &max_name).unwrap();
    assert_eq!(decrypt_file_name(&KEY, &sealed).unwrap(), max_name);
}

#[test]
fn randomly_sealed_same_name_never_repeats() {
    let first = encrypt_file_name(&KEY, "same.mp4").unwrap();
    let second = encrypt_file_name(&KEY, "same.mp4").unwrap();
    assert_ne!(first, second);
    assert_eq!(decrypt_file_name(&KEY, &first).unwrap(), "same.mp4");
    assert_eq!(decrypt_file_name(&KEY, &second).unwrap(), "same.mp4");
}

#[test]
fn filename_and_master_key_input_are_bounded() {
    assert!(matches!(
        encrypt_file_name(&KEY, ""),
        Err(NameCryptoError::InvalidName)
    ));
    assert!(matches!(
        encrypt_file_name(&KEY, &"x".repeat(4097)),
        Err(NameCryptoError::InvalidName)
    ));
    assert!(matches!(
        encrypt_file_name(&[0; 31], "abc"),
        Err(NameCryptoError::InvalidKey)
    ));
    // Maximum name bytes, not Unicode scalar count.
    assert!(matches!(
        encrypt_file_name(&KEY, &"🦋".repeat(1200)),
        Err(NameCryptoError::InvalidName)
    ));
}

#[test]
fn wrong_key_bad_version_malformed_base64_and_tamper_are_rejected() {
    let sealed = encrypt_file_name(&KEY, "movie.mkv").unwrap();
    assert!(matches!(
        decrypt_file_name(&[0x77; 32], &sealed),
        Err(NameCryptoError::AuthenticationFailed)
    ));
    assert!(matches!(
        decrypt_file_name(&KEY, &sealed.replace("v1:", "v9:")),
        Err(NameCryptoError::UnsupportedVersion)
    ));
    for bad in ["v1:", "v1:AAAA==", "v1:!!!!!", "plain.png", "v1:A"] {
        assert!(decrypt_file_name(&KEY, bad).is_err(), "{bad}");
    }
    let mut modified = sealed.into_bytes();
    let n = modified.len();
    modified[n / 2] = if modified[n / 2] == b'A' { b'B' } else { b'A' };
    assert!(decrypt_file_name(&KEY, std::str::from_utf8(&modified).unwrap()).is_err());
}

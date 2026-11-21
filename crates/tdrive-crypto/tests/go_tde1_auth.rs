use std::{env, fs, path::PathBuf};
use tdrive_crypto::{DecryptError, decrypt_small_tde1};

const SIZES: &[usize] = &[0, 1, 65_535, 65_536, 65_537, 196_625];

fn expected_plaintext(size: usize) -> Vec<u8> {
    (0..size).map(|i| ((i * 31 + 7) % 251) as u8).collect()
}

#[test]
fn go_tde1_authenticated_decrypt_boundary_vectors() {
    let Some(root) = env::var_os("TDRIVE_GO_TDE1_FIXTURES_DIR") else {
        eprintln!("Go encryption fixture unavailable: set TDRIVE_GO_TDE1_FIXTURES_DIR");
        return;
    };
    let root = PathBuf::from(root);
    let master = [0x42_u8; 32];
    for &size in SIZES {
        let ciphertext = fs::read(root.join(format!("tde1-{size}.bin")))
            .expect("read Go-encrypted TDE1 fixture");
        let plain = decrypt_small_tde1(&ciphertext, &master)
            .unwrap_or_else(|error| panic!("Rust failed Go TDE1 size={size}: {error}"));
        assert_eq!(plain, expected_plaintext(size), "Go/Rust mismatch: size={size}");

        let mut tampered = ciphertext.clone();
        tampered[50] ^= 1; // First authenticated ciphertext/tag, never the header.
        assert_eq!(
            decrypt_small_tde1(&tampered, &master),
            Err(DecryptError::AuthenticationFailed),
            "tamper size={size}"
        );
        assert_eq!(
            decrypt_small_tde1(&ciphertext, &[0x43_u8; 32]),
            Err(DecryptError::AuthenticationFailed),
            "wrong key size={size}"
        );
        assert_ne!(
            decrypt_small_tde1(&ciphertext[..ciphertext.len()-1], &master),
            Ok(expected_plaintext(size)),
            "truncated size={size}"
        );
        let mut extended = ciphertext.clone();
        extended.push(0);
        assert!(decrypt_small_tde1(&extended, &master).is_err(), "extra bytes size={size}");
    }
}

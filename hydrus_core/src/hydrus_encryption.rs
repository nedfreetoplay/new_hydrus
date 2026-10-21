//! Rust port of `hydrus/core/HydrusEncryption.py`.
//!
//! Python shells out to the `cryptography` package to build a self-signed
//! localhost certificate. The Rust port uses [`rcgen`] instead, which generates
//! the same kind of artefact (a PEM certificate plus a PEM private key) without
//! an OpenSSL installation.
//!
//! Two differences are documented rather than hidden:
//!
//! * Python generates an RSA-2048 key; `rcgen`'s `ring` backend cannot generate
//!   RSA keys, so the port generates an ECDSA P-256 key. Both are accepted by
//!   the TLS servers hydrus talks to.
//! * Python writes the key in traditional OpenSSL (`RSA PRIVATE KEY`) format;
//!   `rcgen` writes PKCS#8 (`PRIVATE KEY`), which is what modern OpenSSL and
//!   Python's `ssl` module expect.
//!
//! As in Python, both files are made read-only after they are written.

use std::fs;
use std::path::Path;

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use time::{Duration, OffsetDateTime};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// Python's `CRYPTO_OK`. `rcgen` is always compiled in.
pub const CRYPTO_OK: bool = true;

/// Python's `OPENSSL_OK`. The port never uses pyOpenSSL.
pub const OPENSSL_OK: bool = false;

fn crypto_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Unknown, message)
}

fn io_error(action: &str, path: &Path, error: std::io::Error) -> HydrusException {
    crypto_error(format!(
        "Could not {action} \"{}\": {error}",
        path.display()
    ))
}

fn random_hex(bytes: usize) -> String {
    use rand::RngCore;

    let mut buffer = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buffer);

    let mut hex = String::with_capacity(bytes * 2);

    for byte in buffer {
        use std::fmt::Write;

        let _ = write!(hex, "{byte:02x}");
    }

    hex
}

fn make_read_only(path: &Path) -> HydrusResult<()> {
    let metadata = fs::metadata(path).map_err(|error| io_error("inspect", path, error))?;
    let mut permissions = metadata.permissions();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        permissions.set_mode(0o444);
    }

    #[cfg(not(unix))]
    {
        permissions.set_readonly(true);
    }

    fs::set_permissions(path, permissions).map_err(|error| io_error("protect", path, error))
}

/// `generate_open_ssl_cert_and_key_file( cert_path, key_path )`
///
/// Writes a self-signed certificate valid for `localhost` to `cert_path` and
/// its private key to `key_path`, then marks both files read-only.
pub fn generate_open_ssl_cert_and_key_file(
    cert_path: &Path,
    key_path: &Path,
) -> HydrusResult<()> {
    if !CRYPTO_OK {
        return Err(crypto_error(
            "Sorry, to make your own cert, you need the \"cryptography\" library! You should be able to get it with pip.",
        ));
    }

    let mut params = CertificateParams::new(vec!["localhost".to_string()])
        .map_err(|error| crypto_error(format!("Could not build certificate parameters: {error}")))?;

    let now = OffsetDateTime::now_utc();

    params.not_before = now;
    params.not_after = now + Duration::days(365 * 10);

    let mut distinguished_name = DistinguishedName::new();

    distinguished_name.push(DnType::CountryName, "HN");
    distinguished_name.push(DnType::OrganizationName, "hydrus network");
    distinguished_name.push(DnType::OrganizationalUnitName, random_hex(32));

    params.distinguished_name = distinguished_name;

    let key = KeyPair::generate()
        .map_err(|error| crypto_error(format!("Could not generate a private key: {error}")))?;

    let certificate = params
        .self_signed(&key)
        .map_err(|error| crypto_error(format!("Could not sign the certificate: {error}")))?;

    fs::write(cert_path, certificate.pem())
        .map_err(|error| io_error("write the certificate to", cert_path, error))?;

    make_read_only(cert_path)?;

    fs::write(key_path, key.serialize_pem())
        .map_err(|error| io_error("write the private key to", key_path, error))?;

    make_read_only(key_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_writable(path: &Path) {
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                permissions.set_mode(0o644);
            }

            #[cfg(not(unix))]
            {
                permissions.set_readonly(false);
            }

            let _ = fs::set_permissions(path, permissions);
        }
    }

    #[test]
    fn generates_a_read_only_certificate_and_key() {
        let temp_dir =
            crate::hydrus_temp::get_sub_temp_dir("encryption_test").expect("temp dir");

        let cert_path = temp_dir.join("hydrus.crt");
        let key_path = temp_dir.join("hydrus.key");

        generate_open_ssl_cert_and_key_file(&cert_path, &key_path).expect("certificate");

        let certificate = fs::read_to_string(&cert_path).expect("read certificate");
        let key = fs::read_to_string(&key_path).expect("read key");

        assert!(certificate.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(certificate.trim_end().ends_with("-----END CERTIFICATE-----"));
        assert!(key.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(key.trim_end().ends_with("-----END PRIVATE KEY-----"));

        assert!(fs::metadata(&cert_path).expect("cert metadata").permissions().readonly());
        assert!(fs::metadata(&key_path).expect("key metadata").permissions().readonly());

        // Undo the protection so Windows can delete the directory.
        make_writable(&cert_path);
        make_writable(&key_path);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn unwritable_paths_are_reported() {
        let missing_dir = crate::hydrus_temp::get_hydrus_temp_dir().join("no_such_dir_for_encryption");

        let result = generate_open_ssl_cert_and_key_file(
            &missing_dir.join("hydrus.crt"),
            &missing_dir.join("hydrus.key"),
        );

        assert!(result.is_err());
    }
}

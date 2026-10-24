//! A port of `hydrus.core.networking.HydrusServerContextFactory`.
//!
//! Python builds a Twisted/OpenSSL `CertificateOptions`, which also loads only
//! the first certificate from a full-chain PEM file. The important Hydrus logic
//! is the chain ordering in [`load_certs_ordered`]: the leaf must be first and
//! intermediates follow in issuer order, with the self-signed root omitted.
//!
//! The Rust port keeps that logic and returns [`TlsContextSettings`] for a TLS
//! integration layer (for example rustls) to turn into a real server context.
//! This crate does not pretend to construct a Twisted context factory.
//!
//! The Python function also checks `HG.twisted_is_broke`; Rust has no Twisted,
//! so that check has no equivalent here.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use x509_parser::pem::parse_x509_pem;
use x509_parser::prelude::X509Certificate;

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// The minimum protocol version requested by the Python code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsVersion {
    /// `twisted.internet.ssl.TLSVersion.TLSv1_2`
    Tls1_2,
}

impl TlsVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            TlsVersion::Tls1_2 => "TLSv1.2",
        }
    }
}

/// One certificate in the ordered chain.
///
/// `der` is the parsed certificate body; `pem` is the PEM block that produced
/// it, ready to be concatenated into a full-chain file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderedCertificate {
    pub der: Vec<u8>,
    pub pem: Vec<u8>,
}

/// The inputs a TLS server needs, with the certificate chain already ordered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsContextSettings {
    /// Leaf first, then intermediates. The self-signed root is omitted when it
    /// is present in the input.
    pub certificates: Vec<OrderedCertificate>,
    pub private_key_pem: Vec<u8>,
    pub minimum_tls_version: TlsVersion,
}

impl TlsContextSettings {
    /// Concatenate the ordered PEM blocks (leaf first) into one full-chain PEM.
    pub fn certificate_chain_pem(&self) -> Vec<u8> {
        self.certificates
            .iter()
            .flat_map(|certificate| certificate.pem.iter().copied())
            .collect()
    }
}

fn certificate_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Unknown, message)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

const BEGIN_CERTIFICATE: &[u8] = b"-----BEGIN CERTIFICATE-----";
const END_CERTIFICATE: &[u8] = b"-----END CERTIFICATE-----";

/// Split a PEM bundle exactly the way the Python code does, then parse each
/// block.
fn extract_pem_blocks(pem_bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut blocks = Vec::new();
    let mut rest = pem_bytes;

    while let Some(end_position) = find_subslice(rest, END_CERTIFICATE) {
        let chunk = &rest[..end_position];

        if let Some(begin_position) = find_subslice(chunk, BEGIN_CERTIFICATE) {
            let mut block = chunk[begin_position..].to_vec();
            block.extend_from_slice(END_CERTIFICATE);
            blocks.push(block);
        }

        rest = &rest[end_position + END_CERTIFICATE.len()..];
    }

    blocks
}

#[derive(Clone)]
struct ParsedCertificate {
    der: Vec<u8>,
    pem: Vec<u8>,
    subject: Vec<u8>,
    issuer: Vec<u8>,
}

/// `load_certs_ordered`.
///
/// Returns `[ leaf, intermediates... ]` even when the PEM blocks were supplied
/// in arbitrary order. A self-signed bundle returns just the leaf.
pub fn load_certs_ordered(pem_bytes: &[u8]) -> HydrusResult<Vec<OrderedCertificate>> {
    let mut certificates = Vec::new();

    for block in extract_pem_blocks(pem_bytes) {
        let (_, pem) = parse_x509_pem(&block).map_err(|error| {
            certificate_error(format!(
                "Could not parse a certificate PEM block: {error:?}"
            ))
        })?;

        let parsed = pem.parse_x509().map_err(|error| {
            certificate_error(format!("Could not parse an X.509 certificate: {error:?}"))
        })?;

        certificates.push(parsed_certificate(parsed, block));
    }

    if certificates.is_empty() {
        return Err(certificate_error(
            "No certificates were found in the supplied PEM data!",
        ));
    }

    Ok(order_certificates(&certificates))
}

fn parsed_certificate(certificate: X509Certificate<'_>, pem: Vec<u8>) -> ParsedCertificate {
    ParsedCertificate {
        der: certificate.as_ref().to_vec(),
        pem,
        subject: certificate.subject().as_raw().to_vec(),
        issuer: certificate.issuer().as_raw().to_vec(),
    }
}

fn order_certificates(certificates: &[ParsedCertificate]) -> Vec<OrderedCertificate> {
    // Python builds these maps with the last certificate winning on duplicates.
    let mut by_subject: HashMap<Vec<u8>, usize> = HashMap::new();

    for (index, certificate) in certificates.iter().enumerate() {
        by_subject.insert(certificate.subject.clone(), index);
    }

    let issuers: HashSet<Vec<u8>> = certificates
        .iter()
        .map(|certificate| certificate.issuer.clone())
        .collect();

    let mut leaf_index = certificates
        .iter()
        .position(|certificate| !issuers.contains(&certificate.subject))
        .unwrap_or(0);

    let mut ordered = Vec::new();
    let mut visited = HashSet::new();

    visited.insert(leaf_index);
    ordered.push(ordered_certificate(&certificates[leaf_index]));

    loop {
        let current = &certificates[leaf_index];

        // A self-signed root is not sent to clients.
        if current.issuer == current.subject {
            break;
        }

        let Some(&next_index) = by_subject.get(&current.issuer) else {
            break;
        };

        // The Python loop assumes a well-formed chain. A visited check keeps a
        // malformed cycle from hanging a Rust server thread.
        if !visited.insert(next_index) {
            break;
        }

        let next = &certificates[next_index];

        // Stop before adding a self-signed root.
        if next.subject == next.issuer {
            break;
        }

        ordered.push(ordered_certificate(next));
        leaf_index = next_index;
    }

    ordered
}

fn ordered_certificate(certificate: &ParsedCertificate) -> OrderedCertificate {
    OrderedCertificate {
        der: certificate.der.clone(),
        pem: certificate.pem.clone(),
    }
}

/// The file-reading half of `generate_ssl_context_factory`.
///
/// The returned settings are the Twisted call's inputs: the ordered leaf and
/// chain, the private key, and the minimum TLS version.
pub fn generate_ssl_context_settings(
    ssl_cert_path: &Path,
    ssl_key_path: &Path,
) -> HydrusResult<TlsContextSettings> {
    let fullchain_pem = fs::read(ssl_cert_path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the SSL certificate \"{}\": {error}",
                ssl_cert_path.display()
            ),
        )
    })?;

    let certificates = load_certs_ordered(&fullchain_pem)?;

    let private_key_pem = fs::read(ssl_key_path).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!(
                "Could not read the SSL private key \"{}\": {error}",
                ssl_key_path.display()
            ),
        )
    })?;

    Ok(TlsContextSettings {
        certificates,
        private_key_pem,
        minimum_tls_version: TlsVersion::Tls1_2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};

    fn ca_params(common_name: &str) -> CertificateParams {
        let mut params = CertificateParams::new(vec![common_name.to_string()]).expect("params");

        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

        params
    }

    fn leaf_params(common_name: &str) -> CertificateParams {
        let mut params = CertificateParams::new(vec!["localhost".to_string()]).expect("params");

        params
            .distinguished_name
            .push(DnType::CommonName, common_name);

        params
    }

    #[test]
    fn a_self_signed_certificate_is_the_leaf() {
        let key = KeyPair::generate().expect("key");
        let certificate = leaf_params("localhost")
            .self_signed(&key)
            .expect("certificate");

        let ordered = load_certs_ordered(certificate.pem().as_bytes()).expect("ordered");

        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].der, certificate.der().as_ref());
    }

    #[test]
    fn the_chain_is_ordered_and_the_root_is_omitted() {
        let root_key = KeyPair::generate().expect("root key");
        let root_params = ca_params("root");
        let root_certificate = root_params.self_signed(&root_key).expect("root cert");

        let intermediate_key = KeyPair::generate().expect("intermediate key");
        let intermediate_params = ca_params("intermediate");
        let intermediate_certificate = intermediate_params
            .signed_by(
                &intermediate_key,
                &Issuer::from_params(&root_params, &root_key),
            )
            .expect("intermediate cert");

        let leaf_key = KeyPair::generate().expect("leaf key");
        let leaf_certificate = leaf_params("leaf")
            .signed_by(
                &leaf_key,
                &Issuer::from_params(&intermediate_params, &intermediate_key),
            )
            .expect("leaf cert");

        // Supply the bundle in a deliberately unhelpful order.
        let bundle = format!(
            "{}{}{}",
            root_certificate.pem(),
            leaf_certificate.pem(),
            intermediate_certificate.pem()
        );

        let ordered = load_certs_ordered(bundle.as_bytes()).expect("ordered");

        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].der, leaf_certificate.der().as_ref());
        assert_eq!(ordered[1].der, intermediate_certificate.der().as_ref());
        assert!(
            !ordered
                .iter()
                .any(|cert| cert.der == root_certificate.der().as_ref())
        );
    }

    #[test]
    fn settings_read_the_files_and_keep_the_chain_order() {
        let key = KeyPair::generate().expect("key");
        let certificate = leaf_params("localhost")
            .self_signed(&key)
            .expect("certificate");

        let unique = format!(
            "{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );

        let cert_path = std::env::temp_dir().join(format!("hydrus_cert_{unique}.pem"));
        let key_path = std::env::temp_dir().join(format!("hydrus_key_{unique}.pem"));

        std::fs::write(&cert_path, certificate.pem()).expect("write cert");
        std::fs::write(&key_path, key.serialize_pem()).expect("write key");

        let settings = generate_ssl_context_settings(&cert_path, &key_path).expect("settings");

        assert_eq!(settings.certificates.len(), 1);
        assert_eq!(settings.minimum_tls_version, TlsVersion::Tls1_2);
        assert_eq!(settings.minimum_tls_version.as_str(), "TLSv1.2");
        assert!(!settings.private_key_pem.is_empty());
        assert_eq!(
            settings.certificate_chain_pem(),
            settings.certificates[0].pem
        );

        std::fs::remove_file(&cert_path).expect("remove cert");
        std::fs::remove_file(&key_path).expect("remove key");
    }

    #[test]
    fn empty_or_malformed_pem_is_an_error() {
        assert!(load_certs_ordered(b"").is_err());
        assert!(load_certs_ordered(b"not a certificate").is_err());
        assert!(
            load_certs_ordered(
                b"-----BEGIN CERTIFICATE-----\nnot base64\n-----END CERTIFICATE-----"
            )
            .is_err()
        );
    }
}

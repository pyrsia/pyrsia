///
/// This module contains structs that are used to organize data in the pyrsia blockchain and also to
/// manage the blockchain.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Result;
use x509_parser::certificate::X509Certificate;

/// This is an enumeration of the signature algorithms supported for pyrsia's metadata.
/// No RSA size smaller than 4096 is supported. This is to give us plenty of time to adopt
/// quantum-resistant signature algorithms when they are available.
#[derive(Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    RSA4096,
}

#[derive(Serialize, Deserialize)]
pub struct X509CertificateSignature<'a> {
    pub x509_certificate: X509Certificate<'a>,
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub enum ExternalIdentity<'a> {
    X509(X509Certificate<'a>),
}

/// This struct contains the information for an identity stored in the blockchain.
#[derive(Serialize, Deserialize)]
pub struct Identity<'a> {
    pub identity_public_key: &'a str,
    pub identity_algorithm: SignatureAlgorithm,
    pub name: &'a str,
    pub description: &'a str,
    pub email: &'a str,
    pub web_url: &'a str,
    pub phone_number: &'a str,
    pub external_identities: Vec<ExternalIdentity<'a>>,
    pub signature: Vec<u8>,
}

/// Identities are allowed to share a trust store so others can trust the same certificate
/// authorities
#[derive(Serialize, Deserialize)]
pub struct TrustStore<'a> {
    pub id: &'a str,
    pub valid_after: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub trust_store: Vec<u8>,
    pub signature: Vec<u8>,
}

#[cfg(test)]
mod blockchain_tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

///
/// This module contains structs that are used to organize data in the pyrsia blockchain and also to
/// manage the blockchain.
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::*;


/// This is an enumeration of the signature algorithms supported for pyrsia's metadata.
/// No RSA size smaller than 4096 is supported. This is to give us plenty of time to adopt
/// quantum-resistant signature algorithms when they are available.
#[derive(Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    RSA4096,
}

#[derive(Serialize, Deserialize)]
pub struct X509CertificateSignature<'b> {
    pub x509_certificate_der: &'b str,
    pub signature: Vec<u8>,
    #[serde(skip)]
    #[serde(default="X509CertificateSignature::no_certificate")]
    x509_certificate_binary: Option<X509Certificate<'b>>
}

impl<'a> X509CertificateSignature<'a> {
    /// Parse the X.509 certificate from the PEM string
    pub fn x509_certificate(&mut self) -> Result<X509Certificate, anyhow::Error> {
        match X509Certificate::from_der(self.x509_certificate_der.as_bytes()) {
            Ok((_rem, x509)) => Ok(x509),
            _ => Err(anyhow!(format!("Unable to parse x509_certificate_der value {}", self.x509_certificate_der)))
        }
    }

    fn no_certificate() -> Option<X509Certificate<'a>> {
        Option::None
    }
}

#[derive(Serialize, Deserialize)]
pub enum ExternalIdentity<'a> {
    #[serde(borrow)]
    X509(X509CertificateSignature<'a>)
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
    pub valid_after: i64, // the number of non-leap seconds since January 1, 1970 0:00:00 UTC
    pub valid_until: i64, //the number of non-leap seconds since January 1, 1970 0:00:00 UTC
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

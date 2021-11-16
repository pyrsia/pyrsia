/// The blockchain manager is the interface that the rest of Pyrsia has with the blockchain.
///
/// Currently we are just defining an interface. There is no implementation of a real blockchain.
/// There is a mock implementation.
///
/// This module contains structs that are used to organize data in the pyrsia blockchain and also to
/// manage the blockchain.

use anyhow::{anyhow, Context, Error, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use x509_parser::certificate::X509Certificate;
use x509_parser::prelude::*;

//TODO Add delegation struct and trait.

/// This is an enumeration of the signature algorithms supported for pyrsia's metadata.
/// No RSA size smaller than 4096 is supported. This is to give us plenty of time to adopt
/// quantum-resistant signature algorithms when they are available.
#[derive(Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    RSA4096,
}

/// This struct can be included an identity as evidence of the entity that the identity corresponds
/// to in the real world. This struct contains an X.509 certificate, which provides evidence of an
/// external identity and its associated public key. It also contains a digital signature of the
/// internal identity's public key using the private key associated with the X.509 certificate and
/// the signature algorithm specified by the X.509 certificate.
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
    identity_public_key: &'a str,
    identity_algorithm: SignatureAlgorithm,
    name: &'a str,
    description: Option<&'a str>,
    email: Option<&'a str>,
    web_url: Option<&'a str>,
    pub phone_number: Option<&'a str>,
    pub external_identities: Vec<ExternalIdentity<'a>>,
    pub signature: Vec<u8>,
}

// TODO complete the TrustStore porition of the blockchain interface.

/// Identities are allowed to share a trust store so others can trust the same certificate
/// authorities
// #[derive(Serialize, Deserialize)]
// pub struct TrustStore<'a> {
//     pub id: &'a str,
//     pub valid_after: i64, // the number of non-leap seconds since January 1, 1970 0:00:00 UTC
//     pub valid_until: i64, //the number of non-leap seconds since January 1, 1970 0:00:00 UTC
//     pub trust_store: Vec<u8>,
//     pub signature: Vec<u8>,
// }
//
// impl<'a> TrustStore<'a> {
//     /// Get the valid_after time as a DateTime<Utc>
//     pub fn valid_after_as_datetime(self) -> Result<DateTime<Utc>, anyhow::Error> {
//         TrustStore::timestamp_to_datetime(self.valid_after)
//     }
//
//     pub fn valid_until_as_datetime(self) -> Result<DateTime<Utc>, anyhow::Error> {
//         TrustStore::timestamp_to_datetime(self.valid_until)
//     }
//
//     fn timestamp_to_datetime(ts: i64) -> Result<DateTime<Utc>, Error> {
//         match NaiveDateTime::from_timestamp_opt(ts, 0) {
//             Some(naive_date_time) => Ok(DateTime::from_utc(naive_date_time, Utc)),
//             None => Err(anyhow!("Timestamp value is out of range: {}", ts))
//         }
//     }
// }

#[cfg(test)]
mod blockchain_tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

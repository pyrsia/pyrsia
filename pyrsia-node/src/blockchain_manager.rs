/// The blockchain manager is the interface that the rest of Pyrsia has with the blockchain.
///
/// Currently we are just defining an interface. There is no implementation of a real blockchain.
/// There is a mock implementation.
///
/// This module contains structs that are used to organize data in the pyrsia blockchain and also to
/// manage the blockchain.

use anyhow::{anyhow, Context, Error, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use openssl::x509::X509;
use serde::{Deserialize, Serialize};

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
pub struct X509CertificateSignature {
    x509_certificate_der: Vec<u8>,
    signature: Vec<u8>,
    #[serde(skip)]
    #[serde(default="X509CertificateSignature::no_certificate")]
    x509_certificate_binary: Option<X509>
}

impl<'a> X509CertificateSignature {
    fn new(certificate: &X509, signature: Vec<u8>) -> Result<Self, anyhow::Error> {
        let der: Vec<u8> = certificate.to_der().context("Fail to get der encoding of certificate")?;
        Ok(X509CertificateSignature{ x509_certificate_der: der, signature, x509_certificate_binary: None })
    }


    /// Parse the X.509 certificate from the binary DER
    pub fn x509_certificate(&mut self) -> Result<X509, anyhow::Error> {
        match X509::from_der(self.x509_certificate_der.as_slice()).context("Unable to parse x509_certificate_der value ") {
            Ok(x509) => {
                self.x509_certificate_binary = Some(x509);
                Ok(self.x509_certificate_binary.as_ref().unwrap().clone())
            },
            _ => Err(anyhow!("Unable to parse x509_certificate_der value."))
        }
    }

    fn no_certificate() -> Option<X509> {
        Option::None
    }

    /// The raw signature
    pub fn raw_signature(self) -> Vec<u8> {
        self.signature.clone()
    }

    //TODO Add method to use the private key associated with the certificate and the signature algorithm specified by the certificate to create a signature based on the public key of the enclosing identity using the webpki crate.
    //TODO Add method to validate the signature using the public key and algorithm specified by the certificate using the webpki crate.
}

#[derive(Serialize, Deserialize)]
pub enum ExternalIdentity {
    //#[serde(borrow)]
    X509(X509CertificateSignature)
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
    pub external_identities: Vec<ExternalIdentity>,
    pub signature: Vec<u8>,
}

// TODO complete the TrustStore porition of the blockchain interface.

/// Identities are allowed to share a trust store so others can trust the same certificate
/// authorities
#[derive(Serialize, Deserialize)]
pub struct TrustStore<'a> {
    pub id: &'a str,
    pub identity_public_key: &'a str, // The identity that the trust store belongs to
    pub valid_after: i64, // the number of non-leap seconds since January 1, 1970 0:00:00 UTC
    pub valid_until: i64, //the number of non-leap seconds since January 1, 1970 0:00:00 UTC
    pub trust_store: Vec<u8>,
    pub signature: Vec<u8>,
}

impl<'a> TrustStore<'a> {
    /// Get the valid_after time as a DateTime<Utc>
    pub fn valid_after_as_datetime(self) -> Result<DateTime<Utc>, anyhow::Error> {
        TrustStore::timestamp_to_datetime(self.valid_after)
    }

    pub fn valid_until_as_datetime(self) -> Result<DateTime<Utc>, anyhow::Error> {
        TrustStore::timestamp_to_datetime(self.valid_until)
    }

    fn timestamp_to_datetime(ts: i64) -> Result<DateTime<Utc>, Error> {
        match NaiveDateTime::from_timestamp_opt(ts, 0) {
            Some(naive_date_time) => Ok(DateTime::from_utc(naive_date_time, Utc)),
            None => Err(anyhow!("Timestamp value is out of range: {}", ts))
        }
    }
}

#[cfg(test)]
mod blockchain_tests {
    use crate::blockchain_manager::X509CertificateSignature;
    use anyhow::{Result};

    #[test]
    fn x509_certificate_signaure_test() -> Result<(), anyhow::Error> {
        let der_encoded_cert: Vec<u8> = [0x30u8, 0x82u8, 0x07u8, 0xfdu8, 0x30u8, 0x82u8, 0x05u8, 0xe5u8, 0xa0u8, 0x03u8, 0x02u8, 0x01u8, 0x02u8, 0x02u8, 0x10u8, 0x68u8, 0x16u8, 0x04u8, 0xdfu8, 0xf3u8, 0x34u8, 0xf1u8, 0x71u8, 0xd8u8, 0x0au8, 0x73u8, 0x55u8, 0x99u8, 0xc1u8, 0x41u8, 0x72u8, 0x30u8, 0x0du8, 0x06u8, 0x09u8, 0x2au8, 0x86u8, 0x48u8, 0x86u8, 0xf7u8, 0x0du8, 0x01u8, 0x01u8, 0x0bu8, 0x05u8, 0x00u8, 0x30u8, 0x72u8, 0x31u8, 0x0bu8, 0x30u8, 0x09u8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x06u8, 0x13u8, 0x02u8, 0x55u8, 0x53u8, 0x31u8, 0x0eu8, 0x30u8, 0x0cu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x08u8, 0x0cu8, 0x05u8, 0x54u8, 0x65u8, 0x78u8, 0x61u8, 0x73u8, 0x31u8, 0x10u8, 0x30u8, 0x0eu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x07u8, 0x0cu8, 0x07u8, 0x48u8, 0x6fu8, 0x75u8, 0x73u8, 0x74u8, 0x6fu8, 0x6eu8, 0x31u8, 0x11u8, 0x30u8, 0x0fu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x0au8, 0x0cu8, 0x08u8, 0x53u8, 0x53u8, 0x4cu8, 0x20u8, 0x43u8, 0x6fu8, 0x72u8, 0x70u8, 0x31u8, 0x2eu8, 0x30u8, 0x2cu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x03u8, 0x0cu8, 0x25u8, 0x53u8, 0x53u8, 0x4cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x20u8, 0x45u8, 0x56u8, 0x20u8, 0x53u8, 0x53u8, 0x4cu8, 0x20u8, 0x49u8, 0x6eu8, 0x74u8, 0x65u8, 0x72u8, 0x6du8, 0x65u8, 0x64u8, 0x69u8, 0x61u8, 0x74u8, 0x65u8, 0x20u8, 0x43u8, 0x41u8, 0x20u8, 0x52u8, 0x53u8, 0x41u8, 0x20u8, 0x52u8, 0x33u8, 0x30u8, 0x1eu8, 0x17u8, 0x0du8, 0x32u8, 0x30u8, 0x30u8, 0x34u8, 0x30u8, 0x31u8, 0x30u8, 0x30u8, 0x35u8, 0x38u8, 0x33u8, 0x33u8, 0x5au8, 0x17u8, 0x0du8, 0x32u8, 0x31u8, 0x30u8, 0x37u8, 0x31u8, 0x36u8, 0x30u8, 0x30u8, 0x35u8, 0x38u8, 0x33u8, 0x33u8, 0x5au8, 0x30u8, 0x81u8, 0xbdu8, 0x31u8, 0x0bu8, 0x30u8, 0x09u8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x06u8, 0x13u8, 0x02u8, 0x55u8, 0x53u8, 0x31u8, 0x0eu8, 0x30u8, 0x0cu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x08u8, 0x0cu8, 0x05u8, 0x54u8, 0x65u8, 0x78u8, 0x61u8, 0x73u8, 0x31u8, 0x10u8, 0x30u8, 0x0eu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x07u8, 0x0cu8, 0x07u8, 0x48u8, 0x6fu8, 0x75u8, 0x73u8, 0x74u8, 0x6fu8, 0x6eu8, 0x31u8, 0x11u8, 0x30u8, 0x0fu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x0au8, 0x0cu8, 0x08u8, 0x53u8, 0x53u8, 0x4cu8, 0x20u8, 0x43u8, 0x6fu8, 0x72u8, 0x70u8, 0x31u8, 0x16u8, 0x30u8, 0x14u8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x05u8, 0x13u8, 0x0du8, 0x4eu8, 0x56u8, 0x32u8, 0x30u8, 0x30u8, 0x38u8, 0x31u8, 0x36u8, 0x31u8, 0x34u8, 0x32u8, 0x34u8, 0x33u8, 0x31u8, 0x14u8, 0x30u8, 0x12u8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x03u8, 0x0cu8, 0x0bu8, 0x77u8, 0x77u8, 0x77u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x31u8, 0x1du8, 0x30u8, 0x1bu8, 0x06u8, 0x03u8, 0x55u8, 0x04u8, 0x0fu8, 0x0cu8, 0x14u8, 0x50u8, 0x72u8, 0x69u8, 0x76u8, 0x61u8, 0x74u8, 0x65u8, 0x20u8, 0x4fu8, 0x72u8, 0x67u8, 0x61u8, 0x6eu8, 0x69u8, 0x7au8, 0x61u8, 0x74u8, 0x69u8, 0x6fu8, 0x6eu8, 0x31u8, 0x17u8, 0x30u8, 0x15u8, 0x06u8, 0x0bu8, 0x2bu8, 0x06u8, 0x01u8, 0x04u8, 0x01u8, 0x82u8, 0x37u8, 0x3cu8, 0x02u8, 0x01u8, 0x02u8, 0x0cu8, 0x06u8, 0x4eu8, 0x65u8, 0x76u8, 0x61u8, 0x64u8, 0x61u8, 0x31u8, 0x13u8, 0x30u8, 0x11u8, 0x06u8, 0x0bu8, 0x2bu8, 0x06u8, 0x01u8, 0x04u8, 0x01u8, 0x82u8, 0x37u8, 0x3cu8, 0x02u8, 0x01u8, 0x03u8, 0x13u8, 0x02u8, 0x55u8, 0x53u8, 0x30u8, 0x82u8, 0x01u8, 0x22u8, 0x30u8, 0x0du8, 0x06u8, 0x09u8, 0x2au8, 0x86u8, 0x48u8, 0x86u8, 0xf7u8, 0x0du8, 0x01u8, 0x01u8, 0x01u8, 0x05u8, 0x00u8, 0x03u8, 0x82u8, 0x01u8, 0x0fu8, 0x00u8, 0x30u8, 0x82u8, 0x01u8, 0x0au8, 0x02u8, 0x82u8, 0x01u8, 0x01u8, 0x00u8, 0xc7u8, 0x85u8, 0xe4u8, 0x64u8, 0x6du8, 0xbdu8, 0x45u8, 0x09u8, 0xceu8, 0xf1u8, 0x44u8, 0xabu8, 0x2du8, 0xc0u8, 0xadu8, 0x09u8, 0x20u8, 0x66u8, 0x8au8, 0x63u8, 0xcbu8, 0x7bu8, 0x25u8, 0xb4u8, 0xb6u8, 0x6du8, 0x0du8, 0x9bu8, 0xe9u8, 0x82u8, 0x09u8, 0x0eu8, 0x09u8, 0xc7u8, 0xb8u8, 0x86u8, 0x07u8, 0xa8u8, 0x1au8, 0xc2u8, 0x51u8, 0x5eu8, 0xfdu8, 0xa1u8, 0xe9u8, 0x62u8, 0x92u8, 0x4au8, 0x24u8, 0x46u8, 0x41u8, 0x6fu8, 0x72u8, 0xfau8, 0x5au8, 0x2au8, 0x29u8, 0xc5u8, 0x1cu8, 0x34u8, 0x07u8, 0x52u8, 0x95u8, 0x84u8, 0x23u8, 0xa4u8, 0x54u8, 0x11u8, 0x16u8, 0x26u8, 0x48u8, 0x28u8, 0x37u8, 0x3bu8, 0xc5u8, 0xa2u8, 0xe3u8, 0x6bu8, 0x8eu8, 0x71u8, 0x5du8, 0x81u8, 0xe5u8, 0x96u8, 0x9bu8, 0x99u8, 0x70u8, 0xa4u8, 0xc1u8, 0xdcu8, 0x58u8, 0xe4u8, 0x47u8, 0x25u8, 0xe7u8, 0x50u8, 0x5bu8, 0x33u8, 0xc5u8, 0x27u8, 0x19u8, 0xdau8, 0x00u8, 0x19u8, 0xb7u8, 0x4du8, 0x9au8, 0x24u8, 0x66u8, 0x4au8, 0x64u8, 0xe3u8, 0x72u8, 0xcfu8, 0xa5u8, 0x84u8, 0xccu8, 0x60u8, 0xe1u8, 0xf1u8, 0x58u8, 0xeau8, 0x50u8, 0x69u8, 0x88u8, 0x45u8, 0x45u8, 0x88u8, 0x65u8, 0x23u8, 0x19u8, 0x14u8, 0x7eu8, 0xebu8, 0x54u8, 0x7au8, 0xecu8, 0xbcu8, 0xfau8, 0x53u8, 0x82u8, 0x89u8, 0x78u8, 0xb3u8, 0x5cu8, 0x0au8, 0x6du8, 0x3bu8, 0x43u8, 0x01u8, 0x58u8, 0x28u8, 0x19u8, 0xa9u8, 0x8bu8, 0x4fu8, 0x20u8, 0x77u8, 0x28u8, 0x12u8, 0xbdu8, 0x17u8, 0x54u8, 0xc3u8, 0x9eu8, 0x49u8, 0xa2u8, 0x9au8, 0xdeu8, 0x76u8, 0x3fu8, 0x95u8, 0x1au8, 0xd8u8, 0xd4u8, 0x90u8, 0x1eu8, 0x21u8, 0x15u8, 0x3eu8, 0x06u8, 0x41u8, 0x7fu8, 0xe0u8, 0x86u8, 0xdeu8, 0xbdu8, 0x46u8, 0x5au8, 0xb3u8, 0xffu8, 0xefu8, 0x2eu8, 0xd1u8, 0xd1u8, 0x10u8, 0x92u8, 0x1bu8, 0x94u8, 0xbau8, 0xe7u8, 0x2bu8, 0xa9u8, 0xa9u8, 0x66u8, 0x48u8, 0x6cu8, 0xb8u8, 0xdcu8, 0x74u8, 0x70u8, 0x05u8, 0xf0u8, 0xcau8, 0x17u8, 0x06u8, 0x1eu8, 0x58u8, 0xceu8, 0xc2u8, 0x3cu8, 0xc7u8, 0x79u8, 0x7bu8, 0xf7u8, 0x4eu8, 0xfau8, 0xddu8, 0x3cu8, 0xb7u8, 0xc3u8, 0xdbu8, 0x8fu8, 0x35u8, 0x53u8, 0x4eu8, 0xfeu8, 0x61u8, 0x40u8, 0x30u8, 0xacu8, 0x11u8, 0x82u8, 0x15u8, 0xd9u8, 0x3eu8, 0xc0u8, 0x14u8, 0x8fu8, 0x52u8, 0x70u8, 0xdcu8, 0x4cu8, 0x92u8, 0x1eu8, 0xffu8, 0x02u8, 0x03u8, 0x01u8, 0x00u8, 0x01u8, 0xa3u8, 0x82u8, 0x03u8, 0x41u8, 0x30u8, 0x82u8, 0x03u8, 0x3du8, 0x30u8, 0x1fu8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x23u8, 0x04u8, 0x18u8, 0x30u8, 0x16u8, 0x80u8, 0x14u8, 0xbfu8, 0xc1u8, 0x5au8, 0x87u8, 0xffu8, 0x28u8, 0xfau8, 0x41u8, 0x3du8, 0xfdu8, 0xb7u8, 0x4fu8, 0xe4u8, 0x1du8, 0xafu8, 0xa0u8, 0x61u8, 0x58u8, 0x29u8, 0xbdu8, 0x30u8, 0x7fu8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x01u8, 0x01u8, 0x04u8, 0x73u8, 0x30u8, 0x71u8, 0x30u8, 0x4du8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x30u8, 0x02u8, 0x86u8, 0x41u8, 0x68u8, 0x74u8, 0x74u8, 0x70u8, 0x3au8, 0x2fu8, 0x2fu8, 0x77u8, 0x77u8, 0x77u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x2fu8, 0x72u8, 0x65u8, 0x70u8, 0x6fu8, 0x73u8, 0x69u8, 0x74u8, 0x6fu8, 0x72u8, 0x79u8, 0x2fu8, 0x53u8, 0x53u8, 0x4cu8, 0x63u8, 0x6fu8, 0x6du8, 0x2du8, 0x53u8, 0x75u8, 0x62u8, 0x43u8, 0x41u8, 0x2du8, 0x45u8, 0x56u8, 0x2du8, 0x53u8, 0x53u8, 0x4cu8, 0x2du8, 0x52u8, 0x53u8, 0x41u8, 0x2du8, 0x34u8, 0x30u8, 0x39u8, 0x36u8, 0x2du8, 0x52u8, 0x33u8, 0x2eu8, 0x63u8, 0x72u8, 0x74u8, 0x30u8, 0x20u8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x30u8, 0x01u8, 0x86u8, 0x14u8, 0x68u8, 0x74u8, 0x74u8, 0x70u8, 0x3au8, 0x2fu8, 0x2fu8, 0x6fu8, 0x63u8, 0x73u8, 0x70u8, 0x73u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x30u8, 0x1fu8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x11u8, 0x04u8, 0x18u8, 0x30u8, 0x16u8, 0x82u8, 0x0bu8, 0x77u8, 0x77u8, 0x77u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x82u8, 0x07u8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x30u8, 0x5fu8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x20u8, 0x04u8, 0x58u8, 0x30u8, 0x56u8, 0x30u8, 0x07u8, 0x06u8, 0x05u8, 0x67u8, 0x81u8, 0x0cu8, 0x01u8, 0x01u8, 0x30u8, 0x0du8, 0x06u8, 0x0bu8, 0x2au8, 0x84u8, 0x68u8, 0x01u8, 0x86u8, 0xf6u8, 0x77u8, 0x02u8, 0x05u8, 0x01u8, 0x01u8, 0x30u8, 0x3cu8, 0x06u8, 0x0cu8, 0x2bu8, 0x06u8, 0x01u8, 0x04u8, 0x01u8, 0x82u8, 0xa9u8, 0x30u8, 0x01u8, 0x03u8, 0x01u8, 0x04u8, 0x30u8, 0x2cu8, 0x30u8, 0x2au8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x02u8, 0x01u8, 0x16u8, 0x1eu8, 0x68u8, 0x74u8, 0x74u8, 0x70u8, 0x73u8, 0x3au8, 0x2fu8, 0x2fu8, 0x77u8, 0x77u8, 0x77u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x2fu8, 0x72u8, 0x65u8, 0x70u8, 0x6fu8, 0x73u8, 0x69u8, 0x74u8, 0x6fu8, 0x72u8, 0x79u8, 0x30u8, 0x1du8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x25u8, 0x04u8, 0x16u8, 0x30u8, 0x14u8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x03u8, 0x02u8, 0x06u8, 0x08u8, 0x2bu8, 0x06u8, 0x01u8, 0x05u8, 0x05u8, 0x07u8, 0x03u8, 0x01u8, 0x30u8, 0x48u8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x1fu8, 0x04u8, 0x41u8, 0x30u8, 0x3fu8, 0x30u8, 0x3du8, 0xa0u8, 0x3bu8, 0xa0u8, 0x39u8, 0x86u8, 0x37u8, 0x68u8, 0x74u8, 0x74u8, 0x70u8, 0x3au8, 0x2fu8, 0x2fu8, 0x63u8, 0x72u8, 0x6cu8, 0x73u8, 0x2eu8, 0x73u8, 0x73u8, 0x6cu8, 0x2eu8, 0x63u8, 0x6fu8, 0x6du8, 0x2fu8, 0x53u8, 0x53u8, 0x4cu8, 0x63u8, 0x6fu8, 0x6du8, 0x2du8, 0x53u8, 0x75u8, 0x62u8, 0x43u8, 0x41u8, 0x2du8, 0x45u8, 0x56u8, 0x2du8, 0x53u8, 0x53u8, 0x4cu8, 0x2du8, 0x52u8, 0x53u8, 0x41u8, 0x2du8, 0x34u8, 0x30u8, 0x39u8, 0x36u8, 0x2du8, 0x52u8, 0x33u8, 0x2eu8, 0x63u8, 0x72u8, 0x6cu8, 0x30u8, 0x1du8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x0eu8, 0x04u8, 0x16u8, 0x04u8, 0x14u8, 0x00u8, 0xc0u8, 0x15u8, 0x42u8, 0x1au8, 0xcfu8, 0x0eu8, 0x6bu8, 0x64u8, 0x81u8, 0xdau8, 0xa6u8, 0x74u8, 0x71u8, 0x21u8, 0x49u8, 0xe9u8, 0xc3u8, 0xe1u8, 0x8bu8, 0x30u8, 0x0eu8, 0x06u8, 0x03u8, 0x55u8, 0x1du8, 0x0fu8, 0x01u8, 0x01u8, 0xffu8, 0x04u8, 0x04u8, 0x03u8, 0x02u8, 0x05u8, 0xa0u8, 0x30u8, 0x82u8, 0x01u8, 0x7du8, 0x06u8, 0x0au8, 0x2bu8, 0x06u8, 0x01u8, 0x04u8, 0x01u8, 0xd6u8, 0x79u8, 0x02u8, 0x04u8, 0x02u8, 0x04u8, 0x82u8, 0x01u8, 0x6du8, 0x04u8, 0x82u8, 0x01u8, 0x69u8, 0x01u8, 0x67u8, 0x00u8, 0x77u8, 0x00u8, 0xf6u8, 0x5cu8, 0x94u8, 0x2fu8, 0xd1u8, 0x77u8, 0x30u8, 0x22u8, 0x14u8, 0x54u8, 0x18u8, 0x08u8, 0x30u8, 0x94u8, 0x56u8, 0x8eu8, 0xe3u8, 0x4du8, 0x13u8, 0x19u8, 0x33u8, 0xbfu8, 0xdfu8, 0x0cu8, 0x2fu8, 0x20u8, 0x0bu8, 0xccu8, 0x4eu8, 0xf1u8, 0x64u8, 0xe3u8, 0x00u8, 0x00u8, 0x01u8, 0x71u8, 0x33u8, 0x48u8, 0x68u8, 0x6fu8, 0x00u8, 0x00u8, 0x04u8, 0x03u8, 0x00u8, 0x48u8, 0x30u8, 0x46u8, 0x02u8, 0x21u8, 0x00u8, 0xebu8, 0x17u8, 0xa5u8, 0x88u8, 0xd4u8, 0x7cu8, 0x1au8, 0x4fu8, 0xfau8, 0xdeu8, 0x96u8, 0x1du8, 0x9du8, 0x2fu8, 0xefu8, 0x3bu8, 0x1fu8, 0xc2u8, 0x8eu8, 0x9bu8, 0x44u8, 0x30u8, 0x4bu8, 0xfcu8, 0xf5u8, 0x65u8, 0xa1u8, 0xd7u8, 0xfbu8, 0xabu8, 0x58u8, 0x81u8, 0x02u8, 0x21u8, 0x00u8, 0xf2u8, 0x06u8, 0xb7u8, 0x87u8, 0x53u8, 0x6eu8, 0x43u8, 0xcfu8, 0x0bu8, 0xa4u8, 0x41u8, 0xa4u8, 0x50u8, 0x8fu8, 0x05u8, 0xbau8, 0xe7u8, 0x96u8, 0x4bu8, 0x92u8, 0xa0u8, 0xa7u8, 0xc5u8, 0xbcu8, 0x50u8, 0x59u8, 0x18u8, 0x8eu8, 0x7au8, 0x68u8, 0xfdu8, 0x24u8, 0x00u8, 0x75u8, 0x00u8, 0x94u8, 0x20u8, 0xbcu8, 0x1eu8, 0x8eu8, 0xd5u8, 0x8du8, 0x6cu8, 0x88u8, 0x73u8, 0x1fu8, 0x82u8, 0x8bu8, 0x22u8, 0x2cu8, 0x0du8, 0xd1u8, 0xdau8, 0x4du8, 0x5eu8, 0x6cu8, 0x4fu8, 0x94u8, 0x3du8, 0x61u8, 0xdbu8, 0x4eu8, 0x2fu8, 0x58u8, 0x4du8, 0xa2u8, 0xc2u8, 0x00u8, 0x00u8, 0x01u8, 0x71u8, 0x33u8, 0x48u8, 0x68u8, 0xdcu8, 0x00u8, 0x00u8, 0x04u8, 0x03u8, 0x00u8, 0x46u8, 0x30u8, 0x44u8, 0x02u8, 0x20u8, 0x19u8, 0x11u8, 0x38u8, 0xc3u8, 0x36u8, 0x9bu8, 0x35u8, 0x17u8, 0x43u8, 0xf2u8, 0x4au8, 0xbfu8, 0xbcu8, 0x53u8, 0xf7u8, 0xb5u8, 0x07u8, 0xb6u8, 0x86u8, 0x6du8, 0x31u8, 0xe6u8, 0x75u8, 0xeeu8, 0x96u8, 0x8cu8, 0x21u8, 0xe0u8, 0x86u8, 0xf0u8, 0xdeu8, 0x59u8, 0x02u8, 0x20u8, 0x56u8, 0x1bu8, 0xffu8, 0x79u8, 0x52u8, 0x0eu8, 0x99u8, 0x52u8, 0xecu8, 0x07u8, 0x11u8, 0xe2u8, 0xbfu8, 0x97u8, 0xa5u8, 0x6bu8, 0x44u8, 0x29u8, 0x24u8, 0xc5u8, 0x58u8, 0x99u8, 0x8du8, 0x09u8, 0x16u8, 0xdcu8, 0x5cu8, 0x9bu8, 0xabu8, 0xd9u8, 0x11u8, 0x81u8, 0x00u8, 0x75u8, 0x00u8, 0xeeu8, 0xc0u8, 0x95u8, 0xeeu8, 0x8du8, 0x72u8, 0x64u8, 0x0fu8, 0x92u8, 0xe3u8, 0xc3u8, 0xb9u8, 0x1bu8, 0xc7u8, 0x12u8, 0xa3u8, 0x69u8, 0x6au8, 0x09u8, 0x7bu8, 0x4bu8, 0x6au8, 0x1au8, 0x14u8, 0x38u8, 0xe6u8, 0x47u8, 0xb2u8, 0xcbu8, 0xedu8, 0xc5u8, 0xf9u8, 0x00u8, 0x00u8, 0x01u8, 0x71u8, 0x33u8, 0x48u8, 0x68u8, 0xf3u8, 0x00u8, 0x00u8, 0x04u8, 0x03u8, 0x00u8, 0x46u8, 0x30u8, 0x44u8, 0x02u8, 0x20u8, 0x7au8, 0x22u8, 0xf6u8, 0xe8u8, 0x5au8, 0xcbu8, 0x37u8, 0x47u8, 0x82u8, 0x2du8, 0x57u8, 0x08u8, 0xdeu8, 0x6eu8, 0x5eu8, 0xc3u8, 0xdfu8, 0x2au8, 0x05u8, 0x69u8, 0x7du8, 0x0du8, 0x0eu8, 0x1du8, 0x9du8, 0x5au8, 0x18u8, 0x60u8, 0xc0u8, 0x2cu8, 0x6bu8, 0x1fu8, 0x02u8, 0x20u8, 0x09u8, 0xfau8, 0xbbu8, 0xa1u8, 0xc3u8, 0x02u8, 0xe6u8, 0xdfu8, 0xb5u8, 0x8eu8, 0x2eu8, 0x4cu8, 0xe7u8, 0x16u8, 0x8bu8, 0x98u8, 0xf0u8, 0xb8u8, 0x23u8, 0xe5u8, 0x97u8, 0xdcu8, 0x8fu8, 0xc0u8, 0x46u8, 0x45u8, 0x92u8, 0xcau8, 0x23u8, 0xbbu8, 0x21u8, 0x07u8, 0x30u8, 0x0du8, 0x06u8, 0x09u8, 0x2au8, 0x86u8, 0x48u8, 0x86u8, 0xf7u8, 0x0du8, 0x01u8, 0x01u8, 0x0bu8, 0x05u8, 0x00u8, 0x03u8, 0x82u8, 0x02u8, 0x01u8, 0x00u8, 0x27u8, 0xaeu8, 0xbau8, 0xbeu8, 0x10u8, 0x9eu8, 0xe8u8, 0xeau8, 0x9au8, 0x0bu8, 0x92u8, 0xacu8, 0x75u8, 0x37u8, 0x9au8, 0x17u8, 0xfeu8, 0x70u8, 0x9au8, 0x1du8, 0xcdu8, 0x34u8, 0x0du8, 0xaau8, 0x8eu8, 0x2du8, 0x75u8, 0xefu8, 0x8fu8, 0x0fu8, 0x5fu8, 0xdeu8, 0x15u8, 0xd6u8, 0x00u8, 0x10u8, 0xbbu8, 0xbcu8, 0xc4u8, 0x5fu8, 0xb4u8, 0x02u8, 0xdeu8, 0xf1u8, 0x26u8, 0x23u8, 0xd8u8, 0x8bu8, 0x94u8, 0x4au8, 0xc2u8, 0x29u8, 0x72u8, 0x3fu8, 0x9eu8, 0xafu8, 0xfbu8, 0x78u8, 0x98u8, 0xd9u8, 0x3fu8, 0x65u8, 0xc3u8, 0xb4u8, 0xbcu8, 0x4cu8, 0x9du8, 0x38u8, 0xd5u8, 0x52u8, 0xe1u8, 0x68u8, 0x82u8, 0xa9u8, 0xd7u8, 0x83u8, 0x33u8, 0x49u8, 0x4cu8, 0xd1u8, 0xc9u8, 0xeau8, 0x0eu8, 0x02u8, 0xc2u8, 0x7bu8, 0x40u8, 0x00u8, 0xccu8, 0x0au8, 0x51u8, 0xcau8, 0x50u8, 0x39u8, 0x47u8, 0x51u8, 0x4du8, 0xa9u8, 0x36u8, 0xeau8, 0x3cu8, 0xf1u8, 0x8eu8, 0xa2u8, 0x82u8, 0x8bu8, 0xd3u8, 0xddu8, 0xbbu8, 0x27u8, 0xc0u8, 0x93u8, 0x62u8, 0x11u8, 0x03u8, 0x6au8, 0xcau8, 0x64u8, 0x92u8, 0x62u8, 0x19u8, 0x2du8, 0xc3u8, 0x4bu8, 0x5au8, 0x76u8, 0xeau8, 0x2au8, 0x8eu8, 0xa5u8, 0xe7u8, 0xd3u8, 0xa8u8, 0x2cu8, 0x56u8, 0x2au8, 0x16u8, 0x4du8, 0x50u8, 0xd7u8, 0xcau8, 0xc7u8, 0x79u8, 0xa8u8, 0x4cu8, 0x78u8, 0xb7u8, 0xabu8, 0x08u8, 0x80u8, 0x87u8, 0x0cu8, 0x9bu8, 0x6eu8, 0x98u8, 0x1fu8, 0x5bu8, 0xc9u8, 0xa4u8, 0x24u8, 0x04u8, 0x84u8, 0xaau8, 0x5cu8, 0xdbu8, 0x2du8, 0x3bu8, 0x81u8, 0x19u8, 0x24u8, 0x94u8, 0x16u8, 0x51u8, 0xb4u8, 0xc8u8, 0xd3u8, 0x86u8, 0xfeu8, 0x1cu8, 0x5fu8, 0x2cu8, 0x8cu8, 0x5fu8, 0xbbu8, 0x93u8, 0x71u8, 0xd4u8, 0xfbu8, 0x00u8, 0x90u8, 0x4fu8, 0xb9u8, 0xe8u8, 0x9fu8, 0x0au8, 0x85u8, 0x76u8, 0xe4u8, 0x9cu8, 0x57u8, 0xbau8, 0x8fu8, 0x1du8, 0xe7u8, 0x5du8, 0xfdu8, 0x83u8, 0x03u8, 0xf5u8, 0x04u8, 0x07u8, 0xbbu8, 0x20u8, 0x15u8, 0x4fu8, 0xc7u8, 0x6bu8, 0xbbu8, 0x28u8, 0xdfu8, 0xd4u8, 0xc8u8, 0xe5u8, 0xddu8, 0x66u8, 0x6cu8, 0x0cu8, 0x7fu8, 0xf4u8, 0xe6u8, 0x14u8, 0x6cu8, 0x03u8, 0x74u8, 0x27u8, 0xecu8, 0xc8u8, 0x77u8, 0xffu8, 0x66u8, 0xc0u8, 0x76u8, 0xc0u8, 0xb1u8, 0xe8u8, 0xcdu8, 0x36u8, 0x28u8, 0x01u8, 0x59u8, 0x90u8, 0xf4u8, 0x5au8, 0x14u8, 0xd4u8, 0x92u8, 0xe0u8, 0x71u8, 0x58u8, 0xafu8, 0xa8u8, 0x9fu8, 0xafu8, 0x36u8, 0x50u8, 0x61u8, 0x1du8, 0x78u8, 0x65u8, 0xc4u8, 0xc7u8, 0x4du8, 0xd2u8, 0x3fu8, 0x34u8, 0x47u8, 0xd3u8, 0x73u8, 0xe8u8, 0x42u8, 0x20u8, 0x95u8, 0x08u8, 0xdeu8, 0x2bu8, 0x73u8, 0xbcu8, 0x23u8, 0xf7u8, 0x05u8, 0x1au8, 0x6fu8, 0xc1u8, 0xf3u8, 0xeeu8, 0x36u8, 0x84u8, 0xe9u8, 0x42u8, 0x21u8, 0xdfu8, 0x59u8, 0x76u8, 0xd9u8, 0xddu8, 0x25u8, 0xc4u8, 0x49u8, 0x56u8, 0x38u8, 0xb4u8, 0xc0u8, 0x3du8, 0x2au8, 0xc1u8, 0xebu8, 0xc2u8, 0x69u8, 0xf0u8, 0x3du8, 0x8cu8, 0x99u8, 0x47u8, 0xbfu8, 0xf8u8, 0xecu8, 0x13u8, 0xe2u8, 0x3du8, 0x53u8, 0x3eu8, 0x9cu8, 0xa4u8, 0x2cu8, 0xa1u8, 0xb3u8, 0x0fu8, 0xa5u8, 0xacu8, 0x57u8, 0x71u8, 0x52u8, 0x0au8, 0x94u8, 0xe7u8, 0xc6u8, 0xb1u8, 0xa9u8, 0xe2u8, 0xbcu8, 0xf4u8, 0x54u8, 0x7eu8, 0x36u8, 0x8eu8, 0x2au8, 0xd0u8, 0x82u8, 0x0eu8, 0xf8u8, 0x98u8, 0xb5u8, 0xacu8, 0x92u8, 0xabu8, 0xf6u8, 0x79u8, 0x12u8, 0x07u8, 0x40u8, 0x6au8, 0x5eu8, 0x8cu8, 0xd5u8, 0x9cu8, 0x4du8, 0x58u8, 0x07u8, 0xf2u8, 0x8bu8, 0xbdu8, 0xd2u8, 0x2cu8, 0xb9u8, 0x86u8, 0x49u8, 0xbau8, 0xa6u8, 0xf6u8, 0xa4u8, 0xa9u8, 0x2eu8, 0xfbu8, 0x3cu8, 0xd3u8, 0xeau8, 0x05u8, 0x30u8, 0x1du8, 0x44u8, 0xd9u8, 0xbcu8, 0x18u8, 0x8du8, 0x3au8, 0xd5u8, 0xcbu8, 0xe0u8, 0xdcu8, 0x70u8, 0x73u8, 0xf2u8, 0x93u8, 0xedu8, 0x6cu8, 0xceu8, 0x49u8, 0xddu8, 0xb0u8, 0x3fu8, 0x5du8, 0x10u8, 0x23u8, 0xc0u8, 0xcau8, 0x83u8, 0x8bu8, 0xdfu8, 0x88u8, 0xd0u8, 0xecu8, 0x1du8, 0x69u8, 0x81u8, 0xd5u8, 0xceu8, 0x0au8, 0x8eu8, 0x2eu8, 0xa0u8, 0x3au8, 0x00u8, 0x39u8, 0xb9u8, 0x25u8, 0x33u8, 0x68u8, 0x69u8, 0xaau8, 0xfeu8, 0xfeu8, 0x15u8, 0x9du8, 0xc2u8, 0xb9u8, 0x52u8, 0xbfu8, 0xa7u8, 0xf4u8, 0xb6u8, 0xdfu8, 0x9du8, 0xf2u8, 0xdcu8, 0xdbu8, 0xc2u8, 0x79u8, 0x7eu8, 0xdfu8, 0xc6u8, 0xa2u8, 0xd8u8, 0xa7u8, 0x33u8, 0x20u8, 0xe4u8, 0xdeu8, 0x26u8, 0xabu8, 0x17u8, 0x5du8, 0x18u8, 0x96u8, 0xa7u8, 0x0eu8, 0x99u8, 0xe5u8, 0xf5u8, 0xb8u8, 0x59u8, 0x8au8, 0x6du8, 0xd8u8, 0xbfu8, 0x5eu8, 0x8au8, 0xc6u8, 0x96u8, 0x40u8, 0xa8u8, 0x30u8, 0x5du8, 0xd3u8, 0x0fu8, 0x1fu8, 0x2bu8, 0x9au8, 0x9fu8, 0x43u8, 0x06u8, 0x20u8, 0x7fu8].to_vec();

        let mut xcs = X509CertificateSignature{x509_certificate_der : der_encoded_cert.to_vec(), signature: vec![], x509_certificate_binary: Option::None };
        xcs.x509_certificate()?;
        Ok(())
    }
}

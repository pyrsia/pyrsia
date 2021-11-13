///
/// This module contains structs that are used to organize data in the pyrsia blockchain and also to
/// manage the blockchain.
pub mod blockchain_manager {
    use chrono::{DateTime, Utc};
    use x509_parser::certificate::X509Certificate;

    /// This is an enumeration of the signature algorithms supported for pyrsia's metadata.
    /// No RSA size smaller than 4096 is supported. This is to give us plenty of time to adopt
    /// quantum-resistant signature algorithms when they are available.
    pub enum SignatureAlgorithm {
        RSA4096,
    }

    pub struct X509CertificateSignature<'a> {
        x509_certificate: X509Certificate<'a>,
        signature: Vec<u8>
    }

    /// This struct contains the information for an identity stored in the blockchain.
    pub struct Identity<'a> {
        identity_public_key: str,
        identity_algorithm: SignatureAlgorithm,
        name: str,
        description: str,
        email: str,
        web_url: str,
        phone_number: str,
        x509: &'a X509Certificate<'a>,
        signature: Vec<u8>
    }

    /// Identities are allowed to share a trust store so others can trust the same certificate
    /// authorities
    pub struct TrustStore<'a> {
        id: &'a str,
        valid_after: DateTime<Utc>,
        valid_until: DateTime<Utc>,
        trust_store: Vec<u8>,
        signature: Vec<u8>
    }
}


#[cfg(test)]
mod blockchain_tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

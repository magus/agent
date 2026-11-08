use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use security_framework::passwords;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io;

#[derive(Serialize, Deserialize)]
pub(super) struct Identity {
    pub certificate: Vec<u8>,
    private_key: Vec<u8>,
}

impl Identity {
    pub fn load() -> io::Result<Self> {
        let service = "com.iamnoah.agent.device";
        match passwords::get_generic_password(service, "identity") {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.code() == -25300 => {
                let identity = Self::generate()?;
                let bytes = serde_json::to_vec(&identity)?;
                passwords::set_generic_password(service, "identity", &bytes)
                    .map_err(|_| io::Error::other("Could not save device identity in Keychain"))?;
                Ok(identity)
            }
            Err(_) => Err(io::Error::other(
                "Could not read device identity from Keychain",
            )),
        }
    }

    pub fn generate() -> io::Result<Self> {
        let generated = rcgen::generate_simple_self_signed(vec!["agent.local".into()])
            .map_err(io::Error::other)?;
        Ok(Self {
            certificate: generated.cert.der().to_vec(),
            private_key: generated.signing_key.serialize_der(),
        })
    }

    pub fn certificate(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.certificate.clone())
    }

    pub fn key(&self) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(self.private_key.clone()).into()
    }
}

pub(super) fn device_id(certificate: &[u8]) -> String {
    format!("{:x}", Sha256::digest(certificate))
}

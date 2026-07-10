use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Serialize, Serializer};

use crate::{AxisError, AxisResult, PublicIdentity, canonical_bytes};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SignatureBytes([u8; 64]);

impl SignatureBytes {
    pub const fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 64] {
        self.0
    }
}

impl Serialize for SignatureBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

pub fn verify_signature<T: Serialize>(
    identity: PublicIdentity,
    signature: SignatureBytes,
    domain: &str,
    payload: &T,
) -> AxisResult<()> {
    identity.verify_consistency()?;
    let verifying_key = VerifyingKey::from_bytes(&identity.verifying_key)
        .map_err(|error| AxisError::Signature(error.to_string()))?;
    let signature = Signature::from_bytes(&signature.bytes());
    let bytes = canonical_bytes(&(domain, payload))?;
    verifying_key
        .verify(&bytes, &signature)
        .map_err(|error| AxisError::Signature(error.to_string()))
}

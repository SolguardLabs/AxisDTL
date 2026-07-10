use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;

use crate::{AccountId, AxisError, AxisResult, Digest, SignatureBytes, canonical_bytes};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PublicIdentity {
    pub account: AccountId,
    pub verifying_key: [u8; 32],
}

pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn public_identity(&self) -> PublicIdentity {
        let verifying_key = self.signing_key.verifying_key().to_bytes();
        let account =
            AccountId::from_bytes(Digest::from_parts("axis-account-v1", &[&verifying_key]).bytes());
        PublicIdentity {
            account,
            verifying_key,
        }
    }

    pub fn sign<T: Serialize>(&self, domain: &str, payload: &T) -> AxisResult<SignatureBytes> {
        let bytes = canonical_bytes(&(domain, payload))?;
        Ok(SignatureBytes::from_bytes(
            self.signing_key.sign(&bytes).to_bytes(),
        ))
    }
}

impl PublicIdentity {
    pub fn verify_consistency(self) -> AxisResult<()> {
        let expected = AccountId::from_bytes(
            Digest::from_parts("axis-account-v1", &[&self.verifying_key]).bytes(),
        );
        if expected != self.account {
            return Err(AxisError::Policy(
                "identity/account binding mismatch".to_owned(),
            ));
        }
        Ok(())
    }
}

use serde::Serialize;

use crate::{
    AccountId, AxisError, AxisResult, Digest, KeyPair, OrderId, PublicIdentity, SignatureBytes,
    TxId, verify_signature,
};

pub const SETTLEMENT_DOMAIN: &str = "axis-swap-settlement-v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementRequest {
    pub network_id: u32,
    pub order_id: OrderId,
    pub solver: AccountId,
    pub settlement_nonce: u64,
    pub observed_quote_digest: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementAuthorizationView {
    network_id: u32,
    order_id: OrderId,
    solver: AccountId,
    settlement_nonce: u64,
    observed_quote_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedSettlement {
    pub signer: PublicIdentity,
    pub request: SettlementRequest,
    pub signature: SignatureBytes,
}

impl SettlementRequest {
    pub fn new(
        network_id: u32,
        order_id: OrderId,
        solver: AccountId,
        settlement_nonce: u64,
        observed_quote_digest: Digest,
    ) -> Self {
        Self {
            network_id,
            order_id,
            solver,
            settlement_nonce,
            observed_quote_digest,
        }
    }

    pub fn authorization_view(self) -> SettlementAuthorizationView {
        SettlementAuthorizationView {
            network_id: self.network_id,
            order_id: self.order_id,
            solver: self.solver,
            settlement_nonce: self.settlement_nonce,
            observed_quote_digest: self.observed_quote_digest,
        }
    }
}

impl SignedSettlement {
    pub fn sign(request: SettlementRequest, key_pair: &KeyPair) -> AxisResult<Self> {
        let signer = key_pair.public_identity();
        if signer.account != request.solver {
            return Err(AxisError::UnauthorizedSettlementSigner {
                expected: request.solver,
                received: signer.account,
            });
        }
        let signature = key_pair.sign(SETTLEMENT_DOMAIN, &request.authorization_view())?;
        Ok(Self {
            signer,
            request,
            signature,
        })
    }

    pub fn verify(&self) -> AxisResult<()> {
        if self.signer.account != self.request.solver {
            return Err(AxisError::UnauthorizedSettlementSigner {
                expected: self.request.solver,
                received: self.signer.account,
            });
        }
        verify_signature(
            self.signer,
            self.signature,
            SETTLEMENT_DOMAIN,
            &self.request.authorization_view(),
        )
    }

    pub fn tx_id(&self) -> AxisResult<TxId> {
        TxId::from_serializable("axis-signed-settlement-v1", self)
    }
}

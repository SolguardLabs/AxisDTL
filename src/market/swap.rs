use serde::Serialize;

use crate::{
    AccountId, Amount, AssetId, AxisError, AxisResult, Digest, ExecutionQuote, KeyPair, OrderId,
    PublicIdentity, SignatureBytes, TxId, verify_signature,
};

pub const SWAP_DOMAIN: &str = "axis-swap-open-v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwapTerms {
    pub network_id: u32,
    pub order_id: OrderId,
    pub payer: AccountId,
    pub receiver: AccountId,
    pub source_asset: AssetId,
    pub target_asset: AssetId,
    pub source_amount: Amount,
    pub min_output: Amount,
    pub payer_nonce: u64,
    pub quote: ExecutionQuote,
    pub salt: Digest,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwapAuthorizationView {
    network_id: u32,
    order_id: OrderId,
    payer: AccountId,
    receiver: AccountId,
    source_asset: AssetId,
    target_asset: AssetId,
    source_amount: Amount,
    min_output: Amount,
    payer_nonce: u64,
    quote_digest: Digest,
    salt: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedSwap {
    pub payer: PublicIdentity,
    pub terms: SwapTerms,
    pub signature: SignatureBytes,
}

impl SwapTerms {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        network_id: u32,
        payer: AccountId,
        receiver: AccountId,
        source_asset: AssetId,
        target_asset: AssetId,
        source_amount: Amount,
        min_output: Amount,
        payer_nonce: u64,
        quote: ExecutionQuote,
        salt: Digest,
    ) -> AxisResult<Self> {
        if source_amount.is_zero() {
            return Err(AxisError::ZeroAmount);
        }
        if quote.source_asset != source_asset || quote.target_asset != target_asset {
            return Err(AxisError::Policy("quote asset mismatch".to_owned()));
        }
        let order_id = OrderId::derive(
            network_id,
            payer,
            receiver,
            source_asset,
            target_asset,
            payer_nonce,
            salt,
        );
        Ok(Self {
            network_id,
            order_id,
            payer,
            receiver,
            source_asset,
            target_asset,
            source_amount,
            min_output,
            payer_nonce,
            quote,
            salt,
        })
    }

    pub fn authorization_view(self) -> AxisResult<SwapAuthorizationView> {
        Ok(SwapAuthorizationView {
            network_id: self.network_id,
            order_id: self.order_id,
            payer: self.payer,
            receiver: self.receiver,
            source_asset: self.source_asset,
            target_asset: self.target_asset,
            source_amount: self.source_amount,
            min_output: self.min_output,
            payer_nonce: self.payer_nonce,
            quote_digest: self.quote.digest()?,
            salt: self.salt,
        })
    }
}

impl SignedSwap {
    pub fn sign(terms: SwapTerms, key_pair: &KeyPair) -> AxisResult<Self> {
        let payer = key_pair.public_identity();
        if payer.account != terms.payer {
            return Err(AxisError::UnauthorizedSwapSigner {
                expected: terms.payer,
                received: payer.account,
            });
        }
        let signature = key_pair.sign(SWAP_DOMAIN, &terms.authorization_view()?)?;
        Ok(Self {
            payer,
            terms,
            signature,
        })
    }

    pub fn verify(&self) -> AxisResult<()> {
        if self.payer.account != self.terms.payer {
            return Err(AxisError::UnauthorizedSwapSigner {
                expected: self.terms.payer,
                received: self.payer.account,
            });
        }
        verify_signature(
            self.payer,
            self.signature,
            SWAP_DOMAIN,
            &self.terms.authorization_view()?,
        )
    }

    pub fn tx_id(&self) -> AxisResult<TxId> {
        TxId::from_serializable("axis-signed-swap-v1", self)
    }
}

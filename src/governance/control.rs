use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    AccountId, AxisError, AxisResult, Digest, KeyPair, PublicIdentity, SignatureBytes,
    verify_signature,
};

const CONTROL_ACTION_DOMAIN: &str = "axis-control-action-v1";
const CONTROL_VOTE_DOMAIN: &str = "axis-control-vote-v1";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    AssetPolicy,
    OracleCommittee,
    RoutePolicy,
    RiskLimits,
    TreasuryPolicy,
    EmergencyPause,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ControlAction {
    pub kind: ActionKind,
    pub payload_digest: Digest,
    pub proposer: AccountId,
    pub proposal_nonce: u64,
    pub earliest_epoch: u64,
    pub expires_at_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedControlAction {
    pub action: ControlAction,
    pub signer: PublicIdentity,
    pub signature: SignatureBytes,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlVoteKind {
    Approve,
    Cancel,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ControlVote {
    pub action_digest: Digest,
    pub reviewer: AccountId,
    pub reviewer_nonce: u64,
    pub decision: ControlVoteKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SignedControlVote {
    pub vote: ControlVote,
    pub signer: PublicIdentity,
    pub signature: SignatureBytes,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PendingControlAction {
    pub action: ControlAction,
    pub digest: Digest,
    pub approvals: BTreeSet<AccountId>,
    pub cancellations: BTreeSet<AccountId>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ControlVoteOutcome {
    Pending,
    Approved,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
pub struct ControlCommittee {
    quorum: usize,
    reviewers: BTreeMap<AccountId, PublicIdentity>,
    nonces: BTreeMap<AccountId, u64>,
    pending: BTreeMap<Digest, PendingControlAction>,
    approved: BTreeSet<Digest>,
    executed: BTreeSet<Digest>,
}

impl ControlAction {
    pub fn new(
        kind: ActionKind,
        payload_digest: Digest,
        proposer: AccountId,
        proposal_nonce: u64,
        earliest_epoch: u64,
        expires_at_epoch: u64,
    ) -> AxisResult<Self> {
        if earliest_epoch >= expires_at_epoch {
            return Err(AxisError::Policy(
                "control action execution window is invalid".to_owned(),
            ));
        }
        Ok(Self {
            kind,
            payload_digest,
            proposer,
            proposal_nonce,
            earliest_epoch,
            expires_at_epoch,
        })
    }

    pub fn digest(self) -> AxisResult<Digest> {
        Digest::from_serializable(CONTROL_ACTION_DOMAIN, &self)
    }
}

impl SignedControlAction {
    pub fn sign(action: ControlAction, key_pair: &KeyPair) -> AxisResult<Self> {
        let signer = key_pair.public_identity();
        if signer.account != action.proposer {
            return Err(AxisError::Policy(
                "control action signer is not proposer".to_owned(),
            ));
        }
        let signature = key_pair.sign(CONTROL_ACTION_DOMAIN, &action)?;
        Ok(Self {
            action,
            signer,
            signature,
        })
    }

    pub fn verify(&self) -> AxisResult<()> {
        if self.signer.account != self.action.proposer {
            return Err(AxisError::Policy(
                "control action identity mismatch".to_owned(),
            ));
        }
        verify_signature(
            self.signer,
            self.signature,
            CONTROL_ACTION_DOMAIN,
            &self.action,
        )
    }
}

impl SignedControlVote {
    pub fn sign(vote: ControlVote, key_pair: &KeyPair) -> AxisResult<Self> {
        let signer = key_pair.public_identity();
        if signer.account != vote.reviewer {
            return Err(AxisError::Policy(
                "control vote signer is not reviewer".to_owned(),
            ));
        }
        let signature = key_pair.sign(CONTROL_VOTE_DOMAIN, &vote)?;
        Ok(Self {
            vote,
            signer,
            signature,
        })
    }

    pub fn verify(&self) -> AxisResult<()> {
        if self.signer.account != self.vote.reviewer {
            return Err(AxisError::Policy(
                "control vote identity mismatch".to_owned(),
            ));
        }
        verify_signature(self.signer, self.signature, CONTROL_VOTE_DOMAIN, &self.vote)
    }
}

impl ControlCommittee {
    pub fn new(quorum: usize) -> AxisResult<Self> {
        if quorum == 0 {
            return Err(AxisError::Policy(
                "control quorum must be positive".to_owned(),
            ));
        }
        Ok(Self {
            quorum,
            reviewers: BTreeMap::new(),
            nonces: BTreeMap::new(),
            pending: BTreeMap::new(),
            approved: BTreeSet::new(),
            executed: BTreeSet::new(),
        })
    }

    pub fn register_reviewer(&mut self, identity: PublicIdentity) -> AxisResult<()> {
        identity.verify_consistency()?;
        if self.reviewers.insert(identity.account, identity).is_some() {
            return Err(AxisError::Policy(
                "control reviewer already registered".to_owned(),
            ));
        }
        self.nonces.insert(identity.account, 0);
        Ok(())
    }

    pub fn remove_reviewer(&mut self, reviewer: AccountId) -> AxisResult<()> {
        if !self.reviewers.contains_key(&reviewer) {
            return Err(AxisError::Policy(
                "control reviewer is not registered".to_owned(),
            ));
        }
        if self.reviewers.len().saturating_sub(1) < self.quorum {
            return Err(AxisError::Policy(
                "control reviewer removal would violate quorum".to_owned(),
            ));
        }
        if self.pending.values().any(|action| {
            action.approvals.contains(&reviewer) || action.cancellations.contains(&reviewer)
        }) {
            return Err(AxisError::Policy(
                "control reviewer has a vote on a pending action".to_owned(),
            ));
        }
        self.reviewers.remove(&reviewer);
        self.nonces.remove(&reviewer);
        Ok(())
    }

    pub fn set_quorum(&mut self, quorum: usize) -> AxisResult<()> {
        if quorum == 0 || quorum > self.reviewers.len() {
            return Err(AxisError::Policy(
                "control quorum outside reviewer set".to_owned(),
            ));
        }
        self.quorum = quorum;
        Ok(())
    }

    pub fn submit(&mut self, signed: &SignedControlAction) -> AxisResult<Digest> {
        signed.verify()?;
        let proposer = signed.action.proposer;
        self.require_reviewer(proposer)?;
        self.consume_nonce(proposer, signed.action.proposal_nonce)?;
        let digest = signed.action.digest()?;
        if self.pending.contains_key(&digest)
            || self.approved.contains(&digest)
            || self.executed.contains(&digest)
        {
            return Err(AxisError::Policy(
                "control action digest already known".to_owned(),
            ));
        }
        let mut approvals = BTreeSet::new();
        approvals.insert(proposer);
        self.pending.insert(
            digest,
            PendingControlAction {
                action: signed.action,
                digest,
                approvals,
                cancellations: BTreeSet::new(),
            },
        );
        if self.quorum == 1 {
            self.approved.insert(digest);
        }
        Ok(digest)
    }

    pub fn vote(&mut self, signed: &SignedControlVote) -> AxisResult<ControlVoteOutcome> {
        signed.verify()?;
        let reviewer = signed.vote.reviewer;
        self.require_reviewer(reviewer)?;
        self.consume_nonce(reviewer, signed.vote.reviewer_nonce)?;
        let pending = self
            .pending
            .get_mut(&signed.vote.action_digest)
            .ok_or_else(|| AxisError::Policy("control action is not pending".to_owned()))?;
        if pending.approvals.contains(&reviewer) || pending.cancellations.contains(&reviewer) {
            return Err(AxisError::Policy(
                "control reviewer already voted".to_owned(),
            ));
        }
        match signed.vote.decision {
            ControlVoteKind::Approve => {
                pending.approvals.insert(reviewer);
                if pending.approvals.len() >= self.quorum {
                    self.approved.insert(pending.digest);
                    return Ok(ControlVoteOutcome::Approved);
                }
            }
            ControlVoteKind::Cancel => {
                pending.cancellations.insert(reviewer);
                if pending.cancellations.len() >= self.quorum {
                    self.pending.remove(&signed.vote.action_digest);
                    self.approved.remove(&signed.vote.action_digest);
                    return Ok(ControlVoteOutcome::Cancelled);
                }
            }
        }
        Ok(ControlVoteOutcome::Pending)
    }

    pub fn execute(&mut self, digest: Digest, current_epoch: u64) -> AxisResult<ControlAction> {
        if !self.approved.contains(&digest) {
            return Err(AxisError::Policy(
                "control action has not reached quorum".to_owned(),
            ));
        }
        let pending = self
            .pending
            .get(&digest)
            .ok_or_else(|| AxisError::Policy("control action is not pending".to_owned()))?;
        if current_epoch < pending.action.earliest_epoch {
            return Err(AxisError::Policy(
                "control action timelock is active".to_owned(),
            ));
        }
        if current_epoch > pending.action.expires_at_epoch {
            return Err(AxisError::Policy(
                "control action execution window expired".to_owned(),
            ));
        }
        let action = pending.action;
        self.pending.remove(&digest);
        self.approved.remove(&digest);
        self.executed.insert(digest);
        Ok(action)
    }

    pub fn reviewer_nonce(&self, reviewer: AccountId) -> AxisResult<u64> {
        self.nonces
            .get(&reviewer)
            .copied()
            .ok_or_else(|| AxisError::Policy("control reviewer is not registered".to_owned()))
    }

    pub fn pending(&self, digest: Digest) -> Option<&PendingControlAction> {
        self.pending.get(&digest)
    }

    pub fn was_executed(&self, digest: Digest) -> bool {
        self.executed.contains(&digest)
    }

    pub const fn quorum(&self) -> usize {
        self.quorum
    }

    pub fn reviewer_count(&self) -> usize {
        self.reviewers.len()
    }

    fn require_reviewer(&self, reviewer: AccountId) -> AxisResult<()> {
        if self.reviewers.contains_key(&reviewer) {
            Ok(())
        } else {
            Err(AxisError::Policy(
                "control signer is not a reviewer".to_owned(),
            ))
        }
    }

    fn consume_nonce(&mut self, reviewer: AccountId, received: u64) -> AxisResult<()> {
        let expected = self.reviewer_nonce(reviewer)?;
        if received != expected {
            return Err(AxisError::Policy(
                "control reviewer nonce mismatch".to_owned(),
            ));
        }
        self.nonces.insert(
            reviewer,
            expected.checked_add(1).ok_or(AxisError::AmountOverflow)?,
        );
        Ok(())
    }
}

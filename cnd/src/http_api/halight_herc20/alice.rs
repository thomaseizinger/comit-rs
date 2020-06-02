use crate::{
    asset,
    ethereum::{Bytes, ChainId},
    http_api::{
        halight, herc20,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, Halight, Herc20, Ledger, LedgerEvents,
        },
        ActionNotFound, AliceSwap,
    },
    swap_protocols::actions::{ethereum, lnd, lnd::Chain},
    DeployAction, FundAction, InitAction, Never, RedeemAction, RefundAction, SecretHash, Timestamp,
};
use blockchain_contracts::ethereum::rfc003::EtherHtlc;

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Herc20
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Created {
                beta_created: herc20_asset,
                ..
            }
            | AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                beta_finalized: herc20::Finalized { asset: herc20_asset, .. },
                ..
            } => Self {
                protocol: "herc20".to_owned(),
                quantity: herc20_asset.quantity.to_wei_dec(),
                token_contract: herc20_asset.token_contract.to_string(),
            },
        }
    }
}

impl From<AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>>
    for Halight
{
    fn from(
        from: AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>,
    ) -> Self {
        match from {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Created {
                alpha_created: halight_asset,
                ..
            }
            | AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                alpha_finalized: halight::Finalized { asset: halight_asset, .. },
                ..
            } => Self {
                protocol: "halight".to_owned(),
                quantity: halight_asset.as_sat().to_string(),
            },
        }
    }
}

impl BetaParams for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Herc20;
    fn beta_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl BetaEvents for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Created { .. } => None,
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                beta_finalized: herc20::Finalized { state: herc20_state, .. },
                ..
            } => Some(From::<herc20::State>::from(herc20_state.clone())),
        }
    }
}

impl AlphaParams
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Halight;
    fn alpha_params(&self) -> Self::Output {
        self.clone().into()
    }
}

impl AlphaEvents
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_events(&self) -> Option<LedgerEvents> {
        match self {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Created { .. } => None,
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                alpha_finalized: halight::Finalized { state: halight_state, .. },
                ..
            } => Some(From::<halight::State>::from(*halight_state)),
        }
    }
}

impl FundAction for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                alpha_finalized:
                halight::Finalized {
                    state: halight::State::Opened(_),
                    asset: halight_asset,
                    refund_identity: halight_refund_identity,
                    redeem_identity: halight_redeem_identity,
                    cltv_expiry,
                },
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::None,
                        ..
                    },
                secret,
                ..
            } => {
                let to_public_key = *halight_redeem_identity;
                let amount = *halight_asset;
                let secret_hash = SecretHash::new(*secret);
                let final_cltv_delta = *cltv_expiry;
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = *halight_refund_identity;

                Ok(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl RedeemAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> anyhow::Result<Self::Output> {
        match self {
            AliceSwap::<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>::Finalized {
                beta_finalized:
                    herc20::Finalized {
                        state: herc20::State::Funded { htlc_location, .. },
                        ..
                    },
                secret,
                ..
            } => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = None;

                Ok(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => anyhow::bail!(ActionNotFound),
        }
    }
}

impl InitAction for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    type Output = Never;
    fn init_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl DeployAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn deploy_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl RefundAction
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    type Output = Never;
    fn refund_action(&self) -> anyhow::Result<Self::Output> {
        anyhow::bail!(ActionNotFound)
    }
}

impl AlphaLedger
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_ledger(&self) -> Ledger {
        Ledger::Bitcoin
    }
}

impl BetaLedger for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> {
    fn beta_ledger(&self) -> Ledger {
        Ledger::Ethereum
    }
}

impl AlphaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn alpha_absolute_expiry(&self) -> Option<Timestamp> {
        None // No absolute expiry time for halight.
    }
}

impl BetaAbsoluteExpiry
    for AliceSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized>
{
    fn beta_absolute_expiry(&self) -> Option<Timestamp> {
        match self {
            AliceSwap::Created { .. } => None,
            AliceSwap::Finalized {
                beta_finalized: herc20::Finalized { expiry, .. },
                ..
            } => Some(*expiry),
        }
    }
}
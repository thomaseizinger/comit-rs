use crate::{
    btsieve::{EthereumQuery, EventMatcher, Topic},
    swap_protocols::{
        ledger::Ethereum,
        rfc003::{
            ethereum::{REDEEM_LOG_MSG, REFUND_LOG_MSG},
            events::{NewHtlcFundedQuery, NewHtlcRedeemedQuery, NewHtlcRefundedQuery},
            state_machine::HtlcParams,
        },
    },
};
use ethereum_support::{web3::types::Address, EtherQuantity};

impl NewHtlcFundedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_funded_query(htlc_params: &HtlcParams<Ethereum, EtherQuantity>) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(htlc_params.bytecode()),
            transaction_data_length: None,
        }
    }
}

impl NewHtlcRefundedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_refunded_query(
        _htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some(*htlc_location),
                data: None,
                topics: vec![Some(Topic(REFUND_LOG_MSG.into()))],
            }],
        }
    }
}

impl NewHtlcRedeemedQuery<Ethereum, EtherQuantity> for EthereumQuery {
    fn new_htlc_redeemed_query(
        _htlc_params: &HtlcParams<Ethereum, EtherQuantity>,
        htlc_location: &Address,
    ) -> Self {
        EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some(*htlc_location),
                data: None,
                topics: vec![Some(Topic(REDEEM_LOG_MSG.into()))],
            }],
        }
    }
}

pub mod erc20 {
    use super::*;
    use crate::swap_protocols::rfc003::ethereum::TRANSFER_LOG_MSG;
    use ethereum_support::Erc20Token;

    pub fn new_htlc_deployed_query(
        htlc_params: &HtlcParams<Ethereum, Erc20Token>,
    ) -> EthereumQuery {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(htlc_params.bytecode()),
            transaction_data_length: None,
        }
    }

    pub fn new_htlc_funded_query(
        htlc_params: &HtlcParams<Ethereum, Erc20Token>,
        htlc_location: &Address,
    ) -> EthereumQuery {
        EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some(htlc_params.asset.token_contract()),
                data: None,
                topics: vec![
                    Some(Topic(TRANSFER_LOG_MSG.into())),
                    None,
                    Some(Topic(htlc_location.into())),
                ],
            }],
        }
    }

    pub fn new_htlc_refunded_query(htlc_location: &Address) -> EthereumQuery {
        EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some(*htlc_location),
                data: None,
                topics: vec![Some(Topic(REFUND_LOG_MSG.into()))],
            }],
        }
    }

    pub fn new_htlc_redeemed_query(htlc_location: &Address) -> EthereumQuery {
        EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some(*htlc_location),
                data: None,
                topics: vec![Some(Topic(REDEEM_LOG_MSG.into()))],
            }],
        }
    }
}

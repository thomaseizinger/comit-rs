use bitcoin_htlc::Htlc;
use bitcoin_rpc;
use bitcoin_rpc::BlockHeight;
use exchange_api_client::OfferResponseBody;
use secret::Secret;
use std::collections::HashMap;
use std::sync::RwLock;
use symbol::Symbol;
use uuid::Uuid;
use web3::types::Address as EthAddress;

//pub use self::OfferCreated as OfferCreatedState;

// State after exchange has made an offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferCreated {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub rate: f32, // Actually need to specify the exact amounts of each currency
}

impl From<OfferResponseBody> for OfferCreated {
    fn from(offer: OfferResponseBody) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
        }
    }
}

// State after client accepts trade offer
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderCreated {
    pub uid: Uuid,
    pub client_success_address: EthAddress,
    pub client_refund_address: bitcoin_rpc::Address,
    pub secret: Secret,
    pub long_relative_timelock: BlockHeight,
}

#[derive(Clone, Debug)]
pub struct OrderTaken {
    pub uid: Uuid,
    pub exchange_refund_address: EthAddress,
    // This is embedded in the HTLC but we keep it here as well for completeness
    pub exchange_success_address: bitcoin_rpc::Address,
    pub exchange_contract_time_lock: u64,
    pub htlc: Htlc,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDeployed {
    pub uid: Uuid,
    pub address: EthAddress,
}

pub struct EventStore {
    offer_created: RwLock<HashMap<Uuid, OfferCreated>>,
    order_created: RwLock<HashMap<Uuid, OrderCreated>>,
    order_taken: RwLock<HashMap<Uuid, OrderTaken>>,
    contract_deployed: RwLock<HashMap<Uuid, ContractDeployed>>,
}

#[derive(PartialEq)]
enum TradeState {
    NonExistent,
    OfferCreated,
    OrderCreated,
    OrderTaken,
    ContractDeployed,
}

#[derive(Debug)]
pub enum Error {
    IncorrectState,
}

impl EventStore {
    pub fn new() -> EventStore {
        EventStore {
            offer_created: RwLock::new(HashMap::new()),
            order_created: RwLock::new(HashMap::new()),
            order_taken: RwLock::new(HashMap::new()),
            contract_deployed: RwLock::new(HashMap::new()),
        }
    }

    fn current_state(&self, id: &Uuid) -> TradeState {
        if self._get(&self.offer_created, id).is_none() {
            return TradeState::NonExistent;
        }

        if self._get(&self.order_created, id).is_none() {
            return TradeState::OfferCreated;
        }

        if self._get(&self.order_taken, id).is_none() {
            return TradeState::OrderCreated;
        }

        if self._get(&self.contract_deployed, id).is_none() {
            return TradeState::OrderTaken;
        }

        TradeState::ContractDeployed
    }

    fn _store<E: Clone>(
        &self,
        event_map: &RwLock<HashMap<Uuid, E>>,
        required_state: TradeState,
        id: Uuid,
        event: &E,
    ) -> Result<(), Error> {
        if self.current_state(&id) == required_state {
            event_map.write().unwrap().insert(id, event.clone());
            Ok(())
        } else {
            Err(Error::IncorrectState)
        }
    }

    pub fn store_offer_created(&self, event: OfferCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.offer_created, TradeState::NonExistent, uid, &event)
    }

    pub fn store_trade_created(&self, event: OrderCreated) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.order_created, TradeState::OfferCreated, uid, &event)
    }

    pub fn store_trade_accepted(&self, event: OrderTaken) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.order_taken, TradeState::OrderCreated, uid, &event)
    }

    pub fn store_contract_deployed(&self, event: ContractDeployed) -> Result<(), Error> {
        let uid = event.uid.clone();
        self._store(&self.contract_deployed, TradeState::OrderTaken, uid, &event)
    }

    fn _get<E: Clone>(&self, event_map: &RwLock<HashMap<Uuid, E>>, id: &Uuid) -> Option<E> {
        event_map.read().unwrap().get(id).map(Clone::clone)
    }

    pub fn get_offer_created(&self, id: &Uuid) -> Result<OfferCreated, Error> {
        self._get(&self.offer_created, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_order_created(&self, id: &Uuid) -> Result<OrderCreated, Error> {
        self._get(&self.order_created, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_order_taken(&self, id: &Uuid) -> Result<OrderTaken, Error> {
        self._get(&self.order_taken, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }

    pub fn get_contract_deployed(&self, id: &Uuid) -> Result<ContractDeployed, Error> {
        self._get(&self.contract_deployed, id)
            .map_or(Err(Error::IncorrectState), |event| Ok(event.clone()))
    }
}

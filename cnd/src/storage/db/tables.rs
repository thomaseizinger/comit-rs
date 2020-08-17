use crate::{
    asset, bitcoin, halbit, hbit, herc20, identity, lightning,
    storage::db::{
        schema::{halbits, hbits, herc20s, secret_hashes, swap_contexts, swaps},
        wrapper_types::{
            custom_sql_types::{Text, U32},
            BitcoinNetwork, Erc20Amount, Satoshis,
        },
        Sqlite,
    },
    LocalSwapId, LockProtocol, Role, Side,
};
use anyhow::Context;
use chrono::NaiveDateTime;
use diesel::{prelude::*, RunQueryDsl};
use libp2p::PeerId;

#[derive(Identifiable, Queryable, PartialEq, Debug)]
#[table_name = "swaps"]
pub struct Swap {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub counterparty_peer_id: Text<PeerId>,
    pub start_of_swap: NaiveDateTime,
}

impl From<Swap> for InsertableSwap {
    fn from(swap: Swap) -> Self {
        InsertableSwap {
            local_swap_id: swap.local_swap_id,
            role: swap.role,
            counterparty_peer_id: swap.counterparty_peer_id,
            start_of_swap: swap.start_of_swap,
        }
    }
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "swaps"]
pub struct InsertableSwap {
    local_swap_id: Text<LocalSwapId>,
    role: Text<Role>,
    counterparty_peer_id: Text<PeerId>,
    start_of_swap: NaiveDateTime,
}

impl InsertableSwap {
    pub fn new(
        swap_id: LocalSwapId,
        counterparty: PeerId,
        role: Role,
        start_of_swap: NaiveDateTime,
    ) -> Self {
        InsertableSwap {
            local_swap_id: Text(swap_id),
            role: Text(role),
            counterparty_peer_id: Text(counterparty),
            start_of_swap,
        }
    }
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[table_name = "swap_contexts"]
pub struct SwapContext {
    id: i32,
    pub local_swap_id: Text<LocalSwapId>,
    pub role: Text<Role>,
    pub alpha: Text<LockProtocol>,
    pub beta: Text<LockProtocol>,
}

#[derive(Associations, Clone, Copy, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "secret_hashes"]
pub struct SecretHash {
    id: i32,
    swap_id: i32,
    pub secret_hash: Text<comit::SecretHash>,
}

#[derive(Insertable, Debug, Clone, Copy)]
#[table_name = "secret_hashes"]
pub struct InsertableSecretHash {
    swap_id: i32,
    secret_hash: Text<comit::SecretHash>,
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "herc20s"]
pub struct Herc20 {
    id: i32,
    swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub token_contract: Text<identity::Ethereum>,
    pub redeem_identity: Option<Text<identity::Ethereum>>,
    pub refund_identity: Option<Text<identity::Ethereum>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "herc20s"]
pub struct InsertableHerc20 {
    pub swap_id: i32,
    pub amount: Text<Erc20Amount>,
    pub chain_id: U32,
    pub expiry: U32,
    pub token_contract: Text<identity::Ethereum>,
    pub redeem_identity: Option<Text<identity::Ethereum>>,
    pub refund_identity: Option<Text<identity::Ethereum>>,
    pub side: Text<Side>,
}

impl From<Herc20> for asset::Erc20 {
    fn from(herc20: Herc20) -> asset::Erc20 {
        asset::Erc20 {
            quantity: herc20.amount.0.into(),
            token_contract: herc20.token_contract.0,
        }
    }
}

pub trait IntoInsertable {
    type Insertable;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable;
}

pub trait Insert<I> {
    fn insert(&self, connection: &SqliteConnection, insertable: &I) -> anyhow::Result<()>;
}

impl IntoInsertable for herc20::CreatedSwap {
    type Insertable = InsertableHerc20;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable {
        let redeem_identity = match (role, side) {
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => Some(Text(self.identity)),
            _ => None,
        };
        let refund_identity = match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => Some(Text(self.identity)),
            _ => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHerc20 {
            swap_id,
            amount: Text(self.asset.quantity.into()),
            chain_id: U32(self.chain_id.into()),
            expiry: U32(self.absolute_expiry),
            token_contract: Text(self.asset.token_contract),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "halbits"]
pub struct Halbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Debug, Clone)]
#[table_name = "halbits"]
pub struct InsertableHalbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub chain: String,
    pub cltv_expiry: U32,
    pub redeem_identity: Option<Text<lightning::PublicKey>>,
    pub refund_identity: Option<Text<lightning::PublicKey>>,
    pub side: Text<Side>,
}

impl From<Halbit> for asset::Bitcoin {
    fn from(halbit: Halbit) -> Self {
        halbit.amount.0.into()
    }
}

impl IntoInsertable for halbit::CreatedSwap {
    type Insertable = InsertableHalbit;

    fn into_insertable(self, swap_id: i32, role: Role, side: Side) -> Self::Insertable {
        let redeem_identity = match (role, side) {
            (Role::Alice, Side::Beta) | (Role::Bob, Side::Alpha) => Some(Text(self.identity)),
            _ => None,
        };
        let refund_identity = match (role, side) {
            (Role::Alice, Side::Alpha) | (Role::Bob, Side::Beta) => Some(Text(self.identity)),
            _ => None,
        };
        assert!(redeem_identity.is_some() || refund_identity.is_some());

        InsertableHalbit {
            swap_id,
            amount: Text(self.asset.into()),
            network: Text(self.network.into()),
            chain: "bitcoin".to_string(), // We currently only support Lightning on top of Bitcoin.
            cltv_expiry: U32(self.cltv_expiry),
            redeem_identity,
            refund_identity,
            side: Text(side),
        }
    }
}

#[derive(Associations, Clone, Debug, Identifiable, Queryable, PartialEq)]
#[belongs_to(Swap)]
#[table_name = "hbits"]
pub struct Hbit {
    id: i32,
    swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub expiry: U32,
    pub final_identity: Text<bitcoin::Address>,
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

#[derive(Insertable, Clone, Debug)]
#[table_name = "hbits"]
pub struct InsertableHbit {
    pub swap_id: i32,
    pub amount: Text<Satoshis>,
    pub network: Text<BitcoinNetwork>,
    pub expiry: U32,
    pub final_identity: Text<bitcoin::Address>,
    pub transient_identity: Option<Text<bitcoin::PublicKey>>,
    pub side: Text<Side>,
}

impl From<Hbit> for asset::Bitcoin {
    fn from(hbit: Hbit) -> Self {
        hbit.amount.0.into()
    }
}

impl IntoInsertable for hbit::CreatedSwap {
    type Insertable = InsertableHbit;

    fn into_insertable(self, swap_id: i32, _: Role, side: Side) -> Self::Insertable {
        InsertableHbit {
            swap_id,
            amount: Text(self.amount.into()),
            network: Text(self.network.into()),
            expiry: U32(self.absolute_expiry),
            final_identity: Text(self.final_identity.into()),
            // We always retrieve the transient identity from the other party
            transient_identity: None,
            side: Text(side),
        }
    }
}

impl Insert<InsertableHerc20> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHerc20,
    ) -> anyhow::Result<()> {
        diesel::insert_into(herc20s::dsl::herc20s)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

impl Insert<InsertableHalbit> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHalbit,
    ) -> anyhow::Result<()> {
        diesel::insert_into(halbits::dsl::halbits)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

impl Insert<InsertableHbit> for Sqlite {
    fn insert(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableHbit,
    ) -> anyhow::Result<()> {
        diesel::insert_into(hbits::dsl::hbits)
            .values(insertable)
            .execute(connection)?;

        Ok(())
    }
}

macro_rules! swap_id_fk {
    ($local_swap_id:expr) => {
        swaps::table
            .filter(swaps::local_swap_id.eq(Text($local_swap_id)))
            .select(swaps::id)
    };
}

trait EnsureSingleRowAffected {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize>;
}

impl EnsureSingleRowAffected for usize {
    fn ensure_single_row_affected(self) -> anyhow::Result<usize> {
        if self != 1 {
            return Err(anyhow::anyhow!(
                "Expected rows to be updated should have been 1 but was {}",
                self
            ));
        }
        Ok(self)
    }
}

impl Sqlite {
    pub fn save_swap(
        &self,
        connection: &SqliteConnection,
        insertable: &InsertableSwap,
    ) -> anyhow::Result<i32> {
        diesel::insert_into(swaps::dsl::swaps)
            .values(insertable)
            .execute(connection)?;

        let swap_id = swap_id_fk!(insertable.local_swap_id.0).first(connection)?;

        Ok(swap_id)
    }

    pub fn insert_secret_hash(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        secret_hash: comit::SecretHash,
    ) -> anyhow::Result<()> {
        let swap_id = swap_id_fk!(local_swap_id)
            .first(connection)
            .with_context(|| {
                format!(
                    "failed to find swap_id foreign key for swap {}",
                    local_swap_id
                )
            })?;
        let insertable = InsertableSecretHash {
            swap_id,
            secret_hash: Text(secret_hash),
        };

        diesel::insert_into(secret_hashes::table)
            .values(insertable)
            .execute(&*connection)
            .with_context(|| format!("failed to insert secret hash for swap {}", local_swap_id))?;

        Ok(())
    }

    pub fn update_halbit_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halbits::table)
            .filter(halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halbits::refund_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit refund identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_halbit_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Lightning,
    ) -> anyhow::Result<()> {
        diesel::update(halbits::table)
            .filter(halbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(halbits::redeem_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update halbit redeem identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_herc20_refund_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(herc20s::table)
            .filter(herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(herc20s::refund_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update herc20 refund identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_herc20_redeem_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Ethereum,
    ) -> anyhow::Result<()> {
        diesel::update(herc20s::table)
            .filter(herc20s::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(herc20s::redeem_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update herc20 redeem identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }

    pub fn update_hbit_transient_identity(
        &self,
        connection: &SqliteConnection,
        local_swap_id: LocalSwapId,
        identity: identity::Bitcoin,
    ) -> anyhow::Result<()> {
        diesel::update(hbits::table)
            .filter(hbits::swap_id.eq_any(swap_id_fk!(local_swap_id)))
            .set(hbits::transient_identity.eq(Text(identity)))
            .execute(connection)?
            .ensure_single_row_affected()
            .with_context(|| {
                format!(
                    "failed to update hbit transient identity for swap {}",
                    local_swap_id
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proptest::*;
    use proptest::prelude::*;
    use tokio::runtime::Runtime;

    proptest! {
        /// Verify that our database enforces foreign key relations
        ///
        /// We generate a random InsertableHalbit. This comes with a
        /// random swap_id already.
        /// We start with an empty database, so there is no swap that
        /// exists with this swap_id.
        #[test]
        fn fk_relations_are_enforced(
            insertable_halbit in db::tables::insertable_halbit(),
        ) {
            let db = Sqlite::test();
            let mut runtime = Runtime::new().unwrap();

            let result = runtime.block_on(db.do_in_transaction(|conn| db.insert(conn, &insertable_halbit)));

            result.unwrap_err();
        }
    }
}

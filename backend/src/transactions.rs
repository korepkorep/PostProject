
// Copyright 2018 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![ allow( bare_trait_objects ) ]

extern crate serde_json;
extern crate serde;


use serde::{Deserialize, Serialize, Deserializer, Serializer};

use exonum::blockchain::{ExecutionError, ExecutionResult, Transaction};
use exonum::crypto::{CryptoHash, PublicKey, Hash, gen_keypair};
use exonum::messages::Message;
use exonum::storage::Fork;
use exonum::storage::StorageValue;
use exonum::messages::RawMessage;
use exonum::storage::Snapshot;
//use exonum::messages::Message::from_raw;
use exonum::explorer::TransactionInfo;


use CRYPTOCURRENCY_SERVICE_ID;
use schema::CurrencySchema;


/// Error codes emitted by wallet transactions during execution.
#[derive(Debug, Fail)]
#[repr(u8)]
pub enum Error {
    /// Wallet already exists.
    ///
    /// Can be emitted by `CreateWallet`.
    #[fail(display = "Wallet already exists")]
    WalletAlreadyExists = 0,

    /// Sender doesn't exist.
    ///
    /// Can be emitted by `Transfer`.
    #[fail(display = "Sender doesn't exist")]
    SenderNotFound = 1,

    /// Receiver doesn't exist.
    ///
    /// Can be emitted by `Transfer` or `Issue`.
    #[fail(display = "Receiver doesn't exist")]
    ReceiverNotFound = 2,

    /// Insufficient currency amount.
    ///
    /// Can be emitted by `Transfer`.
    #[fail(display = "Insufficient currency amount")]
    InsufficientCurrencyAmount = 3,
}

impl From<Error> for ExecutionError {
    fn from(value: Error) -> ExecutionError {
        let description = format!("{}", value);
        ExecutionError::with_description(value as u8, description)
    }
}

transactions! {
    pub WalletTransactions {
        const SERVICE_ID = CRYPTOCURRENCY_SERVICE_ID;

        /// Transfer `amount` of the currency from one wallet to another.
        struct Transfer {
            from:    &PublicKey,
            to:      &PublicKey,
            amount:  u64,
            seed:    u64,
        }

        /// Issue `amount` of the currency to the `wallet`.
        struct Issue {
            pub_key:  &PublicKey,
            issuer_key: &PublicKey,
            amount:  u64,
            seed:    u64,
        }

        /// Create wallet with the given `name`.
        struct CreateWallet {
            pub_key: &PublicKey,
            name:    &str,
        }

        struct MailPreparation {
            meta: &str,
            pub_key: &PublicKey,
            amount: u64,
            seed: u64,
        }

        struct MailAcceptance {
            sender: &PublicKey,
            pub_key: &PublicKey,
            amount: u64,
            accept:  bool,
            seed: u64,
        }
        
        struct Cancellation {
            pub_key: &PublicKey,
            sender: &PublicKey,
            tx_hash: &Hash,
            type_transaction: u64,
        }
    }
}

impl Transaction for Issue {
    fn verify(&self) -> bool {
        self.verify_signature(self.issuer_key())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema :: new(fork);
        let pub_key = self.pub_key();
        let hash = self.hash();

        if let Some(wallet) = schema.wallet(pub_key) {
            let amount = self.amount();
            schema.increase_wallet_balance(wallet, amount, &hash, 0);
            Ok(())
        } else {
            Err(Error::ReceiverNotFound)?
        }
    }

}


impl Transaction for Transfer {
    fn verify(&self) -> bool {
        (self.from() != self.to()) && self.verify_signature(self.from())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema::new(fork);
        let from = self.from();
        let to = self.to();
        let hash = self.hash();
        let amount = self.amount();
        let freezed_balance = 0;
        let sender = schema.wallet(from).ok_or(Error :: SenderNotFound)?;

        let receiver = schema.wallet(to).ok_or(Error :: ReceiverNotFound)?;

        if sender.balance() < amount {
            Err(Error::InsufficientCurrencyAmount)?;

        }

        schema.decrease_wallet_balance(sender, amount, &hash, freezed_balance);
        schema.increase_wallet_balance(receiver, amount, &hash, freezed_balance);

        Ok(())
    }
}

impl Transaction for CreateWallet {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema::new(fork);
        let pub_key = self.pub_key();
        let hash = self.hash();

        if schema.wallet(pub_key).is_none(){
            let name = self.name();
            let freezed_balance = 0;
            schema.create_wallet(pub_key, name, &hash, freezed_balance);
            Ok(())
        } else {
            Err(Error::WalletAlreadyExists)?
        } 
    }    
}


impl Transaction for MailPreparation {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema :: new(fork);
        let pub_key = self.pub_key();
        let amount = self.amount();
        let hash = self.hash();
        let sender = schema.wallet(pub_key).ok_or(Error :: SenderNotFound)?;
        if sender.balance() < amount {
            Err(Error::InsufficientCurrencyAmount)?;
        }
        // freeze_wallet_balance rrealize
        schema.decrease_wallet_balance(sender, amount, &hash, amount);
        Ok(())
    }
}


impl Transaction for MailAcceptance {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }



    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema :: new(fork);
        let sender_key = self.sender();

        let hash = self.hash();
        let sender = schema.wallet(sender_key).ok_or(Error :: SenderNotFound)?;
        let freezed_balance = 0;
        schema.decrease_wallet_balance(sender, freezed_balance, &hash, freezed_balance);
        Ok(())

    }
}

impl Transaction for Cancellation {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    

    fn execute(&self, fork: &mut Fork) -> ExecutionResult {
        let mut schema = CurrencySchema :: new(fork);
        let sender_key = self.sender();
        let tx_hash = self.tx_hash();
        ///pub fn transaction(schema: &CurrencySchema<T>, tx_hash: &Hash) -> Option<Transaction> {
        let raw_tx = schema.transactions().get(&tx_hash).unwrap();
       //println!("transactions = {:?}", &raw_tx.body());
        //let json = serde_json::to_value(&raw_tx.into_bytes()).unwrap();
        //let info: Transfer = serde_json::from_value(json).unwrap();
        let transaction: Transfer = Message::from_raw(raw_tx.clone()).unwrap();
        //println!("transactions2 = {:?}", StorageValue :: from_bytes(t));

        
        
        /*match raw_tx {
            Some(v) => v,
            None => Err(Error :: SenderNotFound)?,
        };*/
        //assert_eq!(raw_tx, None);

        /*let content: Value = match serde_json::from_slice(&raw_tx.into_bytes()) {
            Ok(r) => r,
            Err(_er) => Err(Error :: ReceiverNotFound)?,
        };
        */
        let id = self.type_transaction();
        if id == 1 { //Transfer
            let from = transaction.from();
            let to = transaction.to();
            let amount = transaction.amount();
            let wallet_from = schema.wallet(&from).ok_or(Error :: SenderNotFound)?;
            let wallet_to = schema.wallet(to).ok_or(Error :: ReceiverNotFound)?;
            schema.decrease_wallet_balance(wallet_to, amount, &tx_hash, 0);
            schema.increase_wallet_balance(wallet_from, amount, &tx_hash, 0);
        }/* else if id == 2 { //issue
            let pub_key = transaction.pub_key();
            let amount = transaction.amount();
            let sender = schema.wallet(pub_key).ok_or(Error :: ReceiverNotFound)?;
            schema.decrease_wallet_balance(sender, amount, &tx_hash, 0);
        }*/
        Ok(())

    }

}
/*
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", bound(serialize = "T: SerializeContent"))]
pub enum TransactionInfo<T = Box<dyn Transaction>> {
    /// Transaction is in the memory pool, but not yet committed to the blockchain.
    InPool {
        /// Transaction contents.
        #[serde(serialize_with = "SerializeContent::serialize_content")]
        content: T,
    },

    /// Transaction is already committed to the blockchain.
    Committed(CommittedTransaction<T>),
}
impl<T> TransactionInfo<T> {
    /// Returns the content of this transaction.
    pub fn content(&self) -> &T {
        match *self {
            TransactionInfo::InPool { ref content } => content,
            TransactionInfo::Committed(ref tx) => tx.content(),
        }
    }

    /// Is this in-pool transaction?
    pub fn is_in_pool(&self) -> bool {
        match *self {
            TransactionInfo::InPool { .. } => true,
            _ => false,
        }
    }

    /// Is this a committed transaction?
    pub fn is_committed(&self) -> bool {
        match *self {
            TransactionInfo::Committed(_) => true,
            _ => false,
        }
    }

    /// Returns a reference to the inner committed transaction if this transaction is committed.
    /// For transactions in pool, returns `None`.
    pub fn as_committed(&self) -> Option<&CommittedTransaction<T>> {
        match *self {
            TransactionInfo::Committed(ref tx) => Some(tx),
            _ => None,
        }
    }
}

pub trait SerializeContent {
    /// Serializes content of a transaction with the given serializer.
    fn serialize_content<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}*/
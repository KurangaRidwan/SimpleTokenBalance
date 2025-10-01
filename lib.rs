// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod simple_token {
    use ink::storage::Mapping;
    use ink::prelude::string::String;

    /// The main storage struct
    #[ink(storage)]
    pub struct SimpleToken {
        /// Account balances
        balances: Mapping<AccountId, u128>,
        /// Owner (only they can mint new tokens)
        owner: AccountId,
    }

    /// Events (like transaction receipts)
    #[ink(event)]
    pub struct Mint {
        #[ink(topic)]
        to: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        amount: u128,
    }

    impl Default for SimpleToken {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SimpleToken {
        /// Constructor: set the contract owner
        #[ink(constructor)]
        pub fn new() -> Self {
            let caller = Self::env().caller();
            Self {
                balances: Mapping::default(),
                owner: caller,
            }
        }

        /// Mint new tokens (only the owner can do this)
        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<(), String> {
            let caller = self.env().caller();
            if caller != self.owner {
                return Err("Only the owner can mint tokens".into());
            }

            let current = self.balances.get(&to).unwrap_or(0);
            let new_balance = current.saturating_add(amount);
            self.balances.insert(&to, &new_balance);

            self.env().emit_event(Mint { to, amount });
            Ok(())
        }

        /// Check balance of an account
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u128 {
            self.balances.get(&owner).unwrap_or(0)
        }

        /// Transfer tokens to another account
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<(), String> {
            let caller = self.env().caller();
            let from_balance = self.balances.get(&caller).unwrap_or(0);

            if from_balance < amount {
                return Err("Not enough balance".into());
            }

            // Update balances
            let updated_from = from_balance.saturating_sub(amount);
            self.balances.insert(&caller, &updated_from);

            let to_balance = self.balances.get(&to).unwrap_or(0);
            let updated_to = to_balance.saturating_add(amount);
            self.balances.insert(&to, &updated_to);

            self.env().emit_event(Transfer {
                from: caller,
                to,
                amount,
            });

            Ok(())
        }
    }
}
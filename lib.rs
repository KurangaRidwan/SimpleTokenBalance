// SPDX-License-Identifier: Apache-2.0
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod simple_token {
    use ink::storage::Mapping;
    use ink::prelude::{vec::Vec, string::String};

    #[ink(storage)]
    pub struct SimpleToken {
        balances: Mapping<AccountId, u128>,
        allowances: Mapping<(AccountId, AccountId), u128>,
        blacklisted: Mapping<AccountId, bool>,
        paused: bool,
        owner: AccountId,
    }

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

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        amount: u128,
    }

    #[ink(event)]
    pub struct Burn {
        #[ink(topic)]
        from: AccountId,
        amount: u128,
    }

    impl Default for SimpleToken {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SimpleToken {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                balances: Mapping::default(),
                allowances: Mapping::default(),
                blacklisted: Mapping::default(),
                paused: false,
                owner: Self::env().caller(),
            }
        }

        #[ink(message)]
        pub fn mint(&mut self, to: AccountId, amount: u128) -> Result<(), String> {
            self.ensure_owner()?;
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(to)?;

            let current = self.balances.get(to).unwrap_or(0);
            let new_balance = current.checked_add(amount).ok_or("Overflow on mint")?;
            self.balances.insert(to, &new_balance);
            self.env().emit_event(Mint { to, amount });
            Ok(())
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u128 {
            self.balances.get(owner).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, amount: u128) -> Result<(), String> {
            let from = self.env().caller();
            self._transfer(from, to, amount)
        }

        #[ink(message)]
        pub fn burn(&mut self, amount: u128) -> Result<(), String> {
            let caller = self.env().caller();
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(caller)?;

            let balance = self.balances.get(caller).unwrap_or(0);
            let new_balance = balance.checked_sub(amount).ok_or("Underflow on burn")?;
            self.balances.insert(caller, &new_balance);
            self.env().emit_event(Burn { from: caller, amount });
            Ok(())
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, amount: u128) -> Result<(), String> {
            let owner = self.env().caller();
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(owner)?;
            self.allowances.insert((owner, spender), &amount);
            self.env().emit_event(Approval { owner, spender, amount });
            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> u128 {
            self.allowances.get((owner, spender)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, amount: u128) -> Result<(), String> {
            let spender = self.env().caller();
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(spender)?;
            self.ensure_not_blacklisted(from)?;
            self.ensure_not_blacklisted(to)?;

            let allowance = self.allowances.get((from, spender)).unwrap_or(0);
            let new_allowance = allowance.checked_sub(amount).ok_or("Allowance underflow")?;
            self.allowances.insert((from, spender), &new_allowance);
            self._transfer(from, to, amount)
        }

        #[ink(message)]
        pub fn pause(&mut self) -> Result<(), String> {
            self.ensure_owner()?;
            self.paused = true;
            Ok(())
        }

        #[ink(message)]
        pub fn unpause(&mut self) -> Result<(), String> {
            self.ensure_owner()?;
            self.paused = false;
            Ok(())
        }

        #[ink(message)]
        pub fn is_paused(&self) -> bool {
            self.paused
        }

        #[ink(message)]
        pub fn blacklist(&mut self, account: AccountId) -> Result<(), String> {
            self.ensure_owner()?;
            self.blacklisted.insert(account, &true);
            Ok(())
        }

        #[ink(message)]
        pub fn unblacklist(&mut self, account: AccountId) -> Result<(), String> {
            self.ensure_owner()?;
            self.blacklisted.insert(account, &false);
            Ok(())
        }

        #[ink(message)]
        pub fn is_blacklisted(&self, account: AccountId) -> bool {
            self.blacklisted.get(account).unwrap_or(false)
        }

        #[ink(message)]
        pub fn batch_transfer(&mut self, recipients: Vec<AccountId>, amounts: Vec<u128>) -> Result<(), String> {
            let sender = self.env().caller();
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(sender)?;

            if recipients.len() != amounts.len() {
                return Err("Mismatched input lengths".into());
            }

            let mut total = 0u128;
            for amount in &amounts {
                total = total.checked_add(*amount).ok_or("Overflow in batch total")?;
            }

            let sender_balance = self.balances.get(sender).unwrap_or(0);
            let new_sender_balance = sender_balance.checked_sub(total).ok_or("Insufficient balance")?;
            self.balances.insert(sender, &new_sender_balance);

            for (i, recipient) in recipients.iter().enumerate() {
                self.ensure_not_blacklisted(*recipient)?;
                let current = self.balances.get(*recipient).unwrap_or(0);
                let updated = current.checked_add(amounts[i]).ok_or("Overflow in recipient balance")?;
                self.balances.insert(*recipient, &updated);
                self.env().emit_event(Transfer {
                    from: sender,
                    to: *recipient,
                    amount: amounts[i],
                });
            }

            Ok(())
        }

        fn _transfer(&mut self, from: AccountId, to: AccountId, amount: u128) -> Result<(), String> {
            self.ensure_not_paused()?;
            self.ensure_not_blacklisted(from)?;
            self.ensure_not_blacklisted(to)?;

            let from_balance = self.balances.get(from).unwrap_or(0);
            let to_balance = self.balances.get(to).unwrap_or(0);

            let new_from = from_balance.checked_sub(amount).ok_or("Insufficient balance")?;
            let new_to = to_balance.checked_add(amount).ok_or("Overflow in recipient balance")?;

            self.balances.insert(from, &new_from);
            self.balances.insert(to, &new_to);

            self.env().emit_event(Transfer { from, to, amount });
            Ok(())
        }

        fn ensure_owner(&self) -> Result<(), String> {
            if self.env().caller() != self.owner {
                return Err("Only owner can call".into());
            }
            Ok(())
        }

        fn ensure_not_paused(&self) -> Result<(), String> {
            if self.paused {
                return Err("Contract is paused".into());
            }
            Ok(())
        }

        fn ensure_not_blacklisted(&self, account: AccountId) -> Result<(), String> {
            if self.blacklisted.get(account).unwrap_or(false) {
                return Err("Account is blacklisted".into());
            }
            Ok(())
        }
    }
}

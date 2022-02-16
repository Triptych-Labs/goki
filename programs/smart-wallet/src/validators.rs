//! Account validators.

use crate::*;
use vipers::{validate::Validate};

impl<'info> Validate<'info> for CreateSmartWallet<'info> {
    fn validate(&self) -> ProgramResult {
        Ok(())
    }
}

impl<'info> Validate<'info> for ExecuteInstructions<'info> {
    fn validate(&self) -> ProgramResult {
       // ensure that the owner is a signer
        // this prevents common frontrunning/flash loan attacks
        self.smart_wallet.owner_index(self.authority_a.key())?;
        self.smart_wallet.owner_index(self.authority_b.key())?;

        Ok(())
    }
}

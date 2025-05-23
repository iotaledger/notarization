use std::marker::PhantomData;

use product_common::transaction::transaction_builder::TransactionBuilder;

use super::notarization::CreateNotarization;
use super::NotarizationMethod;
use crate::core::state::State;
use crate::core::timelock::TimeLock;
use crate::error::Error;

// Method marker types
#[derive(Clone)]
pub struct Locked;
#[derive(Clone)]
pub struct Dynamic;

#[derive(Debug, Clone)]
pub struct NotarizationBuilder<M> {
    pub state: Option<State>,
    pub immutable_description: Option<String>,
    pub updateable_metadata: Option<String>,
    pub delete_lock: Option<TimeLock>,
    pub transfer_lock: Option<TimeLock>,
    pub method: NotarizationMethod,
    _marker: PhantomData<M>,
}

impl NotarizationBuilder<Locked> {
    /// Create locked notarization
    pub fn locked() -> Self {
        Self {
            state: None,
            immutable_description: None,
            updateable_metadata: None,
            delete_lock: None,
            transfer_lock: None,
            method: NotarizationMethod::Locked,
            _marker: PhantomData,
        }
    }

    /// Set delete lock (only available for Locked)
    pub fn with_delete_at(mut self, lock: TimeLock) -> Self {
        self.delete_lock = Some(lock);
        self
    }

    pub fn finish(self) -> Result<TransactionBuilder<CreateNotarization<Locked>>, Error> {
        if self.delete_lock.is_none() {
            return Err(Error::InvalidArgument("Locked needs delete_at()".to_string()));
        }
        Ok(TransactionBuilder::new(CreateNotarization::new(self)))
    }
}

impl NotarizationBuilder<Dynamic> {
    /// Create dynamic notarization
    pub fn dynamic() -> Self {
        Self {
            state: None,
            immutable_description: None,
            updateable_metadata: None,
            delete_lock: None,
            transfer_lock: None,
            method: NotarizationMethod::Dynamic,
            _marker: PhantomData,
        }
    }

    /// Set transfer lock (only available for Dynamic)
    pub fn with_transfer_at(mut self, lock: TimeLock) -> Self {
        self.transfer_lock = Some(lock);
        self
    }

    pub fn finish(self) -> Result<TransactionBuilder<CreateNotarization<Dynamic>>, Error> {
        if self.transfer_lock.is_none() {
            return Err(Error::InvalidArgument("Dynamic needs transfer_at()".to_string()));
        }
        Ok(TransactionBuilder::new(CreateNotarization::new(self)))
    }
}

// Shared methods for both types
impl<M> NotarizationBuilder<M> {
    /// Set state
    pub fn with_state(mut self, state: State) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_bytes_state(self, data: Vec<u8>, metadata: Option<String>) -> Self {
        self.with_state(State::from_bytes(data, metadata))
    }

    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.with_state(State::from_string(data, metadata))
    }

    /// Set description
    pub fn with_immutable_description(mut self, description: String) -> Self {
        self.immutable_description = Some(description);
        self
    }

    /// Set metadata
    pub fn with_updateable_metadata(mut self, metadata: String) -> Self {
        self.updateable_metadata = Some(metadata);
        self
    }
}

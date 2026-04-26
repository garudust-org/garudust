use std::sync::atomic::{AtomicU32, Ordering};

use crate::error::AgentError;

pub struct IterationBudget {
    remaining: AtomicU32,
    max: u32,
}

impl IterationBudget {
    pub fn new(max: u32) -> Self {
        Self {
            remaining: AtomicU32::new(max),
            max,
        }
    }

    pub fn consume(&self) -> Result<u32, AgentError> {
        let prev = self
            .remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |r| {
                if r > 0 {
                    Some(r - 1)
                } else {
                    None
                }
            });
        match prev {
            Ok(r) => Ok(r - 1),
            Err(_) => Err(AgentError::BudgetExhausted(self.max)),
        }
    }

    pub fn remaining(&self) -> u32 {
        self.remaining.load(Ordering::SeqCst)
    }

    pub fn max(&self) -> u32 {
        self.max
    }
}

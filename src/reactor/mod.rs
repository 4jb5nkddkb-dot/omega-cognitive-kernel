#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BinaryHeap;
use core::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorTask {
    pub name: String,
    pub priority_score: i64,
    pub seq: u64,
    pub uncertainty: f64,
    pub risk: f64,
    pub preemptible: bool,
}

impl Eq for ReactorTask {}

impl Ord for ReactorTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority_score
            .cmp(&other.priority_score)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for ReactorTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct PriorityReactor {
    heap: BinaryHeap<ReactorTask>,
    next_seq: u64,
}

impl PriorityReactor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn submit(
        &mut self,
        name: impl Into<String>,
        base_priority: f64,
        uncertainty: f64,
        risk: f64,
        preemptible: bool,
    ) {
        self.next_seq += 1;
        let urgency =
            base_priority + 40.0 * uncertainty.clamp(0.0, 1.0) + 20.0 * risk.clamp(0.0, 2.0);
        self.heap.push(ReactorTask {
            name: name.into(),
            priority_score: (urgency * 1000.0) as i64,
            seq: self.next_seq,
            uncertainty,
            risk,
            preemptible,
        });
    }

    pub fn pop_next(&mut self) -> Option<ReactorTask> {
        self.heap.pop()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn higher_uncertainty_task_preempts_queue() {
        let mut reactor = PriorityReactor::new();
        reactor.submit("routine", 10.0, 0.1, 0.0, true);
        reactor.submit("urgent", 10.0, 0.9, 0.5, true);
         let top = reactor.pop_next().map(|t| t.name);
          assert_eq!(top.as_deref(), Some("urgent"));
    }
}

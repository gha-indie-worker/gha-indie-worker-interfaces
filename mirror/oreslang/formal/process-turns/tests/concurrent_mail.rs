#![forbid(unsafe_code)]

//! Bounded exhaustive abstract model. This verifies the algorithm's state
//! invariants, not the Java/Truffle implementation (which needs executed JVM
//! refinement/conformance tests).
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Stage {
    Queued,
    Running,
    Suspended,
    Done,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Actor {
    phase: [Stage; 3],
    lease: [bool; 3],
    outputs: [u8; 3],
    active: Option<usize>,
    max_inflight: u8,
}

impl Actor {
    fn new(limit: u8) -> Self {
        Self {
            phase: [Stage::Queued; 3],
            lease: [false; 3],
            outputs: [0; 3],
            active: None,
            max_inflight: limit,
        }
    }

    fn inflight(&self) -> usize {
        self.phase.iter().filter(|&&s| {
            matches!(s, Stage::Running | Stage::Suspended)
        }).count()
    }

    fn validate(&self) -> Result<(), &'static str> {
        if self.inflight() > self.max_inflight as usize {
            return Err("message concurrency admission overflow");
        }
        let running: Vec<_> = self.phase.iter().enumerate()
            .filter(|(_, &p)| p == Stage::Running)
            .map(|(i, _)| i).collect();
        if running.len() > 1 || running.first().copied() != self.active {
            return Err("two guest message continuations executing simultaneously");
        }
        for i in 0..3 {
            if self.lease[i] != matches!(self.phase[i], Stage::Running | Stage::Suspended) {
                return Err("mail memory reservation lost or double-released");
            }
            if self.outputs[i] > 2 {
                return Err("unbounded progress outputs");
            }
            if self.outputs[i] > 0 && self.phase[i] == Stage::Queued {
                return Err("response emitted by unadmitted message");
            }
        }
        Ok(())
    }

    fn successors(&self) -> Vec<Self> {
        let mut successors = Vec::new();
        if let Some(i) = self.active {
            let mut awaiting = self.clone();
            awaiting.phase[i] = Stage::Suspended;
            awaiting.active = None;
            successors.push(awaiting);

            let mut completed = self.clone();
            completed.phase[i] = Stage::Done;
            completed.lease[i] = false;
            completed.active = None;
            successors.push(completed);

            if self.outputs[i] < 2 {
                let mut emitted = self.clone();
                emitted.outputs[i] += 1;
                successors.push(emitted);
            }
            return successors;
        }

        for i in 0..3 {
            if self.phase[i] == Stage::Suspended {
                let mut resumed = self.clone();
                resumed.phase[i] = Stage::Running;
                resumed.active = Some(i);
                successors.push(resumed);
            }
            if self.phase[i] == Stage::Queued
                    && self.inflight() < self.max_inflight as usize {
                let mut admitted = self.clone();
                admitted.phase[i] = Stage::Running;
                admitted.lease[i] = true;
                admitted.active = Some(i);
                successors.push(admitted);
            }
        }
        successors
    }
}

fn explore(limit: u8) -> (usize, bool, bool) {
    let first = Actor::new(limit);
    let mut visited = HashSet::from([first.clone()]);
    let mut queue = VecDeque::from([first]);
    let mut interleaved = false;
    let mut all_done = false;

    while let Some(current) = queue.pop_front() {
        current.validate().unwrap();
        interleaved |= current.phase[0] == Stage::Suspended
            && current.phase[1] == Stage::Running;
        all_done |= current.phase.iter().all(|p| *p == Stage::Done);
        for next in current.successors() {
            next.validate().unwrap();
            if visited.insert(next.clone()) {
                queue.push_back(next);
            }
        }
    }
    (visited.len(), interleaved, all_done)
}

#[test]
fn five_is_a_concurrency_limit_not_a_parallelism_limit() {
    let (states, witnessed_interleaving, all_done) = explore(2);
    assert!(states > 50, "state space unexpectedly small");
    assert!(witnessed_interleaving,
        "later mail must advance while another receive is suspended");
    assert!(all_done, "all three distinct messages can finish");
    eprintln!("checked {states} abstract concurrent actor states");
}

#[test]
fn default_single_inflight_remains_serial() {
    let (_, interleaved, completed) = explore(1);
    assert!(!interleaved, "default one-message admission must not interleave");
    assert!(completed);
}

#[test]
fn mutation_oracle_detects_two_guest_turns() {
    let mut invalid = Actor::new(2);
    invalid.phase[0] = Stage::Running;
    invalid.phase[1] = Stage::Running;
    invalid.lease[0] = true;
    invalid.lease[1] = true;
    invalid.active = Some(0);
    assert_eq!(invalid.validate(),
        Err("two guest message continuations executing simultaneously"));
}

#[test]
fn mutation_oracle_detects_lost_message_memory_reservation() {
    let mut invalid = Actor::new(2);
    invalid.phase[0] = Stage::Suspended;
    assert_eq!(invalid.validate(),
        Err("mail memory reservation lost or double-released"));
}

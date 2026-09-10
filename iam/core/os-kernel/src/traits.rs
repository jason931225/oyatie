//! Runnable services and sequence hooks, modeled on the Talos `runtime`
//! controller and service abstractions.

use crate::error::Result;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Lifecycle state of a [`Runnable`]. [`RunState::can_transition_to`] is the
/// state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Initialized,
    Preparing,
    Running,
    Stopped,
    Failed,
}

impl RunState {
    pub fn can_transition_to(self, next: RunState) -> bool {
        use RunState::{Failed, Initialized, Preparing, Running, Stopped};
        matches!(
            (self, next),
            (Initialized, Preparing)
                | (Preparing, Running | Failed)
                | (Running, Stopped | Failed)
                | (Stopped | Failed, Initialized)
        )
    }

    /// Terminal only until a restart: both states can transition back to
    /// `Initialized`.
    pub fn is_terminal(self) -> bool {
        matches!(self, RunState::Stopped | RunState::Failed)
    }
}

/// A long-running component the runtime supervises.
pub trait Runnable {
    /// Stable across restarts, e.g. `"kubelet"`.
    fn id(&self) -> &str;

    /// Implementors MUST make this idempotent: calling it on an
    /// already-running instance succeeds rather than erroring.
    fn start(&mut self) -> Result<()>;

    /// Implementors MUST make this idempotent: calling it on an
    /// already-stopped instance succeeds rather than erroring.
    fn stop(&mut self) -> Result<()>;

    fn state(&self) -> RunState;

    fn is_healthy(&self) -> bool {
        self.state() == RunState::Running
    }
}

/// One phase of a machine-lifecycle sequence such as boot, upgrade or reset.
pub trait SequenceHook {
    fn name(&self) -> &str;

    /// An `Err` aborts the whole sequence; later phases do not run.
    fn run(&mut self) -> Result<()>;

    /// Advisory only: [`Sequence::run_all`] does not consult this, and no
    /// skip-on-rerun path exists yet.
    fn is_idempotent(&self) -> bool {
        false
    }
}

/// Runs [`SequenceHook`] phases in push order, recording those that succeeded.
#[derive(Default)]
pub struct Sequence {
    phases: Vec<Box<dyn SequenceHook>>,
    completed: Vec<String>,
}

impl Sequence {
    pub fn new() -> Self {
        Sequence {
            phases: Vec::new(),
            completed: Vec::new(),
        }
    }

    pub fn push(&mut self, phase: Box<dyn SequenceHook>) {
        self.phases.push(phase);
    }

    pub fn completed(&self) -> &[String] {
        &self.completed
    }

    /// Stops at the first error, leaving [`Sequence::completed`] holding the
    /// phases that had already succeeded.
    pub fn run_all(&mut self) -> Result<()> {
        for phase in &mut self.phases {
            phase.run()?;
            self.completed.push(String::from(phase.name()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    struct DummyService {
        id: String,
        state: RunState,
    }

    impl Runnable for DummyService {
        fn id(&self) -> &str {
            &self.id
        }
        fn start(&mut self) -> Result<()> {
            self.state = RunState::Running;
            Ok(())
        }
        fn stop(&mut self) -> Result<()> {
            self.state = RunState::Stopped;
            Ok(())
        }
        fn state(&self) -> RunState {
            self.state
        }
    }

    #[test]
    fn runnable_lifecycle() {
        let mut s = DummyService {
            id: String::from("kubelet"),
            state: RunState::Initialized,
        };
        assert_eq!(s.id(), "kubelet");
        assert!(!s.is_healthy());
        s.start().unwrap();
        assert!(s.is_healthy());
        s.stop().unwrap();
        assert_eq!(s.state(), RunState::Stopped);
    }

    #[test]
    fn state_transition_rules() {
        assert!(RunState::Initialized.can_transition_to(RunState::Preparing));
        assert!(RunState::Running.can_transition_to(RunState::Failed));
        assert!(RunState::Failed.can_transition_to(RunState::Initialized));
        assert!(!RunState::Initialized.can_transition_to(RunState::Running));
        assert!(!RunState::Stopped.can_transition_to(RunState::Running));
        assert!(RunState::Stopped.is_terminal());
        assert!(!RunState::Running.is_terminal());
    }

    struct OkPhase(String);
    impl SequenceHook for OkPhase {
        fn name(&self) -> &str {
            &self.0
        }
        fn run(&mut self) -> Result<()> {
            Ok(())
        }
    }

    struct FailPhase(String);
    impl SequenceHook for FailPhase {
        fn name(&self) -> &str {
            &self.0
        }
        fn run(&mut self) -> Result<()> {
            Err(Error::invalid_state("phase precondition failed"))
        }
    }

    #[test]
    fn sequence_runs_until_error() {
        let mut seq = Sequence::new();
        seq.push(Box::new(OkPhase(String::from("mount"))));
        seq.push(Box::new(OkPhase(String::from("network"))));
        seq.push(Box::new(FailPhase(String::from("kubelet"))));
        seq.push(Box::new(OkPhase(String::from("never"))));

        let err = seq.run_all().unwrap_err();
        assert_eq!(err.kind(), "invalid_state");
        assert_eq!(
            seq.completed(),
            &[String::from("mount"), String::from("network")]
        );
    }

    #[test]
    fn sequence_all_ok() {
        let mut seq = Sequence::new();
        seq.push(Box::new(OkPhase(String::from("a"))));
        seq.push(Box::new(OkPhase(String::from("b"))));
        assert!(seq.run_all().is_ok());
        assert_eq!(seq.completed().len(), 2);
    }
}

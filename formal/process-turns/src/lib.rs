#![forbid(unsafe_code)]

//! Bounded exhaustive *abstract* model of per-process guest-turn admission.
//! This does not prove that the Java/Truffle runtime refines this model.

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum Turn { Main, TimerCallback }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum Main { Queued, Running, Suspended, Done }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum Wait { None, External, Timer }

    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    enum Timer { Absent, Armed, Expired, Cancelled }

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct Process {
        alive: bool,
        ready: Vec<Turn>,
        running: Option<Turn>,
        call_depth: u8,
        active_guest_stacks: u8,
        main: Main,
        waiting: Wait,
        timer: Timer,
        timer_signal: bool,
        timer_callbacks_completed: u8,
        stop_requested: bool,
        late_completion_dropped: bool,
    }

    impl Process {
        fn new() -> Self {
            Self {
                alive: true,
                ready: vec![Turn::Main],
                running: None,
                call_depth: 0,
                active_guest_stacks: 0,
                main: Main::Queued,
                waiting: Wait::None,
                timer: Timer::Absent,
                timer_signal: false,
                timer_callbacks_completed: 0,
                stop_requested: false,
                late_completion_dropped: false,
            }
        }

        fn exit(&mut self) {
            self.alive = false;
            self.ready.clear();
            self.running = None;
            self.call_depth = 0;
            self.active_guest_stacks = 0;
            self.main = Main::Done;
            self.waiting = Wait::None;
            self.timer_signal = false;
            self.stop_requested = false;
            if self.timer == Timer::Armed {
                self.timer = Timer::Cancelled;
            }
        }

        fn valid(&self) -> Result<(), &'static str> {
            if self.active_guest_stacks > 1 {
                return Err("more than one independent user stack in a process");
            }
            if self.running.is_some() != (self.active_guest_stacks == 1) {
                return Err("guest stack and running-turn ownership disagree");
            }
            if self.running.is_some() != (self.call_depth > 0) {
                return Err("nested-call depth and guest-turn ownership disagree");
            }
            if self.call_depth > 2 {
                return Err("model nesting bound exceeded");
            }
            if self.ready.len() > 3 {
                return Err("bounded ready-queue capacity exceeded");
            }
            if self.timer_callbacks_completed > 1 {
                return Err("one-shot timer callback executed more than once");
            }
            if !self.alive
                && (!self.ready.is_empty()
                    || self.running.is_some()
                    || self.waiting != Wait::None
                    || self.timer == Timer::Armed
                    || self.timer_signal)
            {
                return Err("terminated process retained runnable user work");
            }
            if self.main == Main::Suspended && self.waiting == Wait::None {
                return Err("suspended main lost its continuation dependency");
            }
            if self.waiting != Wait::None && self.main != Main::Suspended {
                return Err("await registered without suspended main");
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct World {
        // ROOT and one independent DADDY process. Both may run simultaneously.
        processes: [Process; 2],
    }

    impl World {
        fn new() -> Self {
            Self { processes: [Process::new(), Process::new()] }
        }

        fn valid(&self) -> Result<(), &'static str> {
            for p in &self.processes { p.valid()?; }
            Ok(())
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum Action {
        AdmitGuestTurn,
        CallNested,
        ReturnNested,
        RegisterTimer,
        AwaitExternal,
        AwaitTimer,
        CompleteExternal,
        ExpireTimer,
        ReturnMain,
        ReturnTimerCallback,
        RequestStop,
        ObserveStopAtSafepoint,
        LateCompletion,
    }

    const ACTIONS: [Action; 13] = [
        Action::AdmitGuestTurn,
        Action::CallNested,
        Action::ReturnNested,
        Action::RegisterTimer,
        Action::AwaitExternal,
        Action::AwaitTimer,
        Action::CompleteExternal,
        Action::ExpireTimer,
        Action::ReturnMain,
        Action::ReturnTimerCallback,
        Action::RequestStop,
        Action::ObserveStopAtSafepoint,
        Action::LateCompletion,
    ];

    // An action is atomic at a process-lane linearization boundary.
    // Timer/producer actions only change readiness; only AdmitGuestTurn
    // transitions from zero to one executing guest stack.
    fn transition(world: &World, index: usize, action: Action) -> Option<World> {
        let mut next = world.clone();
        let p = &mut next.processes[index];
        match action {
            Action::AdmitGuestTurn if p.alive && p.running.is_none() && !p.ready.is_empty() => {
                let turn = p.ready.remove(0);
                p.running = Some(turn);
                p.active_guest_stacks += 1;
                p.call_depth = 1;
                if turn == Turn::Main {
                    p.main = Main::Running;
                    p.waiting = Wait::None;
                }
            }
            Action::CallNested if p.alive && p.running.is_some() && p.call_depth == 1 => {
                // Ordinary inner function calls are part of ONE stack.
                p.call_depth += 1;
            }
            Action::ReturnNested if p.running.is_some() && p.call_depth == 2 => {
                p.call_depth -= 1;
            }
            Action::RegisterTimer if p.running == Some(Turn::Main)
                && p.timer == Timer::Absent && p.alive => {
                p.timer = Timer::Armed;
            }
            Action::AwaitExternal if p.running == Some(Turn::Main)
                && p.call_depth == 1 && p.alive => {
                p.running = None;
                p.active_guest_stacks = 0;
                p.call_depth = 0;
                p.main = Main::Suspended;
                p.waiting = Wait::External;
            }
            Action::AwaitTimer if p.running == Some(Turn::Main)
                && p.call_depth == 1 && p.alive
                && p.timer != Timer::Absent && p.timer != Timer::Cancelled => {
                if p.timer_signal {
                    // select/readch claims a ready signal without suspending.
                    p.timer_signal = false;
                } else {
                    p.running = None;
                    p.active_guest_stacks = 0;
                    p.call_depth = 0;
                    p.main = Main::Suspended;
                    p.waiting = Wait::Timer;
                }
            }
            Action::CompleteExternal if p.alive && p.waiting == Wait::External => {
                p.waiting = Wait::None;
                p.main = Main::Queued;
                p.ready.push(Turn::Main); // Never execute on producer thread.
            }
            Action::ExpireTimer if p.alive && p.timer == Timer::Armed => {
                p.timer = Timer::Expired;
                p.timer_signal = true;
                p.ready.push(Turn::TimerCallback); // No inline guest callback.
                if p.waiting == Wait::Timer {
                    p.waiting = Wait::None;
                    p.main = Main::Queued;
                    p.ready.push(Turn::Main); // select is ready.
                }
            }
            Action::ReturnMain if p.alive && p.running == Some(Turn::Main)
                && p.call_depth == 1 => {
                // A plain unawaited timer is not a process-lifetime root.
                p.exit();
            }
            Action::ReturnTimerCallback if p.alive
                && p.running == Some(Turn::TimerCallback)
                && p.call_depth == 1 => {
                p.timer_callbacks_completed += 1;
                p.active_guest_stacks = 0;
                p.running = None;
                p.call_depth = 0;
                if p.stop_requested { p.exit(); }
            }
            Action::RequestStop if p.alive && !p.stop_requested => {
                if p.running.is_some() {
                    // Cooperative trusted code checks cancellation at safepoint.
                    p.stop_requested = true;
                } else {
                    p.exit();
                }
            }
            Action::ObserveStopAtSafepoint if p.alive && p.stop_requested
                && p.running.is_some() => {
                p.exit();
            }
            Action::LateCompletion if !p.alive && !p.late_completion_dropped => {
                // Late timer/Future wake cannot revive process-owned code.
                p.late_completion_dropped = true;
            }
            _ => return None,
        }
        Some(next)
    }

    #[test]
    fn exhaustive_two_process_safety_and_lifecycle() {
        let initial = World::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([initial.clone()]);
        visited.insert(initial);

        let mut both_processes_running = false;
        let mut timer_callback_during_main_await = false;
        let mut cancelled_unawaited_timer = false;
        let mut late_wake_was_dropped = false;
        let mut timer_select_was_resumed = false;

        while let Some(world) = queue.pop_front() {
            world.valid().unwrap_or_else(|why| panic!("{why}: {world:?}"));
            let [root, daddy] = &world.processes;
            both_processes_running |= root.active_guest_stacks == 1
                && daddy.active_guest_stacks == 1;
            for p in &world.processes {
                timer_callback_during_main_await |=
                    p.running == Some(Turn::TimerCallback)
                    && p.main == Main::Suspended;
                cancelled_unawaited_timer |= !p.alive && p.timer == Timer::Cancelled;
                late_wake_was_dropped |= p.late_completion_dropped;
                timer_select_was_resumed |=
                    p.main == Main::Queued && p.timer == Timer::Expired
                    && p.ready.contains(&Turn::Main)
                    && p.ready.contains(&Turn::TimerCallback);
            }

            for index in 0..2 {
                for action in ACTIONS {
                    if let Some(next) = transition(&world, index, action) {
                        next.valid().unwrap_or_else(|why|
                            panic!("{why}; action={action:?}; from={world:?}; to={next:?}"));
                        if visited.insert(next.clone()) {
                            assert!(visited.len() <= 2_000_000,
                                "raise model bounds only after reviewing state explosion");
                            queue.push_back(next);
                        }
                    }
                }
            }
        }

        assert!(visited.len() > 100, "model exploration unexpectedly trivial");
        assert!(both_processes_running, "sibling process parallelism unreachable");
        assert!(timer_callback_during_main_await,
            "timer callback must be able to run while main is suspended");
        assert!(cancelled_unawaited_timer,
            "returning main must cancel its pending unawaited timer");
        assert!(late_wake_was_dropped, "late completions must not resurrect code");
        assert!(timer_select_was_resumed, "timer readiness must wake a select");
        eprintln!("verified {} abstract interleaving states", visited.len());
    }

    #[test]
    fn mutation_oracle_rejects_inline_timer_guest_execution() {
        let mut bad = Process::new();
        bad.ready.clear();
        bad.running = Some(Turn::Main);
        bad.main = Main::Running;
        bad.timer = Timer::Armed;
        bad.active_guest_stacks = 1;
        bad.call_depth = 1;

        // Incorrect implementation: a CONTROL/timer worker invokes a user
        // callback directly while main is already executing.
        bad.timer = Timer::Expired;
        bad.active_guest_stacks += 1;
        assert_eq!(
            bad.valid(),
            Err("more than one independent user stack in a process"),
        );
    }
}

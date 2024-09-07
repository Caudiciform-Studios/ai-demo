use crate::{Memory, StateEnum};
use bindings::Command;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq)]
pub enum Transition {
    Done,
    Sibling(StateEnum),
    Child(StateEnum),
    Nothing,
}

#[enum_dispatch(StateEnum)]
pub trait State:
    std::fmt::Debug + serde_traitobject::Serialize + serde_traitobject::Deserialize
{
    fn transition(&mut self, _memory: &mut Memory, children: &[StateEnum]) -> Transition {
        Transition::Nothing
    }
    fn run(&mut self, _memory: &mut Memory) -> Command {
        Command::Nothing
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StateStack {
    pub states: Vec<StateEnum>,
}

impl StateStack {
    pub fn transition(&mut self, memory: &mut Memory) {
        let mut i = 0;
        while i < self.states.len() {
            let (a, b) = self.states.split_at_mut(i);
            let transition = b[0].transition(memory, a);
            match transition {
                Transition::Done => {
                    for _ in 0..i + 1 {
                        self.states.remove(0);
                    }
                    i = 0;
                }
                Transition::Sibling(state) => {
                    self.states[i] = state;
                    for _ in 0..i {
                        self.states.remove(0);
                    }
                    i = 1;
                }
                Transition::Child(state) => {
                    for _ in 0..i {
                        self.states.remove(0);
                    }
                    self.states.insert(0, state);
                    i = 1;
                }
                Transition::Nothing => {
                    i += 1;
                }
            }
        }
    }

    pub fn run(&mut self, memory: &mut Memory) -> Command {
        if let Some(state) = self.states.first_mut() {
            state.run(memory)
        } else {
            Command::Nothing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize)]
    struct Extend;
    #[typetag::serde]
    impl State for Extend {
        fn transition(&mut self, _memory: &mut Memory, child: Option<&dyn State>) -> Transition {
            Transition::Child(Box::new(Extend))
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Replace;
    #[typetag::serde]
    impl State for Replace {
        fn transition(&mut self, _memory: &mut Memory, child: Option<&dyn State>) -> Transition {
            Transition::Sibling(Box::new(Replace))
        }
    }

    #[derive(Serialize, Deserialize)]
    struct Nop;
    #[typetag::serde]
    impl State for Nop {
        fn transition(&mut self, _memory: &mut Memory, child: Option<&dyn State>) -> Transition {
            Transition::Nothing
        }
    }

    #[derive(Serialize, Deserialize)]
    struct CountDown(u32);

    #[typetag::serde]
    impl State for CountDown {
        fn transition(&mut self, _memory: &mut Memory, child: Option<&dyn State>) -> Transition {
            if self.0 == 0 {
                Transition::Sibling(Box::new(Nop))
            } else {
                self.0 -= 1;
                Transition::Nothing
            }
        }
    }

    #[test]
    fn test_child() {
        let mut stack = StateStack::default();
        stack.states.push(Box::new(Extend));

        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 2);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 2);
    }

    #[test]
    fn test_replace() {
        let mut stack = StateStack::default();
        stack.states.push(Box::new(Replace));

        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 1);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 1);
    }

    #[test]
    fn test_truncate() {
        let mut stack = StateStack::default();
        stack.states.push(Box::new(Nop));
        stack.states.push(Box::new(Nop));
        stack.states.push(Box::new(Replace));

        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 1);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 1);
    }

    #[test]
    fn test_mutate() {
        let mut stack = StateStack::default();
        stack.states.push(Box::new(Nop));
        stack.states.push(Box::new(CountDown(3)));

        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 2);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 2);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 2);
        stack.transition(&mut Memory::default());
        assert_eq!(stack.states.len(), 1);
    }
}

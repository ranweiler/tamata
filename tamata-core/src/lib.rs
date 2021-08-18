use std::fmt::Debug;

pub trait Fsm: Debug {
    // Macro-generated.
    type State: Debug;
    type Event: Debug;

    // User-provided.
    type Context: Send + Sync;
    type Error: Send + Sync;
}

pub trait State<F>
where
    F: Fsm,
{}

pub trait Event<F>
where
    F: Fsm,
{}

pub trait Transition<F, E>
where
    F: Fsm,
    E: Event<F>,
    Self: State<F>,
{
    type Next: State<F>;

    fn send(self, event: E, ctx: F::Context) -> Result<Self::Next, F::Error>;
}

/// State after sending an event, and whether it is the result of a valid transition.
///
/// `Invalid` values can be converted to errors via `Sent::try_valid()`.
#[derive(Debug)]
pub enum Sent<F>
where
    F: Fsm,
{
    /// The next state was the result of a valid state transition.
    ///
    /// It may be identical to the previous state.
    Valid(F::State),

    /// The sent event was invalid, and the state is unchanged.
    Invalid(F::State, F::Event),
}

impl<F> Sent<F>
where
    F: Fsm,
{
    /// Convert into current state, ignoring whether it resulted from a valid transition.
    pub fn state(self) -> F::State {
        match self {
            Sent::Valid(state) => state,
            Sent::Invalid(state, _) => state,
        }
    }

    /// Convert `self` to a `Result`, where invalid transitions are errors.
    pub fn try_valid(self) -> Result<F::State, Invalid<F>> {
        match self {
            Sent::Valid(state) => Ok(state),
            Sent::Invalid(state, event) => {
                let err = Invalid { state, event };
                Err(err)
            },
        }
    }
}

/// Promotion of an invalid state transition to an error.
#[derive(Debug, thiserror::Error)]
#[error("invalid event for state: state = {state:?}, event = {event:?}")]
pub struct Invalid<F: Fsm> {
    /// FSM state when the event was sent.
    pub state: F::State,

    /// Event sent to the FSM.
    pub event: F::Event,
}

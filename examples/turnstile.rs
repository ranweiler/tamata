use anyhow::{Error, Result};
use tamata::Transition;

fn main() -> Result<()> {
    let mut t = Turnstile::default();

    t.state = t.state.send(Push, ())?.state();
    t.state = t.state.send(Coin, ())?.state();
    t.state = t.state.send(Coin, ())?.state();
    t.state = t.state.send(Push, ())?.state();
    t.state = t.state.send(Push, ())?.state();

    Ok(())
}

tamata::fsm! {
    Turnstile,
    Error = Error,
    Context = (),
    {
        Locked(Coin) -> Unlocked,
        Locked(Push) -> Locked,
        Unlocked(Coin) -> Unlocked,
        Unlocked(Push) -> Locked,
    }
}

#[derive(Debug, Default)]
pub struct Turnstile {
    coins: Vec<Coin>,
    state: TurnstileState,
}

impl Default for TurnstileState {
    fn default() -> Self {
        TurnstileState::Locked(Locked)
    }
}

#[derive(Debug)]
pub struct Locked;

impl Transition<Turnstile, Coin> for Locked {
    type Next = Unlocked;

    fn send(self, _coin: Coin, _ctx: ()) -> Result<Unlocked> {
        println!("unlocking turnstile!");

        Ok(Unlocked)
    }
}

impl Transition<Turnstile, Push> for Locked {
    type Next = Locked;

    fn send(self, _push: Push, _ctx: ()) -> Result<Locked> {
        println!("won't budge.");

        Ok(Locked)
    }
}

#[derive(Debug)]
pub struct Unlocked;

impl Transition<Turnstile, Coin> for Unlocked {
    type Next = Unlocked;

    fn send(self, _coin: Coin, _ctx: ()) -> Result<Unlocked> {
        println!("wasted a coin.");

        Ok(Unlocked)
    }
}

impl Transition<Turnstile, Push> for Unlocked {
    type Next = Locked;

    fn send(self, _push: Push, _ctx: ()) -> Result<Locked> {
        println!("locking behind you!");

        Ok(Locked)
    }
}

#[derive(Debug)]
pub struct Coin;

#[derive(Debug)]
pub struct Push;

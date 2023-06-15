#![no_std]
use gstd::{exec, debug,msg, prelude::*, ActorId};

#[derive(Encode, Decode, TypeInfo)]
pub enum CarAction {
    YourTurn(BTreeMap<ActorId, Car>),
}
#[derive(Encode, Decode, TypeInfo, Clone)]
pub struct Car {
    pub balance: u32,
    pub position: u32,
    pub speed: u32,
    pub penalty: u8,
}

#[derive(Encode, Decode, TypeInfo)]
pub enum StrategyAction {
    BuyAcceleration { amount: u32 },
    BuyShell { amount: u32 },
    Skip,
}

#[no_mangle]
extern "C" fn handle() {
    let shell = get_random_value(10);
    msg::reply(
        StrategyAction::BuyShell {
            amount: shell.into(),
        },
        0,
    )
    .expect("Error in sending a message");
}

static mut SEED: u8 = 0;

pub fn get_random_value(range: u8) -> u8 {
    let seed = unsafe { SEED };
    unsafe { SEED = SEED.wrapping_add(1) };
    let mut random_input: [u8; 32] = exec::program_id().into();
    random_input[0] = random_input[0].wrapping_add(seed);
    let (random, _) = exec::random(random_input).expect("Error in getting random number");
    1 + (random[0] % range)
}

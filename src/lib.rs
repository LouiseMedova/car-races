#![no_std]
use gstd::{debug, exec, msg, prelude::*, ActorId, MessageId, ReservationId};

// prices for acceleration and shuffles are constants for simple implementation
pub const ACCELERATION_COST: u32 = 10;
pub const SHELL_COST: u32 = 20;

pub const MAX_ACC_AMOUNT: u32 = 25;
pub const MAX_SHELL_AMOUNT: u32 = 10;
pub const MAX_DISTANCE: u32 = 10_000;
pub const TIME: u32 = 1;

pub const GAS_FOR_STRATEGY: u64 = 20_000_000_000;
pub const RESERVATION_AMOUNT: u64 = 240_000_000_000;
pub const RESERVATION_TIME: u32 = 86_400;
pub const GAS_MIN_AMOUNT: u64 = 30_000_000_000;

static mut GAME: Option<Game> = None;

#[derive(Encode, Decode, TypeInfo, Clone, Debug)]
pub struct Car {
    pub balance: u32,
    pub position: u32,
    pub speed: u32,
    pub penalty: u8,
}

#[derive(Encode, Decode, TypeInfo, Default, PartialEq, Eq, Debug)]
pub enum GameState {
    #[default]
    Registration,
    ReadyToStart,
    Race,
    Stopped,
    Finished,
}

#[derive(Encode, Decode, TypeInfo, Debug)]
pub struct GameInfo {
    pub round: u32,
    pub cars: BTreeMap<ActorId, Car>,
}
#[derive(Encode, Decode, TypeInfo, Default)]
pub struct Game {
    pub admin: ActorId,
    pub cars: BTreeMap<ActorId, Car>,
    pub car_ids: Vec<ActorId>,
    pub current_turn: u8,
    pub awaiting_reply_to_msg_id: MessageId,
    pub state: GameState,
    pub winner: ActorId,
    pub current_round: u32,
    pub reservations: Vec<ReservationId>,
}

#[derive(Encode, Decode, TypeInfo)]
pub enum GameAction {
    Register { car_id: ActorId },
    StartGame,
    ContinueGame,
    Play,
    MakeReservation,
}

#[derive(Encode, Decode, TypeInfo, Debug)]
pub enum StrategyAction {
    BuyAcceleration { amount: u32 },
    BuyShell { amount: u32 },
    Skip,
}
#[derive(Encode, Decode, TypeInfo)]
pub enum CarAction {
    YourTurn(BTreeMap<ActorId, Car>),
}

#[derive(Encode, Decode, TypeInfo)]
pub enum GameReply {
    Registered,
    NotEnoughGas,
    GameFinished,
    GasReserved,
}

impl Game {
    fn register(&mut self, car_id: ActorId) {
        assert_eq!(self.state, GameState::Registration, "Wrong state");
        self.cars.insert(
            car_id,
            Car {
                balance: 15_000,
                position: 0,
                speed: 100,
                penalty: 0,
            },
        );
        self.car_ids.push(car_id);
        if self.car_ids.len() == 3 {
            self.state = GameState::ReadyToStart;
        }
        msg::reply(GameReply::Registered, 0).expect("Error during reply");
    }

    fn start_game(&mut self) {
        assert_eq!(self.state, GameState::ReadyToStart, "Wrong state");
        self.state = GameState::Race;
        msg::send(exec::program_id(), GameAction::Play, 0).expect("Error in sending a message");
    }

    fn continue_game(&mut self) {
        assert_eq!(self.state, GameState::Stopped, "Wrong state");
        self.state = GameState::Race;
        msg::send(exec::program_id(), GameAction::Play, 0).expect("Error in sending a message");
    }

    fn play(&mut self) {
        if self.state == GameState::Finished {
            msg::reply(GameReply::GameFinished, 0).expect("Error in sending a reply");
            return;
        }
        let car_id = self.get_current_car_id();
        self.awaiting_reply_to_msg_id = msg::send_with_gas(
            car_id,
            CarAction::YourTurn(self.cars.clone()),
            GAS_FOR_STRATEGY,
            0,
        )
        .expect("Error in sending a message");
    }

    fn buy_acceleration(&mut self, amount: u32) {
        let car_id = self.get_current_car_id();
        let car = self.cars.get_mut(&car_id).expect("Get Car: Can't be None");

        if amount > MAX_ACC_AMOUNT {
            car.penalty += 1;
            return;
        };

        let cost_for_amount = ACCELERATION_COST * amount;

        if cost_for_amount > car.balance {
            car.penalty += 1;
            return;
        };

        car.balance -= cost_for_amount;
        car.speed += amount;
    }

    fn buy_shell(&mut self, amount: u32) {
        let car_id = self.get_current_car_id();

        let car = self.cars.get_mut(&car_id).expect("Get Car: Can't be None");

        if amount > MAX_SHELL_AMOUNT {
            car.penalty += 1;
            return;
        };

        let cost_for_amount = SHELL_COST * amount;

        if cost_for_amount > car.balance {
            car.penalty += 1;
            return;
        };

        car.balance -= cost_for_amount;
        let car_position = car.position.clone();
        let closest_car_id = self.find_closest_car(car_id, car_position);

        self.cars
            .entry(closest_car_id)
            .and_modify(|car| car.speed = car.speed.saturating_sub(amount));
    }

    fn get_current_car_id(&self) -> ActorId {
        self.car_ids[self.current_turn as usize]
    }

    fn find_closest_car(&self, current_car_id: ActorId, position: u32) -> ActorId {
        let mut closest_car_id = ActorId::zero();
        let mut distance = MAX_DISTANCE;
        for (car_id, car) in self.cars.iter() {
            if *car_id != current_car_id {
                let new_distance = if position >= car.position {
                    position - car.position
                } else {
                    car.position - position
                };
                if new_distance <= distance {
                    distance = new_distance;
                    closest_car_id = *car_id;
                }
            }
        }
        closest_car_id
    }

    fn check_for_penalties(&mut self) {
        for (car_id, car) in self.cars.iter() {
            if car.penalty >= 5 {
                self.car_ids.retain(|id| id != car_id);
            }
        }
    }

    fn update_positions(&mut self) {
        for (car_id, car) in self.cars.iter_mut() {
            if car.penalty < 5 {
                car.position = car.position + car.speed * TIME;
                if car.position >= MAX_DISTANCE {
                    car.position = MAX_DISTANCE;
                    self.winner = *car_id;
                    self.state = GameState::Finished;
                }
            }
        }
    }

    fn reserve_gas(&mut self) {
        let reservation_id = ReservationId::reserve(RESERVATION_AMOUNT, RESERVATION_TIME)
            .expect("reservation across executions");
        self.reservations.push(reservation_id);

        msg::reply(GameReply::GasReserved, 0).expect("Error in reply");
    }
}
#[no_mangle]
extern "C" fn handle() {
    let action: GameAction = msg::load().expect("Unable to decode the message");
    let game = unsafe { GAME.as_mut().expect("The game is not initialized") };
    match action {
        GameAction::Register { car_id } => game.register(car_id),
        GameAction::StartGame => game.start_game(),
        GameAction::ContinueGame => game.continue_game(),
        GameAction::Play => game.play(),
        GameAction::MakeReservation => game.reserve_gas(),
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    let reply_to = msg::reply_to().expect("Unable to get the msg id");
    let game = unsafe { GAME.as_mut().expect("The game is not initialized") };
    if reply_to != game.awaiting_reply_to_msg_id {
        // unexpected behaviour
        game.state = GameState::Stopped;
        return;
    }
    let bytes = msg::load_bytes().expect("Unable to load bytes");
    // car eliminated from race for wrong payload
    if let Ok(strategy) = StrategyAction::decode(&mut &bytes[..]) {
        match strategy {
            StrategyAction::BuyAcceleration { amount } => {
                game.buy_acceleration(amount);
            }
            StrategyAction::BuyShell { amount } => {
                game.buy_shell(amount);
            }
            StrategyAction::Skip => {}
        }
    } else {
        // car eliminated from race for wrong payload
        let current_car_id = game.get_current_car_id();
        game.cars
            .entry(current_car_id)
            .and_modify(|car| car.penalty = 5);
        game.car_ids.retain(|car_id| *car_id != current_car_id);
    }
    let num_of_cars = game.car_ids.len() as u8;
    game.current_turn = (game.current_turn + 1) % num_of_cars;

    // if one round is made, then we update the positions of the cars
    // and send a message about the new position of the fields
    if game.current_turn == 0 {
        game.check_for_penalties();
        game.update_positions();
        msg::send(
            game.admin,
            GameInfo {
                round: game.current_round,
                cars: game.cars.clone(),
            },
            0,
        )
        .expect("Error in sending a message");
        game.current_round = game.current_round.saturating_add(1);
    }

    // check the gas
    if exec::gas_available() <= GAS_MIN_AMOUNT {
        if let Some(id) = game.reservations.pop() {
            msg::send_from_reservation(id, exec::program_id(), GameAction::Play, 0)
                .expect("Failed to send message");
        } else {
            game.state = GameState::Stopped;
        };
    } else {
        msg::send(exec::program_id(), GameAction::Play, 0).expect("Error in sending a msg");
    }
}

#[no_mangle]
extern "C" fn init() {
    let mut game: Game = Default::default();
    game.admin = msg::source();
    unsafe { GAME = Some(game) };
}

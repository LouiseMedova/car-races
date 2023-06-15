use car_races::{CarAction, GameAction, GameInfo};
use gstd::{prelude::*, ActorId};
use gtest::{Log, Program, System};
use std::f64::consts::PI;
use std::{thread, time::Duration};
use wrecked::RectManager;
const ADMIN: u64 = 100;
#[test]
fn run_game() {
    let system = System::new();

    // system.init_logger();

    let game = Program::current(&system);
    let game_init_result = game.send_bytes(ADMIN, []);
    assert!(!game_init_result.main_failed());
    let car_1 = Program::from_file(
        &system,
        "./target/wasm32-unknown-unknown/release/car_1.opt.wasm",
    );
    let car_init_result = car_1.send_bytes(ADMIN, []);
    assert!(!car_init_result.main_failed());
    let car_2 = Program::from_file(
        &system,
        "./target/wasm32-unknown-unknown/release/car_2.opt.wasm",
    );
    let car_init_result = car_2.send_bytes(ADMIN, []);
    assert!(!car_init_result.main_failed());
    let car_3 = Program::from_file(
        &system,
        "./target/wasm32-unknown-unknown/release/car_3.opt.wasm",
    );
    let car_init_result = car_3.send_bytes(ADMIN, []);
    assert!(!car_init_result.main_failed());

    // Registration
    let run_result = game.send(ADMIN, GameAction::Register { car_id: 2.into() });
    assert!(!run_result.main_failed());

    // Registration
    let run_result = game.send(ADMIN, GameAction::Register { car_id: 3.into() });
    assert!(!run_result.main_failed());

    // Registration
    let run_result = game.send(ADMIN, GameAction::Register { car_id: 4.into() });
    assert!(!run_result.main_failed());

    let run_result = game.send(ADMIN, GameAction::StartGame);

    let mut messages: Vec<GameInfo> = Vec::new();
    for msg in run_result.log().iter() {
        if msg.destination() == ADMIN.into() {
            if let Ok(game_info) = GameInfo::decode(&mut msg.payload()) {
                messages.push(game_info);
            }
        }
    }

    messages.sort_by(|a, b| a.round.cmp(&b.round));

    println!("{:?}", messages);

    let mut rectmanager = wrecked::RectManager::new();
    let (width, height) = rectmanager.get_rect_size(wrecked::ROOT).unwrap();
    let mut points = vec![];

    let colors = [
        wrecked::Color::GREEN,
        wrecked::Color::BLUE,
        wrecked::Color::RED,
    ];

    for x in 0..3 {
        let rect_id = rectmanager.new_rect(wrecked::ROOT).ok().unwrap();
        rectmanager.set_bg_color(rect_id, colors[x]);
        rectmanager.set_character(rect_id, 0, 0, ' ');
        points.push(rect_id);
    }

    let num_of_rounds = messages.len();
    for round in 0..num_of_rounds {
        let mut y = 0;
        for (_, car) in messages[round].cars.iter() {
            let rect_id = points[y % 3];
            rectmanager.set_position(rect_id, (car.position / 60) as isize, y as isize);
            rectmanager.render();
            y = y + 1;
        }
        thread::sleep(Duration::new(1, 0));
    }
    thread::sleep(Duration::new(10, 0));
    rectmanager.kill();
}

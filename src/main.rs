pub mod engine;
pub mod platform_builder;

use engine::config::load_config;
use engine::game::Game;
use crate::engine::Entity;
use crate::platform_builder::player::Player;

fn main() {
    //get configs
    load_config("resources/config.json").expect("Error loading config");

    let mut game = Game::new();

    game
        .add_shader(
            "shaders/basic.vert",
            "shaders/basic.frag",
            "basic"
        ).expect("Could not create basic shader.");
    
    game
        .add_shader(
            "shaders/basic.vert",
            "shaders/color.frag",
            "color"
        ).expect("Could not create color shader.");

    let player= Player::new(&mut game);
    
    game.register_entity(player.id(), player);
    
    game.run();

}

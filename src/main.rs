pub mod engine;
pub mod platform_builder;

use glam::Vec2;
use engine::config::load_config;
use engine::game::Game;
use crate::engine::config::config;
use crate::engine::Entity;
use crate::platform_builder::background::Background;
use crate::platform_builder::camera_control::CameraController;
use crate::platform_builder::player::Player;
use crate::platform_builder::wall::Wall;

fn main() {
    //get configs
    load_config("resources/config.json").expect("Error loading config");

    let mut game = Game::new();

    game
        .add_shader(
            "shaders/camera.vert",
            "shaders/basic.frag",
            "camera"
        ).expect("Could not create camera shader.");

    let world = game.world_mut();

    let mut player= Player::new(world);
    let transform = world.get_mut_transform(player.id()).expect("Failed to get transform.");
    transform.translate(Vec2::new(
        config().logic_size.0 as f32 / 2.0 - 16.0,
        config().logic_size.1 as f32 / 2.0 - 32.0,
    ));
    player.z_index = 1;

    let mut other= Player::new(world);
    let transform = world.get_mut_transform(other.id()).expect("Failed to get transform.");
    transform.translate(Vec2::new(
        config().logic_size.0 as f32 / 4.0 - 16.0,
        config().logic_size.1 as f32 / 4.0 - 32.0,
    ));
    other.z_index = 3;
    other.is_active = false;

    let mut wall = Wall::new(
        "resources/textures/square.png",
        16,
        64,
        world
    );
    wall.z_index = 2;

    let mut background = Background::new(
        world,
        "resources/textures/background_camo.png",
        config().logic_size.0 * 2,
        config().logic_size.1 * 2
    );
    background.z_index = 0;

    let def_cam = world.camera_mut();

    def_cam.position = Vec2::new(
        config().logic_size.0 as f32 / 2.0,
        config().logic_size.1 as f32 / 2.0,
    );

    game.register_entity(player.id(), player);
    game.register_entity(wall.id(), wall);
    game.register_entity(background.id(), background);
    game.register_entity(other.id(), other);
    
    game.run();
}

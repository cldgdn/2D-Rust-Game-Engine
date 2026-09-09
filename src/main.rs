pub mod engine;
pub mod platform_builder;

use glam::Vec4;
use engine::config::load_config;
use engine::game::Game;
use engine::mask_bit;
use crate::engine::camera::{Camera, CameraID};
use crate::engine::config::config;
use crate::engine::rendering::pass::{RenderPass, ZIndexOrdering};
use crate::engine::rendering::post_process::PostProcessLayer;
use crate::engine::rendering::shader::{GlslType, Shader, ShaderBuilder, ShaderDescriptor, ShaderHook};
use crate::platform_builder::moving_platform::MovingPlatform;
use crate::platform_builder::player_platform::PlayerPlatform;
use crate::platform_builder::player_top_down::{PlayerTopDown};
use crate::platform_builder::scene_manager::SceneManager;
use crate::platform_builder::wall::{Wall};

fn main() {
    //get configs
    load_config("resources/config.json").expect("Error loading config");

    let mut game = Game::new();
    game.add_entity_factory("Wall", Wall::from_init);
    game.add_entity_factory("PlayerTopDown", PlayerTopDown::from_init);
    game.add_entity_factory("PlayerPlatform", PlayerPlatform::from_init);
    game.add_entity_factory("MovingPlatform", MovingPlatform::from_init);

    let world = game.world_mut();

    MovingPlatform::set_specific_atbs(world);

    let world = game.world();

    let (text_shader, descriptor) = ShaderBuilder::new(world.gl().clone())
        .add_extended_attribute("aColorTint", GlslType::Vec4)
        .inject(ShaderHook::VertexMain, "vColorTint = aColorTint;")
        .add_extended_varying("vColorTint", GlslType::Vec4)
        .inject(ShaderHook::FragmentMain, "color = vec4(color.rgb * vColorTint.rgb, color.a * vColorTint.a);")
        .build()
        .map_err(|e| format!("Error building text shader: {}", e))
        .unwrap();

    game.register_shader("text", text_shader, descriptor);

    let world = game.world_mut();
    let font = world.text_manager_mut().load_font("resources/fonts/Roboto-Regular.ttf").unwrap();

    let mut world_cam = Camera::new(
        world.get_next_camera_id(),
        config().logic_size
    );

    let wcam_id = world_cam.id;
    world.add_camera(world_cam);
    world.name_camera(wcam_id, "world");
    
    register_passes_and_fx(&mut game, wcam_id);
    let world = game.world_mut();
    
    let id = world.get_next_entity_id();
    let scene_manager = SceneManager::new(id, world);
    
    game.spawn(Box::new(scene_manager));
    /*
    let mut ui_camera = Camera::new(
        world.get_next_camera_id(),
        config().logic_size
    );
    ui_camera.position = Vec2::new(
        config().logic_size.0 as f32 / 2.0 / config().unit_pixel_size as f32,
        config().logic_size.1 as f32 / 2.0 / config().unit_pixel_size as f32
    );

    world.add_camera(ui_camera);

    register_passes_and_fx(&mut game, wcam_id);

    let world = game.world_mut();

    let id = world.get_next_entity_id();
    let mut player= Player::new(id, PlayerInit {starting_position: (2.5, 3.0), active: true}, world);
    player.z_index = 1;

    let world_cam = world.camera_mut(wcam_id).unwrap();
    world_cam.set_target(Some(player.id()));
    world_cam.set_deadzone(Vec2::new(5.0, 3.0));

    let id = world.get_next_entity_id();
    let mut other= Player::new(id, PlayerInit {starting_position: (5.5, 3.0), active: false}, world);
    other.z_index = 3;

    let id = world.get_next_entity_id();
    let mut wall = Wall::new(
        id,
        WallInit {
            image_path: "resources/textures/square.png".to_string(),
            width: 1,
            height: 4,
        },
        world
    );
    let transform = world.get_component_mut::<Transform>(wall.id()).expect("Failed to get transform.");
    transform.set_position(Vec2::new(1.0, 1.0) + transform.scaled_size() / 2.0);
    wall.z_index = 2;

    let id = world.get_next_entity_id();
    let mut wall2 = Wall::new(
        id,
        WallInit {
            image_path: "resources/textures/square.png".to_string(),
            width: 3,
            height: 1,
        },
        world
    );
    let transform = world.get_component_mut::<Transform>(wall2.id()).expect("Failed to get transform.");
    transform.set_position(Vec2::new(2.0, 1.0) + transform.scaled_size() / 2.0);
    wall2.z_index = 2;

    let mut background = Background::new(
        world,
        "resources/textures/background_camo.png",
        config().screen_unit_size().x as u32,
        config().screen_unit_size().y as u32
    );
    let transform = world.get_component_mut::<Transform>(background.id()).expect("Failed to get transform.");
    transform.set_position(transform.scaled_size() / 2.0);
    background.z_index = 0;

    let mut enemy = Enemy::new(world);
    let mut transform = world.get_component_mut::<Transform>(enemy.id()).expect("Failed to get transform.");
    transform.set_position(transform.scaled_size() / 2.0 * 6.0);

    let text = TextBox::new(world, "balls,\ncones,\ncylinders\nand cubes", font, 16.0);
    let mut transform = world.get_component_mut::<Transform>(text.id()).expect("Failed to get transform.");
    transform.set_position(Vec2::new(2.0, 10.0));
    
    let def_cam = world.camera_mut(0).unwrap();

    def_cam.position = Vec2::new(
        config().logic_size.0 as f32 / 2.0,
        config().logic_size.1 as f32 / 2.0,
    );

    game.spawn(Box::new(player));
    game.spawn(Box::new(wall));
    game.spawn(Box::new(wall2));
    game.spawn(Box::new(background));
    game.spawn(Box::new(other));
    game.spawn(Box::new(enemy));
    game.spawn(Box::new(text));
    */
    
    
    game.run();
}


fn register_passes_and_fx(game: &mut Game, world_cam: CameraID) {
    let greenshift = Shader::new(
        game.gl(),
        "shaders/post-fx/default.vert",
        "shaders/post-fx/greenshift.frag"
    ).expect("Error compiling greenshift");

    let greenshift_descriptor = ShaderDescriptor::default();

    game.register_shader("greenshift-fx", greenshift, greenshift_descriptor);

    let greenshift = PostProcessLayer::new(
        "greenshift",
        game.world().shader_id("greenshift-fx").unwrap()
    )
        .with_priority(0);

    let world_pass = RenderPass::new("world", mask_bit(1), world_cam)
        .with_priority(1)
        .with_clear_depth(true)
        .with_clear_color(Vec4::new(0.7, 0.0, 0.0, 1.0))
        .with_ordering_rule(Box::new(ZIndexOrdering{}))
        .add_local_post_fx(greenshift);

    let ui_pass = RenderPass::new("ui", mask_bit(2), 2)
        .with_priority(0)
        .with_clear_depth(true)
        .with_ordering_rule(Box::new(ZIndexOrdering{}));

    game.add_render_pass(world_pass);
    game.add_render_pass(ui_pass);
}
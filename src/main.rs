pub mod engine;
pub mod platform_builder;

use glam::{Vec2, Vec4};
use engine::config::load_config;
use engine::game::Game;
use engine::mask_bit;
use crate::engine::camera::{Camera, CameraBoundary, CameraID};
use crate::engine::component::animation::AnimationManager;
use crate::engine::component::transform::Transform;
use crate::engine::component::update_gates::ActiveFlag;
use crate::engine::config::config;
use crate::engine::Entity;
use crate::engine::rendering::pass::{RenderPass, ZIndexOrdering};
use crate::engine::rendering::post_process::PostProcessLayer;
use crate::engine::rendering::shader::{GlslType, Shader, ShaderBuilder, ShaderDescriptor, ShaderHook};
use crate::platform_builder::background::Background;
use crate::platform_builder::enemy::Enemy;
use crate::platform_builder::player::Player;
use crate::platform_builder::text_box::TextBox;
use crate::platform_builder::wall::Wall;

fn main() {
    //get configs
    load_config("resources/config.json").expect("Error loading config");

    let mut game = Game::new();

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
    world_cam.position = Vec2::new(
        2.5,
        3.0
    );
    world_cam.set_boundary(Some(CameraBoundary{
        left: 0.0,
        right: 3.0,
        bottom: 0.0,
        top: 30.0,
    }));

    let wcam_id = world_cam.id;
    world.add_camera(world_cam);

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

    let mut player= Player::new(world);
    let transform = world.get_component_mut::<Transform>(player.id()).expect("Failed to get transform.");
    transform.set_position(Vec2::new(2.0, 2.0) + transform.scaled_size() / 2.0);
    player.z_index = 1;

    let world_cam = world.camera_mut(wcam_id).unwrap();
    world_cam.set_target(Some(player.id()));
    world_cam.set_deadzone(Vec2::new(5.0, 3.0));

    let mut other= Player::new(world);
    let transform = world.get_component_mut::<Transform>(other.id()).expect("Failed to get transform.");
    transform.translate(Vec2::new(5.0, 2.0) + transform.scaled_size() / 2.0);
    other.z_index = 3;
    let active = world.get_component_mut::<ActiveFlag>(other.id()).expect("Failed to get active flag.");
    active.flag = false;

    let mut wall = Wall::new(
        "resources/textures/square.png",
        1,
        4,
        world
    );
    let transform = world.get_component_mut::<Transform>(wall.id()).expect("Failed to get transform.");
    transform.set_position(Vec2::new(1.0, 1.0) + transform.scaled_size() / 2.0);
    wall.z_index = 2;

    let mut wall2 = Wall::new(
        "resources/textures/square.png",
        3,
        1,
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

    game.register_entity(player.id(), player);
    game.register_entity(wall.id(), wall);
    game.register_entity(wall2.id(), wall2);
    game.register_entity(background.id(), background);
    game.register_entity(other.id(), other);
    game.register_entity(enemy.id(), enemy);
    game.register_entity(text.id(), text);
    
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
pub mod engine;

use glfw::{Action, Context as _, Key, WindowHint, WindowMode};
use glow::HasContext;
use std::rc::Rc;

use engine::shader::Shader;
use engine::config::load_config;
use engine::config::config;

fn main() {
    //get configs
    load_config("resources/config.json").expect("Error loading config");
    let con = config();

    //Initialize GLFW

    let mut glfw = glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW");

    glfw.window_hint(WindowHint::ContextVersion(3, 3));
    glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
    #[cfg(target_os = "macos")]
    glfw.window_hint(WindowHint::OpenGlForwardCompat(true));

    let (mut window, events) = glfw
        .create_window(con.window_size.0, con.window_size.1, &con.game_name, WindowMode::Windowed)
        .expect("Failed to create GLFW window");

    window.make_current();
    window.set_key_polling(true);
    window.set_framebuffer_size_polling(true);

    //Create GLOW context
    let gl = Rc::new( unsafe {
        glow::Context::from_loader_function(|s| {
            window
                .get_proc_address(s)
                .map_or(std::ptr::null(), |p| p as *const _)
        })
    });
    
    let basic_shader = Shader::new(
        gl.clone(),
        "shaders/basic.vert",
        "shaders/basic.frag",
    ).expect("Failed to create basic shader");

    //triangle vertex data
    let vertices: [f32; 6] = [
        -0.5, -0.5,
        0.5, -0.5,
        0.0,  0.5,
    ];

    //VAO + VBO
    let (vao, vbo) = unsafe {
        let vao = gl.create_vertex_array().expect("Cannot create VAO");
        let vbo = gl.create_buffer().expect("Cannot create VBO");

        gl.bind_vertex_array(Some(vao));
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(&vertices),
            glow::STATIC_DRAW,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 2 * std::mem::size_of::<f32>() as i32, 0);

        gl.bind_buffer(glow::ARRAY_BUFFER, None);
        gl.bind_vertex_array(None);

        (vao, vbo)
    };

    //set initial viewport
    unsafe {
        gl.viewport(0, 0, con.window_size.0 as i32, con.window_size.1 as i32);
    }

    //Main loop

    while !window.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::FramebufferSize(width, height) => unsafe {
                    gl.viewport(0, 0, width, height);
                },
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                _ => {}
            }
        }

        unsafe {
            gl.clear_color(0.2, 0.3, 0.3, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
        }
        
        basic_shader.use_program();
        basic_shader.set_vec4("color", 1.0, 0.0, 0.5, 0.0);
        
        unsafe {
            gl.bind_vertex_array(Some(vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }

        window.swap_buffers();
    }

    //cleanup
    unsafe {
        gl.delete_buffer(vbo);
        gl.delete_vertex_array(vao);
    }
}

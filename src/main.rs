use rockwork::context::Context;
use rockwork::mesh::Mesh;
use rockwork::program::Program;
use rockwork::texture::Texture;
use rockwork::framebuffer::Framebuffer;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::cell::RefCell;
use std::io::Cursor;
use std::io::Error;
use std::time::Duration;
use gl::types::*;
use nalgebra::{zero, Vector2, Matrix2};
use nalgebra::geometry::Point2;

pub enum GameState {
    Menu,
    Instruction,
    Game,
    GameOver,
}

pub struct GameData {
    program: Program,
    quad: Mesh,
    map: Texture,
    cursor: Texture,
    cursor_position: Point2<i32>,
    fb: Framebuffer,
    color_tex: Texture,
    light_tex: Texture,
}

static mut GAME_DATA: Option<GameData> = None;
static WIDTH: usize = 320;
static HEIGHT: usize = 240;
static SCALING: usize = 3;

fn handle_input(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    for event in ctx.sdl_event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => {
                std::process::exit(0);
            }
            Event::MouseMotion { x, y, .. } => {
                gd.cursor_position = Point2::new(x, y);
            }
            _ => {}
        }
    }
}

fn draw_texture_rect_centered(gd: &GameData, tex: &Texture, p: Point2<i32>) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    gd.fb.bind();
    gd.program.bind_texture("tex", &tex, 0);
    gd.program.set_uniform_mat2("transform", 
                                   &Matrix2::new(tex.width as f32 / WIDTH as f32, 0.0, 
                                                 0.0, tex.height as f32 / HEIGHT as f32));
    gd.program.set_uniform_vec2("offset", &Vector2::new(p.x as f32,
                                                        p.y as f32));
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.program.draw(&gd.quad);
}

fn draw_texture_rect_screenspace(gd: &GameData, tex: &Texture, p: Point2<i32>) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    gd.fb.bind();
    gd.program.bind_texture("tex", &tex, 0);
    gd.program.set_uniform_mat2("transform", 
                                   &Matrix2::new(tex.width as f32 / WIDTH as f32, 0.0, 
                                                 0.0, tex.height as f32 / HEIGHT as f32));
    // transform from [0..W, 0..H] to [-1..1, 1..-1]
    gd.program.set_uniform_vec2("offset", &Vector2::new(2.0 * p.x as f32 / WIDTH as f32 
                                                        - 1.0,
                                                        -2.0 * p.y as f32 / HEIGHT as f32 
                                                        + 1.0));
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.program.draw(&gd.quad);
}

fn draw(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };

    ctx.window().clear();
    draw_texture_rect_centered(gd, &gd.map, Point2::new(0, 0));

    draw_texture_rect_screenspace(gd, &gd.cursor, gd.cursor_position / SCALING as i32);

    Framebuffer::unbind();
    unsafe { gl::Viewport(0, 0,
                          (WIDTH * SCALING) as GLint,
                          (HEIGHT * SCALING) as GLint) };
    gd.program.bind_texture("tex", &gd.color_tex, 0);
    gd.program.set_uniform_mat2("transform", &Matrix2::identity());
    gd.program.set_uniform_vec2("offset", &zero());
    gd.program.draw(&gd.quad);

    ctx.swap_buffers();

    //dbg!(unsafe { gl::GetError() });
}

fn tick(ctx: &mut Context, _dt: Duration) {

    handle_input(ctx);
    draw(ctx);

}

fn main() -> Result<(), String> {
    let mut ctx: Context = Context::new();
    ctx.open_window("Draw".to_string(), WIDTH * SCALING, HEIGHT * SCALING);

    let mut prog = Program::new("Simple".to_string());
    prog.add_vertex_shader(&mut Cursor::new(
        include_bytes!("../assets/deferred.vs").as_ref(),
    ))
    .unwrap();
    prog.add_fragment_shader(&mut Cursor::new(
        include_bytes!("../assets/deferred.fs").as_ref(),
    ))
    .unwrap();
    prog.build().unwrap();

    let quad = Mesh::from_mdl(&mut Cursor::new(
        include_bytes!("../assets/unit_quad.mdl").as_ref(),
    ))
    .unwrap();

    let mut img = image::load(
        &mut Cursor::new(include_bytes!("../assets/map.png").as_ref()),
        image::ImageFormat::PNG,
    )
    .unwrap();
    dbg!(unsafe { gl::GetError() });
    let tex = Texture::new_rgba_from_image(&mut img);
    dbg!(unsafe { gl::GetError() });

    let mut fb = Framebuffer::new();
    dbg!(unsafe { gl::GetError() });
    let color_tex = Texture::new_rgba(WIDTH, HEIGHT);
    dbg!(unsafe { gl::GetError() });
    let light_tex = Texture::new_rgba(WIDTH, HEIGHT);
    fb.add_target(&color_tex);
    fb.add_target(&light_tex);

    unsafe { gl::Viewport(0, 0, 
                          WIDTH as GLint,
                          HEIGHT as GLint) };
    dbg!(unsafe { gl::GetError() });

    /*
    prog.bind_texture(&tex, 0, "tex".to_string());
    fb.bind();
    prog.draw(&quad);
    Framebuffer::unbind();
    unsafe { gl::Viewport(0, 0,
                          (WIDTH * SCALING) as GLint,
                          (HEIGHT * SCALING) as GLint) };
                          */

    dbg!(unsafe { gl::GetError() });
    unsafe {
        GAME_DATA = Some(GameData {
            program: prog,
            quad: quad,
            map: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/map.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            cursor: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/cursor.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            cursor_position: Point2::new(0, 0),
            fb: fb,
            color_tex: color_tex,
            light_tex: light_tex,
        })
    };

    ctx.run(&mut tick);
    Ok(())
}

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

// idea? everything starts negative?
pub struct Stats {
    money: f32, // Debt
    belonging: f32, // Loneliness
    purpose: f32, // Ennui
    pride: f32, // Shame
    stress: f32,  // Relaxation
}

pub struct City {
    name: &'static str,
    position: [i32; 2],
}

// TODO: y value is probably upside-down
static CITIES: &'static [City] = &[
    City { name: "SF", position: [19, 165] },
    City { name: "LA", position: [29, 183] },
    City { name: "Seattle", position: [31, 117] },
    City { name: "Vancouver", position: [32, 103] },
    City { name: "Las Vegas", position: [48, 173] },
    City { name: "Calgary", position: [77, 106] },
    City { name: "Boulder", position: [92, 166] },
    City { name: "Chicago", position: [149, 149] },
    City { name: "Toronto", position: [178, 142] },
    City { name: "Ottawa", position: [183, 123] },
    City { name: "Montreal", position: [196, 122] },
    City { name: "NY", position: [204, 145] },
    City { name: "Miami", position: [196, 223] },
    City { name: "Halifax", position: [232, 115] },
];

pub struct GameData {
    program: Program,
    water: Program,
    water_tex: Texture,
    quad: Mesh,
    map: Texture,
    cursor: Texture,
    cursor_position: Point2<i32>,
    city_marker: Texture,
    bar: Texture,
    bar_base: Texture,
    fb: Framebuffer,
    color_tex: Texture,
    light_tex: Texture,
    tick: f64,
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
    gd.program.set_uniform_float("xtrim", 1.0);
    gd.program.set_uniform_float("tick", gd.tick as f32);
    gd.program.set_uniform_vec2("bounce", &zero());
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.program.draw(&gd.quad);
}

fn draw_texture_rect_screenspace(gd: &GameData, tex: &Texture, p: Point2<i32>) {
    draw_texture_rect_extra(gd, tex, p, 1.0, gd.tick as f32, zero());
}

fn draw_texture_rect_extra(gd: &GameData, tex: &Texture, p: Point2<i32>,
                           xtrim: f32, tick: f32, mut bounce: Vector2<f32>) {
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
    gd.program.set_uniform_float("xtrim", xtrim);
    gd.program.set_uniform_float("tick", tick);
    bounce.x /= WIDTH as f32;
    bounce.y /= HEIGHT as f32;
    gd.program.set_uniform_vec2("bounce", &bounce);
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.program.draw(&gd.quad);
}

fn draw_water(gd: &GameData) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    let tex = &gd.water_tex;
    gd.fb.bind();
    gd.water.bind_texture("tex", tex, 0);
    gd.water.set_uniform_mat2("transform", 
                                   &Matrix2::new(tex.width as f32 / WIDTH as f32, 0.0, 
                                                 0.0, tex.height as f32 / HEIGHT as f32));
    gd.water.set_uniform_vec2("offset", &zero());
    gd.water.set_uniform_float("xtrim", 1.0);
    gd.water.set_uniform_float("tick", gd.tick as f32);
    gd.water.set_uniform_vec2("bounce", &zero());
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.water.draw(&gd.quad);
}

fn draw_bar(gd: &GameData, p: Point2<i32>, value: f32) {
    draw_texture_rect_screenspace(gd, &gd.bar_base, p);
    draw_texture_rect_extra(gd, &gd.bar, p, value, gd.tick as f32, zero());
}

fn draw_map(gd: &GameData) {
    draw_texture_rect_centered(gd, &gd.map, Point2::new(0, 0));
}

fn draw(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };

    ctx.window().clear();
    draw_water(gd);
    draw_map(gd);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 20) as i32), 1.0);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 40) as i32), (gd.tick.sin() / 2.0 + 0.5) as f32);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 60) as i32), 1.0);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 80) as i32), 1.0);

    for city in CITIES {
        let p = Point2::new(city.position[0], city.position[1]);
        draw_texture_rect_screenspace(gd, &gd.city_marker, p);
    }

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
}

fn tick(ctx: &mut Context, dt: Duration) {
    let mut gd = unsafe { GAME_DATA.as_mut().unwrap() };
    gd.tick += dt.subsec_nanos() as f64 / 1_000_000_000.0;
    handle_input(ctx);
    draw(ctx);

}

fn main() -> Result<(), String> {
    let mut ctx: Context = Context::new();
    ctx.open_window("Draw".to_string(), WIDTH * SCALING, HEIGHT * SCALING);

    // Simple shader
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

    // Water shader
    let mut water = Program::new("Water".to_string());
    water.add_vertex_shader(&mut Cursor::new(
        include_bytes!("../assets/deferred.vs").as_ref(),
    ))
    .unwrap();
    water.add_fragment_shader(&mut Cursor::new(
        include_bytes!("../assets/water.fs").as_ref(),
    ))
    .unwrap();
    water.build().unwrap();

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

    unsafe {
        GAME_DATA = Some(GameData {
            program: prog,
            water: water,
            water_tex: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/water.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
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
            city_marker: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/city_marker.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            bar: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/bar.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            bar_base: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/bar_base.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            fb: fb,
            color_tex: color_tex,
            light_tex: light_tex,
            tick: 0.0,
        })
    };

    ctx.run(&mut tick);
    Ok(())
}

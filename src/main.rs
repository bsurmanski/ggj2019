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
use nalgebra::{zero, Vector2, Vector4, Matrix2};
use nalgebra::geometry::Point2;

#[macro_use]
extern crate lazy_static;

pub enum GameState {
    Menu,
    Instruction,
    Game,
    GameOver,
}

pub struct Modal {
    kind: ModalKind,
    descision: Option<bool>,
}

impl Modal {
    pub fn new(k: ModalKind) -> Self {
        Self { kind: k, descision: None }
    }
}

pub enum ModalKind {
    HelloWorld,
    UniOttawa,
    UniToronto,
    UniMontreal,
    GetMarried,
    HaveKids,
}

// idea? everything starts negative?
pub struct Stats {
    money: f32, // Debt
    belonging: f32, // Loneliness
    purpose: f32, // Ennui
    pride: f32, // Shame
    relaxation: f32,  // Stress 

    play_exp: f32,
    social_exp: f32,
    research_exp: f32,
    create_exp: f32,
    work_exp: f32,
}

impl Stats {
    fn new() -> Self {
        Self {
            money: 0.0,
            belonging: 1.0,
            purpose: 1.0,
            pride: 1.0,
            relaxation: 1.0,

            play_exp: 0.0,
            social_exp: 0.0,
            research_exp: 0.0,
            create_exp: 0.0,
            work_exp: 0.0,
        }
    }
}

pub enum Focus {
    Play,
    Socialize,
    Research,
    Create,
    Work,
}

pub fn focus_stage(gd: &GameData) -> u32 {
    match gd.age {
        0 => 0,
        1...7 => 1,
        8...14 => 2,
        15...21 => 3,
        _ => 4,
    }
}

pub fn focus_is_unlocked(gd: &GameData, f: Focus) -> bool {
    match f {
        Focus::Play => true,
        Focus::Socialize => gd.age >= 1,
        Focus::Research => gd.age >= 8,
        Focus::Create => gd.age >= 15,
        Focus::Work => gd.age >= 22
    }
}

pub enum Action {
    Move,
    Travel,
}

#[derive(Clone)]
pub struct Friend {
    bond: f32,
    compatibility: f32,
    exp: f32, // time spent with friend
    //name: &'static str,
}

#[derive(Clone)]
pub struct City {
    name: &'static str,
    position: [i32; 2],
    friends: Vec<Friend>,
    home: bool,
    exp: f32, // time spent here
}

impl City {
    pub fn new(name: &'static str, position: [i32; 2]) -> Self {
        City { name: name, position: position, home: false, friends: vec![], exp: 0.0 }
    }
    pub fn new_home(name: &'static str, position: [i32; 2]) -> Self {
        City { name: name, position: position, home: true, friends: vec![], exp: 0.0 }
    }
}

lazy_static!{
    static ref CITIES: Vec<City> = {
        let mut v = Vec::new();
        let cities = [
            City::new_home("Toronto", [181, 135]),
            City::new("Ottawa", [183, 123]),
            City::new("Montreal", [196, 122]),
            City::new("SF", [19, 165]),
            City::new("Seattle", [31, 117]),
            City::new("Vancouver", [32, 103]),
            City::new("LA", [29, 183]),
            City::new("Las Vegas", [48, 173]),
            City::new("Calgary", [77, 106]),
            City::new("Miami", [196, 223]),
            City::new("Chicago", [149, 149]),
            City::new("NY", [204, 145]),
            City::new("Halifax", [232, 115]),
            City::new("Boulder", [92, 166]),
        ];
        v.extend_from_slice(&cities);
        v
    };
}

pub struct GameData {
    program: Program,
    water: Program,
    water_tex: Texture,
    quad: Mesh,
    map: Texture,
    cursor: Texture,
    cursor_position: Point2<i32>,
    city_marker: Texture,
    home_marker: Texture,
    bar: Texture,
    bar_base: Texture,
    belonging_label: Texture,
    purpose_label: Texture,
    pride_label: Texture,
    relaxation_label: Texture,
    focus_labels: Texture,
    focus_box: Texture,
    numbers: Texture,
    font: Texture,
    age_label: Texture,
    arrow: Texture,
    modal_box: Texture,
    fb: Framebuffer,
    color_tex: Texture,
    light_tex: Texture,
    tick: f64,

    age: u32,
    stats: Stats,
    current_focus: Focus,
    current_city: usize,
    arrow_position: Vector2<f32>,
    current_modal: Option<Modal>,
    paused: bool,
}


static mut GAME_DATA: Option<GameData> = None;
static WIDTH: usize = 320;
static HEIGHT: usize = 240;
static SCALING: usize = 3;
static TICKS_PER_WEEK: f64 = 0.1;

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
            Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                gd.current_focus = Focus::Play;
            }
            Event::KeyDown { keycode: Some(Keycode::S), .. } => {
                if focus_is_unlocked(gd, Focus::Socialize) {
                    gd.current_focus = Focus::Socialize;
                }
            }
            Event::KeyDown { keycode: Some(Keycode::T), .. } => {
                if focus_is_unlocked(gd, Focus::Research) {
                    gd.current_focus = Focus::Research;
                }
            }
            Event::KeyDown { keycode: Some(Keycode::C), .. } => {
                if focus_is_unlocked(gd, Focus::Create) {
                    gd.current_focus = Focus::Create;
                }
            }
            Event::KeyDown { keycode: Some(Keycode::W), .. } => {
                if focus_is_unlocked(gd, Focus::Work) {
                    gd.current_focus = Focus::Work;
                }
            }
            Event::MouseMotion { x, y, .. } => {
                gd.cursor_position = Point2::new(x, y);
            }
            _ => {}
        }
    }
}

fn draw_digit(gd: &GameData, p: Point2<i32>, digit: u32) {
    assert!(digit < 10);
    let new_p = Point2::new(p.x + 23 - (digit * 5) as i32, p.y);
    draw_texture_rect_extra(gd, &gd.numbers, new_p,
                            gd.tick as f32, 
                            Vector2::new(0.1 * digit as f32 + 0.1, 1.0),
                            Vector2::new(1.0 - 0.1 * digit as f32, 1.0),
                            Vector2::new(0.0, 0.0), 
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
}

fn draw_letter(gd: &GameData, p: Point2<i32>, letter: char) {
    let c = letter.to_ascii_uppercase();
    let index = c as i32 - 'A' as i32;
    let new_p = Point2::new(p.x + 78 - (index * 6) as i32, p.y);
    let letter_w = 1.0 / 26.0;

    draw_texture_rect_extra(gd, &gd.font, new_p,
                            gd.tick as f32, 
                            Vector2::new(letter_w * index as f32 + letter_w, 1.0),
                            Vector2::new(1.0 - letter_w * index as f32, 1.0),
                            Vector2::new(0.0, 0.0), 
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
}

fn draw_string(gd: &GameData, p: Point2<i32>, s: String) {
    let mut y = p.y;
    for l in s.lines() {
        let len = l.len();
        for (i, c) in l.chars().enumerate() {
            draw_letter(gd, Point2::new(p.x + 6 * i as i32 
                                        - 3 * len as i32, y), c);
        }
        y += 10;
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
    gd.program.set_uniform_float("tick", gd.tick as f32);
    gd.program.set_uniform_vec2("rtrim", &Vector2::new(1.0, 1.0));
    gd.program.set_uniform_vec2("trim", &Vector2::new(1.0, 1.0));
    gd.program.set_uniform_vec2("bounce", &zero());
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.program.draw(&gd.quad);
}

fn draw_texture_rect_screenspace(gd: &GameData, tex: &Texture, p: Point2<i32>) {
    draw_texture_rect_extra(gd, tex, p, gd.tick as f32, 
                            Vector2::new(1.0, 1.0), 
                            Vector2::new(1.0, 1.0), zero(),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
}

fn draw_texture_rect_extra(gd: &GameData, tex: &Texture, p: Point2<i32>,
                           tick: f32, trim: Vector2<f32>, rtrim: Vector2<f32>, 
                           mut bounce: Vector2<f32>, 
                           tint: Vector4<f32>) {
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
    gd.program.set_uniform_float("tick", tick);
    gd.program.set_uniform_vec2("trim", &trim);
    gd.program.set_uniform_vec2("rtrim", &rtrim);
    bounce.x /= tex.width as f32;
    bounce.y /= tex.height as f32;
    gd.program.set_uniform_vec2("bounce", &bounce);
    gd.program.set_uniform_vec4("tint", &tint);
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
    gd.water.set_uniform_float("tick", gd.tick as f32);
    gd.water.set_uniform_vec2("trim", &Vector2::new(1.0, 1.0));
    gd.water.set_uniform_vec2("bounce", &Vector2::new(10.0 / WIDTH as f32, 0.0));
    unsafe { gl::Viewport(0, 0, WIDTH as GLint, HEIGHT as GLint) };
    gd.water.draw(&gd.quad);
}

fn draw_bar(gd: &GameData, p: Point2<i32>, label: &Texture, value: f32) {
    draw_texture_rect_extra(gd, &gd.bar_base, p, -1.0 * gd.tick as f32,
                            Vector2::new(1.0, 1.0), // trim
                            Vector2::new(1.0, 1.0), // rtrim
                            Vector2::new(0.0, 1.0),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
    draw_texture_rect_extra(gd, &gd.bar, p, -1.0 * gd.tick as f32,
                            Vector2::new(value, 1.0), // trim
                            Vector2::new(1.0, 1.0), // rtrim
                            Vector2::new(0.0, 1.0),
                            Vector4::new(0.3, value * 0.8, 0.3, 1.0));
    draw_texture_rect_extra(gd, &label, p, -1.0 * gd.tick as f32,
                            Vector2::new(1.0, 1.0),
                            Vector2::new(1.0, 1.0), // rtrim
                            Vector2::new(0.0, 1.0),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
}

fn draw_map(gd: &GameData) {
    draw_texture_rect_centered(gd, &gd.map, Point2::new(0, 0));
}

fn draw_focus_box(gd: &GameData) {
    let unlock = [0.25, 0.4, 0.6, 0.7, 1.0];
    draw_texture_rect_screenspace(gd, &gd.focus_box, Point2::new(265, 40));
    draw_texture_rect_extra(gd, &gd.focus_labels, Point2::new(265, 40), gd.tick as f32,
                            Vector2::new(1.0, unlock[focus_stage(gd) as usize]),
                            Vector2::new(1.0, 1.0), // rtrim
                            zero(),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));

    draw_texture_rect_screenspace(gd, &gd.arrow, Point2::new(gd.arrow_position.x as i32,
                                                             gd.arrow_position.y as i32));
}

fn draw_age(gd: &GameData) {
    draw_texture_rect_screenspace(gd, &gd.age_label, Point2::new(175, 17));
    if gd.age >= 100 { draw_digit(&gd, Point2::new(180, 16), gd.age / 100 % 10); }
    if gd.age >= 10 { draw_digit(&gd, Point2::new(185, 16), (gd.age / 10) % 10); }
    draw_digit(&gd, Point2::new(190, 16), gd.age % 10);
}

fn draw_modal(gd: &GameData, m: Modal) {
    draw_texture_rect_extra(gd, &gd.modal_box,
                            Point2::new(WIDTH as i32 / 2, HEIGHT as i32 / 2),
                            gd.tick as f32,
                            Vector2::new(1.0, 1.0),
                            Vector2::new(1.0, 1.0), // rtrim
                            Vector2::new(1.0, 0.0),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));
    match m.kind {
        ModalKind::HelloWorld => {
            draw_string(gd, Point2::new(WIDTH as i32 / 2, HEIGHT as i32 / 2),
            "HELLO\nWorld".to_string());
        }
        _ => {
        }
    }
}

fn draw_cities(gd: &GameData) {
    for city in CITIES.iter() {
        let p = Point2::new(city.position[0], city.position[1]);
        if city.home {
            let offset_y = (gd.tick * 4.0).sin().abs() * 3.0;
            draw_texture_rect_extra(gd, &gd.home_marker,
                                    Point2::new(p.x, p.y + offset_y as i32),
                                    gd.tick as f32,
                                    Vector2::new(1.0, 1.0), // trim
                                    Vector2::new(1.0, 1.0), // rtrim
                                    Vector2::new(0.0, 0.0), // wiggle
                                    Vector4::new(1.0, 0.9, 0.9, 1.0));
        } else {
            draw_texture_rect_screenspace(gd, &gd.city_marker, p);
        }
    }
}

fn draw(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };

    ctx.window().clear();
    draw_water(gd);
    draw_map(gd);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 20) as i32),
             &gd.relaxation_label, gd.stats.relaxation);
    draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 40) as i32), 
             &gd.belonging_label, gd.stats.belonging);
    if gd.age > 4 {
        draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 60) as i32),
                 &gd.pride_label, gd.stats.pride);
    }
    if gd.age > 13 {
        draw_bar(gd, Point2::new((WIDTH - 50) as i32, (HEIGHT - 80) as i32), 
                 &gd.purpose_label, gd.stats.purpose);
    }
    draw_focus_box(&gd);
    draw_age(&gd);

    draw_cities(&gd);

    //draw_modal(gd, Modal::new(ModalKind::HelloWorld));

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

fn update(dt: f64) {
    let mut gd = unsafe { GAME_DATA.as_mut().unwrap() };
    let dweek = (dt / TICKS_PER_WEEK) as f32;

    // arrow
    { 
        let y_offset = 17 + match gd.current_focus {
            Focus::Play => 0,
            Focus::Socialize => 11,
            Focus::Research => 22,
            Focus::Create => 33,
            Focus::Work => 45,
        };
        let delta = y_offset as f32 - gd.arrow_position.y;
        gd.arrow_position.y = gd.arrow_position.y + delta * dt as f32 * 8.0;
    }

    if let Some(m) = &gd.current_modal {
        if let Some(d) = m.descision {
            match m.kind {
                _ => {
                }
            }
        }
    } else { // no modal
        // age
        {
            gd.age = (gd.tick / (TICKS_PER_WEEK * 50.0)) as u32;
        }

        // stats
        {
            match gd.age {
                0...2 => {
                    gd.stats.relaxation -= 0.005 * dweek;
                    gd.stats.belonging -= 0.005 * dweek;
                    match gd.current_focus {
                        Focus::Play => {
                            gd.stats.relaxation += 0.015 * dweek;
                        }
                        Focus::Socialize | _ => {
                            gd.stats.belonging += 0.015 * dweek;
                        }
                    }
                }
                3...5 => {
                    gd.stats.relaxation -= 0.005 * dweek;
                    gd.stats.belonging -= 0.005 * dweek;
                    gd.stats.pride -= 0.005 * dweek;
                    if gd.stats.belonging > 0.99 {
                        gd.stats.pride += 0.006 * dweek;
                    }
                    match gd.current_focus {
                        Focus::Play => {
                            gd.stats.relaxation += 0.015 * dweek;
                            gd.stats.play_exp += 0.005 * dweek;
                        }
                        Focus::Socialize => {
                            gd.stats.belonging += 0.015 * dweek;
                            gd.stats.social_exp += 0.005 * dweek;
                        }
                        Focus::Research | _ => {
                            gd.stats.research_exp += 0.005 * dweek;
                        }
                    }
                }
                6...12 => {
                    gd.stats.relaxation -= 0.005 * dweek;
                    gd.stats.belonging -= 0.005 * dweek;
                    gd.stats.pride -= 0.005 * dweek;
                    gd.stats.purpose -= 0.005 * dweek;
                    match gd.current_focus {
                        Focus::Play => {
                            gd.stats.relaxation += 0.015 * dweek;
                            gd.stats.pride += gd.stats.play_exp * dweek / 100.0;
                            gd.stats.play_exp += 0.005 * dweek;
                        }
                        Focus::Socialize | _ => {
                            gd.stats.belonging += 0.015 * dweek;
                            gd.stats.pride += 0.005 * dweek;
                            gd.stats.social_exp += 0.005 * dweek;
                        }
                        Focus::Research | _ => {
                            gd.stats.research_exp += 0.005 * dweek;
                        }
                    }
                }
                13...20 => {
                    gd.stats.relaxation -= 0.005 * dweek;
                    gd.stats.belonging -= 0.005 * dweek;
                    gd.stats.pride -= 0.005 * dweek;
                    gd.stats.purpose -= 0.005 * dweek;
                    match gd.current_focus {
                        Focus::Play => {
                            gd.stats.relaxation += 0.010 * dweek;
                            gd.stats.play_exp += 0.001 * dweek;
                        }
                        Focus::Socialize => {
                            gd.stats.belonging += 0.010 * dweek;
                            gd.stats.social_exp += 0.001 * dweek;
                        }
                        Focus::Research => {
                            gd.stats.belonging += 0.001 * dweek;
                            gd.stats.pride += 0.001 * dweek;
                            gd.stats.research_exp += 0.001 * dweek;
                        }
                        Focus::Create | _ => {
                            gd.stats.pride += 0.005 * dweek;
                            gd.stats.create_exp += 0.001 * dweek;
                        }
                    }
                }
                _ => {
                    gd.stats.relaxation -= 0.005 * dweek;
                    gd.stats.belonging -= 0.005 * dweek;
                    match gd.current_focus {
                        Focus::Play => {
                            gd.stats.relaxation += 0.015 * dweek;
                            gd.stats.play_exp += 0.005 * dweek;
                        }
                        Focus::Socialize | _ => {
                            gd.stats.belonging += 0.015 * dweek;
                            gd.stats.social_exp += 0.005 * dweek;
                        }
                        Focus::Research => {
                            gd.stats.research_exp += 0.002 * dweek;
                        }
                        Focus::Create => {
                            gd.stats.pride += gd.stats.create_exp / 100.0 * dweek;
                            gd.stats.create_exp += 0.001 * dweek;
                        }
                        Focus::Work => {
                        }
                    }
                }
            }
        }

        gd.stats.belonging = nalgebra::clamp(gd.stats.belonging , 0.0, 1.1);
        gd.stats.purpose = nalgebra::clamp(gd.stats.purpose, 0.0, 1.1);
        gd.stats.pride = nalgebra::clamp(gd.stats.pride, 0.0, 1.1);
        gd.stats.relaxation = nalgebra::clamp(gd.stats.relaxation, 0.0, 1.1);
    }
}

fn tick(ctx: &mut Context, dt: Duration) {
    let mut gd = unsafe { GAME_DATA.as_mut().unwrap() };
    let f64_dt = dt.subsec_nanos() as f64 / 1_000_000_000.0;
    gd.tick += f64_dt;
    handle_input(ctx);
    update(f64_dt);
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
            home_marker: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/home_marker.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            bar: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/bar.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            bar_base: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/bar_base.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            belonging_label: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/belonging_label.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            purpose_label: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/purpose_label.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            pride_label: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/pride_label.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            relaxation_label: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/relaxation_label.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            focus_labels: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/focus_labels.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            focus_box: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/focus_box.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            numbers: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/numbers.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            font: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/font.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            age_label: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/age_label.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            arrow: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/arrow.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            modal_box: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/modal.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            fb: fb,
            color_tex: color_tex,
            light_tex: light_tex,
            tick: 0.0,

            age: 0,
            stats: Stats::new(),
            current_focus: Focus::Play,
            current_city: 0,
            arrow_position: Vector2::new(262.0, 17.0),
            paused: false,
            current_modal: None,
        })
    };

    ctx.run(&mut tick);
    Ok(())
}

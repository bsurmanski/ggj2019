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
use std::sync::Mutex;

#[macro_use]
extern crate lazy_static;

pub enum GameState {
    Title,
    Instruction,
    Game,
    Modal,
    Fly,
    GameOver,
}

pub struct Modal {
    kind: ModalKind,
    text: &'static str,
    choices: Vec<&'static str>,
    selection: i32,
}

impl Modal {
    pub fn new(k: ModalKind, text: &'static str, choices: Vec<&'static str>) -> Self {
        Self { kind: k, text: text, choices: choices, selection: 0 }
    }
}

pub enum ModalKind {
    Tantrum,
    Move,
    Married,
    Divorce,
    Kids,
    Die,
}

pub fn maybe_start_modal(gd: &mut GameData, new_modal: Modal) -> bool {
    for m in gd.modals_done.iter() {
        if str_eq(m.text, new_modal.text) {
            return false;
        }
    }
    gd.current_modal = Some(new_modal);
    gd.game_state = GameState::Modal;
    return true;
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
    static ref CITIES: Mutex<Vec<City>> = {
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
        Mutex::new(v)
    };
}

pub fn str_eq(s1: &'static str, s2: &'static str) -> bool {
    return s1.to_string().to_lowercase() == s2.to_string().to_lowercase();
}


pub fn home_city() -> City {
    for c in CITIES.lock().unwrap().iter() {
        if c.home {
            return c.clone();
        }
    }
    panic!("no home city?");
}

pub fn set_home_city(new_home: &'static str) {
    let mut cities =  CITIES.lock().unwrap();
    for c in cities.iter_mut() {
        if c.home {
            c.home = false;
        }
        if c.name.to_string().to_lowercase() == new_home.to_string().to_lowercase() {
            c.home = true;
        }
    }
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
    title: Texture,
    plane: Texture,
    fb: Framebuffer,
    color_tex: Texture,
    light_tex: Texture,
    tick: f64,

    age: u32,
    stats: Stats,
    current_focus: Focus,
    current_city: usize,
    married: bool,
    kids: u32,
    moves: u32,
    arrow_position: Vector2<f32>,
    plane_position: Vector2<f32>,
    current_modal: Option<Modal>,
    modals_done: Vec<Modal>,
    game_state: GameState,
}


static mut GAME_DATA: Option<GameData> = None;
static WIDTH: usize = 320;
static HEIGHT: usize = 240;
static SCALING: usize = 3;
//static TICKS_PER_WEEK: f64 = 0.1;
static TICKS_PER_WEEK: f64 = 0.06;

fn handle_input(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    match gd.game_state {
        GameState::Title => {
                for event in ctx.sdl_event_pump.poll_iter() {
                    match event {
                    Event::Quit { .. } |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        std::process::exit(0);
                    }
                    Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                        gd.game_state = GameState::Game;
                        gd.tick = 0.0;
                    }
                    _ => {}
                }
            }
        }
        GameState::Modal => {
            for event in ctx.sdl_event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        std::process::exit(0);
                    }
                    Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                        let mut mo = gd.current_modal.as_mut().unwrap();
                        mo.selection -= 1; 
                        if mo.selection < 0 {
                            mo.selection = mo.choices.len() as i32 - 1;
                        }
                    }
                    Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                        let mut mo = gd.current_modal.as_mut().unwrap();
                        mo.selection += 1; 
                        mo.selection %= mo.choices.len() as i32;
                    }
                    Event::KeyDown { keycode: Some(Keycode::Return), .. } => {
                        execute_modal(gd);
                    }
                    _ => {}
                }
            }
        }
        GameState::Game => {
            for event in ctx.sdl_event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        std::process::exit(0);
                    }
                    Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                        match gd.current_focus {
                            Focus::Work => {
                                gd.current_focus = Focus::Create;
                            }
                            Focus::Create => {
                                gd.current_focus = Focus::Research;
                            }
                            Focus::Research => {
                                gd.current_focus = Focus::Socialize;
                            }
                            _ => {
                                gd.current_focus = Focus::Play;
                            }
                        }
                    }
                    Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                        match gd.current_focus {
                            Focus::Play => {
                                if focus_is_unlocked(gd, Focus::Socialize) {
                                    gd.current_focus = Focus::Socialize;
                                }
                            }
                            Focus::Socialize => {
                                if focus_is_unlocked(gd, Focus::Research) {
                                    gd.current_focus = Focus::Research;
                                }
                            }
                            Focus::Research => {
                                if focus_is_unlocked(gd, Focus::Create) {
                                    gd.current_focus = Focus::Create;
                                }
                            }
                            _ => {
                                if focus_is_unlocked(gd, Focus::Work) {
                                    gd.current_focus = Focus::Work;
                                }
                            }
                        }
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
                    Event::KeyDown { keycode: Some(Keycode::L), .. } => {
                        gd.current_modal = Some(Modal::new(ModalKind::Move,
                                                           "goto university?",
                                                           vec!["Toronto", 
                                                           "SF", "Montreal", "No"]));
                        gd.game_state = GameState::Modal;
                    }
                    /*
                    Event::MouseMotion { x, y, .. } => {
                        //gd.cursor_position = Point2::new(x, y);
                    } */
                    _ => {}
                }
            }
        }
        _ => {
            for event in ctx.sdl_event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } |
                        Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
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
    let index = if letter == '?' { 26 } else {
        let c = letter.to_ascii_uppercase();
        c as i32 - 'A' as i32
    };
    let new_p = Point2::new(p.x + 81 - (index * 6) as i32, p.y);
    let letter_w = 1.0 / 27.0;

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
    let mat2 = Matrix2::new(tex.width as f32 / WIDTH as f32, 0.0,
                            0.0, tex.height as f32 / HEIGHT as f32);
    draw_texture_rect_with_mat2(gd, tex, p, tick, trim, rtrim, bounce, tint, mat2);
}

fn draw_texture_rect_with_mat2(gd: &GameData,
                               tex: &Texture,
                               p: Point2<i32>,
                               tick: f32,
                               trim: Vector2<f32>,
                               rtrim: Vector2<f32>,
                               mut bounce: Vector2<f32>,
                               tint: Vector4<f32>,
                               mat: Matrix2<f32>) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };
    gd.fb.bind();
    gd.program.bind_texture("tex", &tex, 0);
    gd.program.set_uniform_mat2("transform", &mat);
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

fn draw_modal(gd: &GameData, m: &Modal) {
    draw_texture_rect_extra(gd, &gd.modal_box,
                            Point2::new(WIDTH as i32 / 2, HEIGHT as i32 / 2),
                            gd.tick as f32,
                            Vector2::new(1.0, 1.0),
                            Vector2::new(1.0, 1.0), // rtrim
                            Vector2::new(1.0, 0.0),
                            Vector4::new(1.0, 1.0, 1.0, 1.0));

    draw_string(gd, Point2::new(WIDTH as i32 / 2, HEIGHT as i32 / 2 - 20),
    m.text.to_string());
    for (i, c) in m.choices.iter().enumerate() {
        let y = HEIGHT as i32 / 2 + 10 * i as i32;
        draw_string(gd, Point2::new(WIDTH as i32 / 2,
                                    y),
        c.to_string());

        if m.selection == i as i32 {
            draw_texture_rect_screenspace(
                gd, &gd.arrow, Point2::new(WIDTH as i32 / 2 - c.len() as i32 * 5, 
                                           y));
        }
    }
}

fn draw_cities(gd: &GameData) {
    for city in CITIES.lock().unwrap().iter() {
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

fn draw_standard(gd: &mut GameData) {
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
}

fn draw(ctx: &mut Context) {
    let gd = unsafe { GAME_DATA.as_mut().unwrap() };

    ctx.window().clear();
    match gd.game_state {
        GameState::Title => {
            let c = (((gd.tick * 3.0).cos() / 2.0 + 0.5) as f32).max(0.3);
            draw_texture_rect_extra(gd, &gd.title,
                                    Point2::new((WIDTH / 2) as i32, (HEIGHT / 2) as i32),
                                    gd.tick as f32,
                                    Vector2::new(1.0, 1.0), // trim
                                    Vector2::new(1.0, 1.0), // rtrim
                                    Vector2::new(10.0, 0.0), // wiggle
                                    Vector4::new(0.4, c, 0.4, 1.0));
        }
        GameState::Modal => {
            draw_standard(gd);
            if let Some(m) = &gd.current_modal {
                draw_modal(gd, m);
            }
        }
        GameState::Fly => {
            draw_standard(gd);

            let home = home_city();
            let home_pos = Vector2::new(home.position[0] as f32,
                                        home.position[1] as f32);
            let rot_matrix = Matrix2::new(gd.plane.width as f32 / WIDTH as f32, 0.0, 
                                          0.0, gd.plane.height as f32 / HEIGHT as f32);
            let mut delta = home_pos - gd.plane_position;
            delta = delta.normalize();

            draw_texture_rect_with_mat2(gd, &gd.plane, 
                                    Point2::new(gd.plane_position.x as i32,
                                                gd.plane_position.y as i32), 
                                    gd.tick as f32,
                                    Vector2::new(1.0, 1.0), // trim
                                    Vector2::new(1.0, 1.0), // rtrim
                                    Vector2::new(0.0, 1.0),
                                    Vector4::new(1.0, 1.0, 1.0, 1.0),
                                    rot_matrix);
        }
        GameState::GameOver => {
            gd.fb.bind();
            unsafe { 
                gl::ClearColor(0.1, 0.0, 0.1, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT); 
            }
        }
        GameState::Game | _ => {
            draw_standard(gd);

            /* Mouse cursor not yet implemented.
            draw_texture_rect_screenspace(gd, &gd.cursor, 
                                          gd.cursor_position / SCALING as i32);
            */
        }
    }

    Framebuffer::unbind();
    unsafe { gl::Viewport(0, 0,
                          (WIDTH * SCALING) as GLint,
                          (HEIGHT * SCALING) as GLint) };
    gd.program.bind_texture("tex", &gd.color_tex, 0);
    gd.program.set_uniform_mat2("transform", &Matrix2::identity());
    gd.program.set_uniform_vec2("offset", &zero());
    gd.program.set_uniform_vec2("trim", &Vector2::new(1.0, 1.0));
    gd.program.set_uniform_vec2("rtrim", &Vector2::new(1.0, 1.0));
    match gd.game_state {
        GameState::Modal => {
            gd.program.set_uniform_vec2("bounce", &Vector2::new(5.0 / WIDTH as f32, 0.0));
        },
        _ => {
            gd.program.set_uniform_vec2("bounce", &Vector2::new(0.0, 0.0));
        }
    }
    gd.program.draw(&gd.quad);

    ctx.swap_buffers();
}

fn execute_modal(gd: &mut GameData) {
    let modal = gd.current_modal.as_ref().unwrap();
    let choice = modal.choices[modal.selection as usize];
    match modal.kind {
        ModalKind::Move => {
            let current_home = home_city();
            let not_moving = str_eq(choice, "no") || str_eq(choice, current_home.name);
            gd.plane_position = Vector2::new(current_home.position[0] as f32,
                                             current_home.position[1] as f32);
            if !not_moving {
                set_home_city(choice);
                gd.moves += 1;

                match gd.moves {
                    1 => {
                        gd.stats.relaxation -= 0.05; 
                        gd.stats.purpose += 0.1;
                        gd.stats.belonging += 0.1;
                        gd.stats.pride += 0.25;
                    }
                    2 => {
                        gd.stats.pride += 0.05;
                    }
                    3 => {
                        gd.stats.relaxation -= 0.05; 
                        gd.stats.purpose -= 0.1;
                    }
                    _ => {
                        gd.stats.belonging -= 0.1;
                        gd.stats.purpose -= 0.1;
                    }
                }
            } 

            gd.game_state = GameState::Fly;
        }
        ModalKind::Married => {
            if str_eq(choice, "yes") {
                gd.stats.purpose += 0.2;
                gd.stats.belonging += 0.1;
                gd.stats.relaxation -= 0.1;
                gd.married = true;
            }
            gd.game_state = GameState::Game;
        }
        ModalKind::Kids => {
            if str_eq(choice, "yes") {
                gd.stats.pride += 0.3;
                gd.stats.purpose += 0.1;
                gd.stats.relaxation -= 0.1;
            }
            gd.game_state = GameState::Game;
        }
        ModalKind::Divorce => {
            if str_eq(choice, "mend") {
                gd.stats.pride += 0.3;
                gd.stats.purpose += 0.1;
                gd.stats.belonging += 0.1;
                gd.stats.relaxation -= 0.2;
            }

            if str_eq(choice, "divorce") {
                gd.stats.pride -= 0.1;
                gd.stats.purpose -= 0.1;
                gd.stats.relaxation += 0.1;
            }

            if str_eq(choice, "suffer") {
                gd.stats.pride -= 0.1;
                gd.stats.purpose += 0.3;
                gd.stats.relaxation -= 0.1;
            }
            gd.game_state = GameState::Game;
        }
        ModalKind::Tantrum => {
            if str_eq(choice, "yes") {
                gd.stats.relaxation += 0.1;
                gd.stats.social_exp -= 0.1;
            } else {
                gd.stats.relaxation -= 0.1;
                gd.stats.social_exp += 0.1;
            }
            gd.game_state = GameState::Game;
        }
        ModalKind::Die => {
            gd.game_state = GameState::GameOver;
        }
        _ => {
            gd.game_state = GameState::Game;
        }
    }
    gd.modals_done.push(gd.current_modal.take().unwrap());
}

fn update(dt: f64) {
    let mut gd = unsafe { GAME_DATA.as_mut().unwrap() };

    match gd.game_state {
        GameState::Modal => {
        }
        GameState::Fly => {
            let home = home_city();
            let home_pos = Vector2::new(home.position[0] as f32,
                                        home.position[1] as f32);
            let mut delta = home_pos - gd.plane_position;
            if delta.magnitude() < 4.0 {
                gd.game_state = GameState::Game;
            }
            delta = delta.normalize();
            gd.plane_position = gd.plane_position + delta;
        }
        GameState::Game => {
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

            // age
            {
                gd.age = (gd.tick / (TICKS_PER_WEEK * 50.0)) as u32;
            }

            if gd.age == 2 {
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Tantrum,
                        "tantrum?", vec!["yes", "no"]));
            }

            if gd.age == 18 {
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Move,
                                   "goto university?",
                                   vec!["toronto", "ottawa", "montreal", "no"]));
            }

            if gd.age == 24 {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Move,
                                   "take job?",
                                   vec![home.name, "vancouver", "sf", "ny"]));
            }

            if gd.age == 30 {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Married,
                                   "get married?",
                                   vec!["yes", "no"]));
            }

            if gd.age == 32 && gd.married {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Kids,
                                   "spouse wants kids",
                                   vec!["yes", "no"]));
            }

            if gd.age == 35 && !gd.married {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Move,
                                   "move somewhere\nexciting?",
                                   vec!["seattle", "calgary", "boulder", "la"]));
            }

            if gd.age == 40 && gd.married {
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Divorce,
                                   "failing marriage",
                                   vec!["divorce", "mend", "suffer"]));
            }

            if gd.age == 60 {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Move,
                                   "time to retire?",
                                   vec![home.name, "miami", "la"]));
            }

            if gd.age == 70 && !gd.married {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Die,
                                   "die?",
                                   vec!["if i must"]));
            }

            if gd.age == 75 {
                let home = home_city();
                maybe_start_modal(
                    gd, Modal::new(ModalKind::Die,
                                   "die?",
                                   vec!["if i must"]));
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
                        gd.stats.pride -= 0.002 * dweek;
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
                                gd.stats.relaxation += 0.020 * dweek;
                                gd.stats.pride += gd.stats.play_exp * dweek / 100.0;
                                gd.stats.play_exp += 0.005 * dweek;
                            }
                            Focus::Socialize | _ => {
                                gd.stats.belonging += 0.020 * dweek;
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
                                gd.stats.relaxation += 0.016 * dweek;
                                gd.stats.play_exp += 0.001 * dweek;
                            }
                            Focus::Socialize => {
                                gd.stats.belonging += 0.016 * dweek;
                                gd.stats.social_exp += 0.001 * dweek;
                            }
                            Focus::Research => {
                                gd.stats.belonging += 0.001 * dweek;
                                gd.stats.pride += 0.001 * dweek;
                                gd.stats.research_exp += 0.001 * dweek;
                            }
                            Focus::Create | _ => {
                                gd.stats.purpose += 0.0010 * dweek;
                                gd.stats.pride += 0.0008 * dweek;
                                gd.stats.create_exp += 0.001 * dweek;
                            }
                        }
                    }
                    21...40 => {
                        gd.stats.relaxation -= 0.005 * dweek;
                        gd.stats.belonging -= 0.005 * dweek;
                        gd.stats.pride -= 0.005 * dweek;
                        gd.stats.purpose -= 0.005 * dweek;
                        match gd.current_focus {
                            Focus::Play => {
                                gd.stats.relaxation += 0.017 * dweek;
                                gd.stats.play_exp += 0.007 * dweek;
                            }
                            Focus::Socialize => {
                                gd.stats.belonging += 0.017 * dweek;
                                gd.stats.social_exp += 0.007 * dweek;
                            }
                            Focus::Research => {
                                gd.stats.research_exp += 0.002 * dweek;
                            }
                            Focus::Create => {
                                gd.stats.pride += gd.stats.create_exp / 100.0 * dweek;
                                gd.stats.create_exp += 0.001 * dweek;
                            }
                            Focus::Work => {
                                gd.stats.pride += 0.02 * dweek;
                                gd.stats.purpose += 0.01 * dweek;
                            }
                        }
                    }
                    _ => {
                        gd.stats.relaxation -= 0.003 * dweek;
                        gd.stats.belonging -= 0.003 * dweek;
                        gd.stats.pride -= 0.003 * dweek;
                        gd.stats.purpose -= 0.003 * dweek;
                        match gd.current_focus {
                            Focus::Play => {
                                gd.stats.relaxation += 0.017 * dweek;
                                gd.stats.play_exp += 0.007 * dweek;
                            }
                            Focus::Socialize => {
                                gd.stats.belonging += 0.017 * dweek;
                                gd.stats.social_exp += 0.007 * dweek;
                            }
                            Focus::Research => {
                                gd.stats.research_exp += 0.002 * dweek;
                            }
                            Focus::Create => {
                                gd.stats.pride += gd.stats.create_exp / 100.0 * dweek;
                                gd.stats.create_exp += 0.001 * dweek;
                            }
                            Focus::Work => {
                                gd.stats.pride += 0.02 * dweek;
                                gd.stats.purpose += 0.01 * dweek;
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
        _ => {}
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
    ctx.open_window("Belonging".to_string(), WIDTH * SCALING, HEIGHT * SCALING);

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
    let tex = Texture::new_rgba_from_image(&mut img);

    let mut fb = Framebuffer::new();
    let color_tex = Texture::new_rgba(WIDTH, HEIGHT);
    let light_tex = Texture::new_rgba(WIDTH, HEIGHT);
    fb.add_target(&color_tex);
    fb.add_target(&light_tex);

    unsafe { gl::Viewport(0, 0, 
                          WIDTH as GLint,
                          HEIGHT as GLint) };

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
            title: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/title.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            plane: Texture::new_rgba_from_image(
                &mut image::load(
                    &mut Cursor::new(include_bytes!("../assets/airplane.png").as_ref()),
                    image::ImageFormat::PNG).unwrap()),
            fb: fb,
            color_tex: color_tex,
            light_tex: light_tex,
            tick: 0.0,

            age: 0,
            stats: Stats::new(),
            current_focus: Focus::Play,
            current_city: 0,
            married: false,
            kids: 0,
            moves: 0,
            arrow_position: Vector2::new(262.0, 17.0),
            plane_position: Vector2::new(0.0, 0.0),
            modals_done: vec![],
            //game_state: GameState::Game,
            game_state: GameState::Title,
            current_modal: None,
        })
    };

    dbg!(unsafe { gl::GetError() });
    ctx.run(&mut tick);
    Ok(())
}

extern crate rand;
extern crate rayon;
extern crate sdl2;
extern crate specs;

use rand::prelude::*;
use rayon::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use specs::prelude::*;

const GRAVITY_CONST: f32 = 1e-5;
const NUM_OBJ: u32 = 1_000;

#[derive(Default)]
struct DeltaTime(f32);

#[derive(Debug, Copy, Clone, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {
    type Storage = VecStorage<Self>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Mass(f32);

impl Component for Mass {
    type Storage = VecStorage<Self>;
}

#[allow(dead_code)]
struct TextRender;

impl<'a> System<'a> for TextRender {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        ReadStorage<'a, Mass>,
    );

    fn run(&mut self, (ent, pos, vel, mass): Self::SystemData) {
        use specs::Join;

        for (ent, pos, vel, mass) in (&ent, &pos, &vel, &mass).join() {
            println!("id {}: {:?} {:?} {:?}", &ent.id(), &pos, &vel, &mass);
        }
    }
}

struct UpdateVel;

impl<'a> System<'a> for UpdateVel {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, Mass>,
        WriteStorage<'a, Velocity>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (ent, delta, pos, mass, mut vel) = data;

        let delta = delta.0;

        (&ent, &pos, &mut vel)
            .par_join()
            .for_each(|(first_ent, first_pos, first_vel)| {
                (&ent, &mass, &pos)
                    .join()
                    .for_each(|(second_ent, second_mass, second_pos)| {
                        let second_mass = second_mass.0;

                        if first_ent.id() != second_ent.id() {
                            let r_sq = (second_pos.x - first_pos.x).powi(2)
                                + (second_pos.y - first_pos.y).powi(2);

                            if r_sq >= 1e6 {
                                return;
                            }

                            let force = GRAVITY_CONST * second_mass / r_sq;

                            let dir_x_sq = (second_pos.x - first_pos.x).powi(2);
                            let dir_y_sq = (second_pos.y - first_pos.y).powi(2);

                            let dir_mag_sq = dir_x_sq + dir_y_sq;

                            let dir_x = dir_x_sq / dir_mag_sq;
                            let dir_y = dir_y_sq / dir_mag_sq;

                            let dir_x = if second_pos.x > first_pos.x {
                                dir_x
                            } else {
                                -dir_x
                            };

                            let dir_y = if second_pos.y > first_pos.y {
                                dir_y
                            } else {
                                -dir_y
                            };

                            let acc_x = force * dir_x;
                            let acc_y = force * dir_y;

                            first_vel.x += acc_x * delta;
                            first_vel.y += acc_y * delta;
                        }
                    });
            });
    }
}

struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (
        Read<'a, DeltaTime>,
        ReadStorage<'a, Velocity>,
        WriteStorage<'a, Position>,
    );

    fn run(&mut self, (delta, vel, mut pos): Self::SystemData) {
        use specs::Join;

        let delta = delta.0;

        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * delta;
            pos.y += vel.y * delta;
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();
    world.register::<Mass>();

    world.add_resource(DeltaTime(0.1));

    for _ in 0..NUM_OBJ {
        world
            .create_entity()
            .with(Position {
                x: 20.0 * random::<f32>(),
                y: 20.0 * random::<f32>(),
            })
            .with(Velocity { x: 0.0, y: 0.0 })
            .with(Mass(1.0))
            .build();
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(UpdateVel, "update_vel", &[])
        .with(UpdatePos, "update_pos", &["update_vel"])
        .build();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("rust-sdl2 demo", 512, 512)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(5, 5, 5));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.set_draw_color(Color::RGB(5, 5, 5));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        dispatcher.dispatch(&world.res);
        world.maintain();

        let pos_storage = world.read_storage::<Position>();
        let mass_storage = world.read_storage::<Mass>();

        canvas.set_draw_color(Color::RGB(240, 240, 240));
        for (pos, _mass) in (&pos_storage, &mass_storage).join() {
            let x = (pos.x / 20.0 - 0.5) * 128.0 + 256.0;
            let y = (pos.y / 20.0 - 0.5) * 128.0 + 256.0;
            let p = Point::new(x.round() as i32, y.round() as i32);
            canvas.draw_point(p).unwrap();
        }

        canvas.present();
    }
}

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};
use nalgebra_glm::{Mat4, Vec3};
use rayon::prelude::*;

mod color;
mod fragment;
mod fragment_shaders;
mod framebuffer;
mod line;
mod obj;
mod shaders;
mod skybox;
mod sphere;
mod triangle;
mod vertex;

use fragment_shaders::{
    azure_planet_shader, crimson_planet_shader, gas_giant_shader, moon_shader, ring_shader,
    rocky_planet_shader, ship_shader, star_shader,
};
use framebuffer::Framebuffer;
use obj::Obj;
use shaders::vertex_shader;
use skybox::Skybox;
use sphere::{generate_ring, generate_sphere};
use triangle::triangle_with_shader;
use vertex::Vertex;

const WINDOW_WIDTH: usize = 1200;
const WINDOW_HEIGHT: usize = 800;
const FRAME_DELAY: Duration = Duration::from_millis(8);

pub struct Uniforms {
    model_matrix: Mat4,
}

struct Moon<'a> {
    orbit_radius: f32,
    orbit_speed: f32,
    rotation_speed: f32,
    scale: f32,
    phase: f32,
    mesh: &'a [Vertex],
    shader: fragment_shaders::FragmentShader,
}

struct RingDef<'a> {
    mesh: &'a [Vertex],
    rotation_speed: f32,
    scale: f32,
}

struct Planet<'a> {
    name: &'static str,
    orbit_radius: f32,
    orbit_speed: f32,
    rotation_speed: f32,
    scale: f32,
    phase: f32,
    orbit_color: u32,
    collision_radius: f32,
    mesh: &'a [Vertex],
    shader: fragment_shaders::FragmentShader,
    moon: Option<Moon<'a>>,
    ring: Option<RingDef<'a>>,
}

impl<'a> Planet<'a> {
    fn position(&self, time: f32) -> Vec3 {
        if self.orbit_radius == 0.0 {
            return Vec3::new(0.0, 0.0, 0.0);
        }
        let angle = time * self.orbit_speed + self.phase;
        Vec3::new(
            self.orbit_radius * angle.cos(),
            0.0,
            self.orbit_radius * angle.sin(),
        )
    }
}

struct WarpState {
    origin: Vec3,
    target: Vec3,
    elapsed: f32,
    duration: f32,
}

struct Camera {
    position: Vec3,
    zoom: f32,
    tilt: f32,
    speed: f32,
    warp: Option<WarpState>,
    last_direction: Vec3,
}

impl Camera {
    fn new() -> Self {
        Camera {
            position: Vec3::new(0.0, 0.0, -250.0),
            zoom: 1.0,
            tilt: 0.45,
            speed: 200.0,
            warp: None,
            last_direction: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    fn handle_input(&mut self, window: &Window, delta: f32) {
        if self.warp.is_some() {
            return;
        }

        let mut direction = Vec3::new(0.0, 0.0, 0.0);
        if window.is_key_down(Key::W) || window.is_key_down(Key::Up) {
            direction.z -= 1.0;
        }
        if window.is_key_down(Key::S) || window.is_key_down(Key::Down) {
            direction.z += 1.0;
        }
        if window.is_key_down(Key::A) || window.is_key_down(Key::Left) {
            direction.x -= 1.0;
        }
        if window.is_key_down(Key::D) || window.is_key_down(Key::Right) {
            direction.x += 1.0;
        }
        if window.is_key_down(Key::R) {
            direction.y += 1.0;
        }
        if window.is_key_down(Key::F) {
            direction.y -= 1.0;
        }

        if direction.magnitude() > 0.0 {
            let move_dir = direction.normalize();
            let boost = if window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift)
            {
                2.2
            } else {
                1.0
            };
            self.position += move_dir * self.speed * boost * delta;
            self.last_direction = move_dir;
        } else {
            self.last_direction *= 0.9;
        }

        if window.is_key_down(Key::Equal) || window.is_key_down(Key::PageUp) {
            self.zoom = (self.zoom + delta * 0.6).min(1.8);
        }
        if window.is_key_down(Key::Minus) || window.is_key_down(Key::PageDown) {
            self.zoom = (self.zoom - delta * 0.6).max(0.35);
        }

        self.position.y = self.position.y.clamp(-140.0, 140.0);
    }

    fn advance_warp(&mut self, delta: f32) {
        if let Some(state) = self.warp.as_mut() {
            state.elapsed += delta;
            let progress = (state.elapsed / state.duration).clamp(0.0, 1.0);
            let eased = ease_in_out_cubic(progress);
            self.position = state.origin + (state.target - state.origin) * eased;
            if progress >= 1.0 {
                self.warp = None;
            }
        }
    }

    fn start_warp(&mut self, target: Vec3) {
        self.warp = Some(WarpState {
            origin: self.position,
            target,
            elapsed: 0.0,
            duration: 0.9,
        });
    }

    fn warp_progress(&self) -> Option<f32> {
        self.warp
            .as_ref()
            .map(|state| (state.elapsed / state.duration).clamp(0.0, 1.0))
    }

    fn resolve_collisions(&mut self, blockers: &[(Vec3, f32)]) {
        for (center, radius) in blockers {
            let planar = Vec3::new(self.position.x - center.x, 0.0, self.position.z - center.z);
            let distance = (planar.x * planar.x + planar.z * planar.z).sqrt();
            if distance < *radius && distance > 0.001 {
                let push = planar.normalize() * (*radius - distance + 4.0);
                self.position.x += push.x;
                self.position.z += push.z;
            }
        }

        self.position.x = self.position.x.clamp(-1600.0, 1600.0);
        self.position.z = self.position.z.clamp(-1600.0, 1600.0);
    }
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0, 0.0, 0.0, 0.0, 0.0, cos_x, -sin_x, 0.0, 0.0, sin_x, cos_x, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y, 0.0, sin_y, 0.0, 0.0, 1.0, 0.0, 0.0, -sin_y, 0.0, cos_y, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0, sin_z, cos_z, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale,
        0.0,
        0.0,
        translation.x,
        0.0,
        scale,
        0.0,
        translation.y,
        0.0,
        0.0,
        scale,
        translation.z,
        0.0,
        0.0,
        0.0,
        1.0,
    );

    transform_matrix * rotation_matrix
}

fn render(
    framebuffer: &mut Framebuffer,
    uniforms: &Uniforms,
    vertex_array: &[Vertex],
    fragment_shader: fragment_shaders::FragmentShader,
) {
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    let fragments = transformed_vertices
        .par_chunks(3)
        .filter(|chunk| chunk.len() == 3)
        .map(|chunk| triangle_with_shader(&chunk[0], &chunk[1], &chunk[2], fragment_shader))
        .reduce(|| Vec::new(), |mut acc, mut chunk| {
            acc.append(&mut chunk);
            acc
        });

    for fragment in fragments {
        let x = fragment.position.x as usize;
        let y = fragment.position.y as usize;
        if x < framebuffer.width && y < framebuffer.height {
            let color = fragment.color.to_hex();
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, fragment.depth);
        }
    }
}

fn world_to_screen(world: Vec3, camera: &Camera) -> Vec3 {
    let relative = world - camera.position;
    let x = WINDOW_WIDTH as f32 * 0.5 + relative.x * camera.zoom;
    let y = WINDOW_HEIGHT as f32 * 0.5 - (relative.y * camera.zoom + relative.z * camera.tilt);
    let depth = (relative.x * relative.x + relative.y * relative.y + relative.z * relative.z)
        .sqrt()
        .max(0.0001);
    Vec3::new(x, y, depth)
}

fn draw_orbit(framebuffer: &mut Framebuffer, planet: &Planet, camera: &Camera) {
    if planet.orbit_radius <= 1.0 {
        return;
    }

    let center = Vec3::new(0.0, 0.0, 0.0);
    let mut prev: Option<Vec3> = None;
    for i in 0..=360 {
        let t = i as f32 / 360.0 * TAU;
        let world = Vec3::new(
            center.x + planet.orbit_radius * t.cos(),
            0.0,
            center.z + planet.orbit_radius * t.sin(),
        );
        let screen = world_to_screen(world, camera);
        if let Some(prev_point) = prev {
            framebuffer.draw_overlay_line(
                prev_point.x as i32,
                prev_point.y as i32,
                screen.x as i32,
                screen.y as i32,
                planet.orbit_color,
            );
        }
        prev = Some(screen);
    }
}

fn draw_warp_overlay(framebuffer: &mut Framebuffer, progress: f32) {
    let center_x = (WINDOW_WIDTH / 2) as i32;
    let center_y = (WINDOW_HEIGHT / 2) as i32;
    let radius = (progress * WINDOW_WIDTH as f32 * 0.4) as i32;
    let color = 0x44CCFF;

    for angle in (0..360).step_by(10) {
        let theta = (angle as f32).to_radians();
        let x = center_x + (theta.cos() * radius as f32) as i32;
        let y = center_y + (theta.sin() * radius as f32) as i32;
        framebuffer.draw_overlay_line(center_x, center_y, x, y, color);
    }
}

fn ease_in_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn main() {
    let mut framebuffer = Framebuffer::new(WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut window = Window::new(
        "Sistema Solar Procedural",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .expect("No se pudo crear la ventana");

    window.set_position(100, 100);
    window.update();

    let skybox = Skybox::load("assets/models/skybox.jpg").expect("No se pudo cargar la skybox");
    let ship_vertices = Obj::load("assets/models/Nave.obj")
        .expect("No se pudo cargar el modelo de la nave")
        .get_vertex_array();

    let star_mesh = generate_sphere(1.0, 70);
    let rocky_mesh = generate_sphere(1.0, 50);
    let gas_mesh = generate_sphere(1.0, 60);
    let moon_mesh = generate_sphere(1.0, 35);
    let ring_mesh = generate_ring(1.2, 2.4, 120);

    let mut planets = Vec::new();
    planets.push(Planet {
        name: "Helios",
        orbit_radius: 0.0,
        orbit_speed: 0.0,
        rotation_speed: 0.25,
        scale: 140.0,
        phase: 0.0,
        orbit_color: 0xFFAA44,
        collision_radius: 160.0,
        mesh: &star_mesh,
        shader: star_shader,
        moon: None,
        ring: None,
    });

    planets.push(Planet {
        name: "Azura",
        orbit_radius: 240.0,
        orbit_speed: 0.62,
        rotation_speed: 0.95,
        scale: 60.0,
        phase: 0.35,
        orbit_color: 0x55D0FF,
        collision_radius: 80.0,
        mesh: &rocky_mesh,
        shader: azure_planet_shader,
        moon: None,
        ring: None,
    });

    planets.push(Planet {
        name: "Aurelia",
        orbit_radius: 340.0,
        orbit_speed: 0.46,
        rotation_speed: 1.0,
        scale: 80.0,
        phase: 1.0,
        orbit_color: 0x66FFCC,
        collision_radius: 95.0,
        mesh: &rocky_mesh,
        shader: rocky_planet_shader,
        moon: Some(Moon {
            orbit_radius: 140.0,
            orbit_speed: 1.5,
            rotation_speed: 0.6,
            scale: 28.0,
            phase: 0.6,
            mesh: &moon_mesh,
            shader: moon_shader,
        }),
        ring: None,
    });

    planets.push(Planet {
        name: "Zephyrus",
        orbit_radius: 500.0,
        orbit_speed: 0.32,
        rotation_speed: 0.4,
        scale: 130.0,
        phase: 2.2,
        orbit_color: 0xCC8844,
        collision_radius: 170.0,
        mesh: &gas_mesh,
        shader: gas_giant_shader,
        moon: None,
        ring: Some(RingDef {
            mesh: &ring_mesh,
            rotation_speed: 0.15,
            scale: 150.0,
        }),
    });

    planets.push(Planet {
        name: "Pyra",
        orbit_radius: 640.0,
        orbit_speed: 0.29,
        rotation_speed: 1.1,
        scale: 78.0,
        phase: 0.7,
        orbit_color: 0xFF4433,
        collision_radius: 100.0,
        mesh: &rocky_mesh,
        shader: crimson_planet_shader,
        moon: Some(Moon {
            orbit_radius: 125.0,
            orbit_speed: 1.6,
            rotation_speed: 0.8,
            scale: 26.0,
            phase: 1.2,
            mesh: &moon_mesh,
            shader: moon_shader,
        }),
        ring: None,
    });

    planets.push(Planet {
        name: "Cryon",
        orbit_radius: 820.0,
        orbit_speed: 0.18,
        rotation_speed: 0.5,
        scale: 110.0,
        phase: 3.4,
        orbit_color: 0x55CCFF,
        collision_radius: 140.0,
        mesh: &gas_mesh,
        shader: gas_giant_shader,
        moon: None,
        ring: None,
    });

    let warp_bindings = [
        (Key::Key1, "Helios"),
        (Key::Key2, "Azura"),
        (Key::Key3, "Aurelia"),
        (Key::Key4, "Zephyrus"),
        (Key::Key5, "Pyra"),
        (Key::Key6, "Cryon"),
    ];

    let mut key_latch: HashMap<Key, bool> =
        warp_bindings.iter().map(|(key, _)| (*key, false)).collect();

    framebuffer.set_background_color(0x000000);
    let mut camera = Camera::new();
    let mut time = 0.0f32;
    let mut last_frame = Instant::now();

    while window.is_open() {
        if window.is_key_down(Key::Escape) {
            break;
        }

        let now = Instant::now();
        let delta_time = now.duration_since(last_frame).as_secs_f32().min(0.05);
        last_frame = now;
        time += delta_time;

        framebuffer.clear();
        skybox.draw(&mut framebuffer);

        let mut planet_positions: HashMap<&'static str, Vec3> = HashMap::new();
        let mut blockers = Vec::new();

        for planet in &planets {
            let position = planet.position(time);
            planet_positions.insert(planet.name, position);
            blockers.push((position, planet.collision_radius));

            if let Some(moon) = &planet.moon {
                let angle = time * moon.orbit_speed + moon.phase;
                let moon_pos = position
                    + Vec3::new(
                        moon.orbit_radius * angle.cos(),
                        0.0,
                        moon.orbit_radius * angle.sin(),
                    );
                blockers.push((moon_pos, moon.scale * 0.6));
            }
        }

        camera.handle_input(&window, delta_time);
        camera.advance_warp(delta_time);
        camera.resolve_collisions(&blockers);

        for planet in &planets {
            draw_orbit(&mut framebuffer, planet, &camera);
        }

        for planet in &planets {
            let world_position = *planet_positions
                .get(planet.name)
                .unwrap_or(&Vec3::new(0.0, 0.0, 0.0));
            let screen_position = world_to_screen(world_position, &camera);
            let rotation = Vec3::new(
                0.0,
                planet.rotation_speed * time,
                planet.rotation_speed * 0.3,
            );
            let scale = planet.scale * camera.zoom;
            let model_matrix = create_model_matrix(screen_position, scale, rotation);
            let uniforms = Uniforms { model_matrix };
            render(&mut framebuffer, &uniforms, planet.mesh, planet.shader);

            if let Some(ring) = &planet.ring {
                let ring_matrix = create_model_matrix(
                    screen_position,
                    ring.scale * camera.zoom,
                    Vec3::new(
                        std::f32::consts::FRAC_PI_4 * 0.3,
                        0.0,
                        time * ring.rotation_speed,
                    ),
                );
                let ring_uniforms = Uniforms {
                    model_matrix: ring_matrix,
                };
                render(&mut framebuffer, &ring_uniforms, ring.mesh, ring_shader);
            }

            if let Some(moon) = &planet.moon {
                let angle = time * moon.orbit_speed + moon.phase;
                let moon_world = world_position
                    + Vec3::new(
                        moon.orbit_radius * angle.cos(),
                        0.0,
                        moon.orbit_radius * angle.sin(),
                    );
                let moon_screen = world_to_screen(moon_world, &camera);
                let moon_matrix = create_model_matrix(
                    moon_screen,
                    moon.scale * camera.zoom,
                    Vec3::new(
                        time * moon.rotation_speed,
                        time * moon.rotation_speed * 0.5,
                        0.0,
                    ),
                );
                let moon_uniforms = Uniforms {
                    model_matrix: moon_matrix,
                };
                render(&mut framebuffer, &moon_uniforms, moon.mesh, moon.shader);
            }
        }

        let ship_world = camera.position + Vec3::new(0.0, 20.0 * (time * 2.0).sin(), -140.0);
        let ship_screen = world_to_screen(ship_world, &camera);
        let bank = -camera.last_direction.x * 0.4;
        let ship_matrix = create_model_matrix(
            ship_screen,
            90.0 * camera.zoom,
            Vec3::new(0.2 + (time * 1.5).sin() * 0.1, PI, bank),
        );
        let ship_uniforms = Uniforms {
            model_matrix: ship_matrix,
        };
        render(
            &mut framebuffer,
            &ship_uniforms,
            &ship_vertices,
            ship_shader,
        );

        for (key, target_name) in &warp_bindings {
            let pressed = window.is_key_down(*key);
            let prev = *key_latch.get(key).unwrap_or(&false);
            if pressed && !prev {
                if let Some(target) = planet_positions.get(target_name) {
                    camera.start_warp(*target);
                }
            }
            key_latch.insert(*key, pressed);
        }

        if let Some(progress) = camera.warp_progress() {
            draw_warp_overlay(&mut framebuffer, progress);
        }

        window
            .update_with_buffer(&framebuffer.buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .expect("No se pudo actualizar la ventana");

        std::thread::sleep(FRAME_DELAY);
    }
}

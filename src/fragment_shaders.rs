use crate::color::Color;
use crate::vertex::Vertex;
use nalgebra_glm::{dot, Vec2, Vec3};

pub type FragmentShader = fn(&Vertex, &Vertex, &Vertex, Vec3, Vec3, Vec2) -> Color;

// Utility functions for noise and patterns
fn hash(n: f32) -> f32 {
    let x = (n * 12.9898).sin() * 43758.5453;
    x - x.floor()
}

fn hash_vec3(p: Vec3) -> f32 {
    let n = p.x * 12.9898 + p.y * 78.233 + p.z * 45.164;
    hash(n)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn noise(p: Vec3) -> f32 {
    let i = Vec3::new(p.x.floor(), p.y.floor(), p.z.floor());
    let f = Vec3::new(p.x - i.x, p.y - i.y, p.z - i.z);

    // Smooth interpolation
    let u = Vec3::new(
        smoothstep(0.0, 1.0, f.x),
        smoothstep(0.0, 1.0, f.y),
        smoothstep(0.0, 1.0, f.z),
    );

    // Hash values at corners
    let a = hash_vec3(i);
    let b = hash_vec3(Vec3::new(i.x + 1.0, i.y, i.z));
    let c = hash_vec3(Vec3::new(i.x, i.y + 1.0, i.z));
    let d = hash_vec3(Vec3::new(i.x + 1.0, i.y + 1.0, i.z));
    let e = hash_vec3(Vec3::new(i.x, i.y, i.z + 1.0));
    let f_val = hash_vec3(Vec3::new(i.x + 1.0, i.y, i.z + 1.0));
    let g = hash_vec3(Vec3::new(i.x, i.y + 1.0, i.z + 1.0));
    let h = hash_vec3(Vec3::new(i.x + 1.0, i.y + 1.0, i.z + 1.0));

    // Trilinear interpolation
    let x1 = a + (b - a) * u.x;
    let x2 = c + (d - c) * u.x;
    let y1 = x1 + (x2 - x1) * u.y;

    let x3 = e + (f_val - e) * u.x;
    let x4 = g + (h - g) * u.x;
    let y2 = x3 + (x4 - x3) * u.y;

    y1 + (y2 - y1) * u.z
}

fn fbm(p: Vec3, octaves: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;

    for _ in 0..octaves {
        value += amplitude * noise(Vec3::new(p.x * frequency, p.y * frequency, p.z * frequency));
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    value
}

// Star/Sun Shader
pub fn star_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let intensity = dot(&normal, &light_dir).max(0.0);

    // Base yellow-orange color
    let base_color = Vec3::new(1.0, 0.7, 0.3);

    // Add noise for surface variation
    let noise_value = fbm(
        Vec3::new(position.x * 5.0, position.y * 5.0, position.z * 5.0),
        3,
    );
    let variation = 0.1 * noise_value;

    // Add bright center effect
    let center_dist = (position.x * position.x + position.y * position.y).sqrt();
    let center_glow = (1.0 - center_dist.min(1.0)).powf(2.0) * 0.3;

    // Add solar flare effect based on angle
    let flare = (normal.z * 0.5 + 0.5).powf(3.0) * 0.2;

    let r = (base_color.x + variation + center_glow + flare).clamp(0.0, 1.0);
    let g = (base_color.y + variation * 0.5 + center_glow * 0.8 + flare * 0.9).clamp(0.0, 1.0);
    let b = (base_color.z + variation * 0.3 + center_glow * 0.5).clamp(0.0, 1.0);

    // Apply lighting
    let light_factor = intensity * 0.7 + 0.3;
    let final_color = Vec3::new(r * light_factor, g * light_factor, b * light_factor);

    Color::from_float(final_color.x, final_color.y, final_color.z)
}

// Rocky Planet Shader (Earth-like)
pub fn rocky_planet_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let intensity = dot(&normal, &light_dir).max(0.0);

    // Use spherical coordinates for consistent mapping
    let lat = (position.y / position.magnitude()).acos();

    // Layer 1: Ocean/Continents base
    let continent_noise = fbm(
        Vec3::new(position.x * 2.0, position.y * 2.0, position.z * 2.0),
        4,
    );
    let is_land = continent_noise > 0.1;

    // Layer 2: Ocean depth variation
    let ocean_depth = if !is_land {
        fbm(
            Vec3::new(position.x * 3.0, position.y * 3.0, position.z * 3.0),
            3,
        ) * 0.3
            + 0.7
    } else {
        0.0
    };

    // Layer 3: Land elevation
    let elevation = if is_land {
        fbm(
            Vec3::new(position.x * 4.0, position.y * 4.0, position.z * 4.0),
            3,
        ) * 0.5
            + 0.5
    } else {
        0.0
    };

    // Layer 4: Climate zones (latitude-based)
    let climate = (lat / std::f32::consts::PI).abs();
    let is_polar = climate > 0.7;
    let is_tropical = climate < 0.3;

    // Calculate colors
    let (r, g, b) = if is_land {
        // Land colors
        let base_green = Vec3::new(0.2, 0.6, 0.2);
        let brown = Vec3::new(0.4, 0.3, 0.2);
        let snow = Vec3::new(0.9, 0.9, 0.95);

        let land_color = if is_polar {
            // Snow at poles
            Vec3::new(
                base_green.x * 0.3 + snow.x * 0.7,
                base_green.y * 0.3 + snow.y * 0.7,
                base_green.z * 0.3 + snow.z * 0.7,
            )
        } else if is_tropical {
            // More green in tropics
            Vec3::new(
                base_green.x * 0.8 + brown.x * 0.2,
                base_green.y * 0.8 + brown.y * 0.2,
                base_green.z * 0.8 + brown.z * 0.2,
            )
        } else {
            // Mix based on elevation
            let mix_factor = elevation * 0.5;
            Vec3::new(
                base_green.x * (1.0 - mix_factor) + brown.x * mix_factor,
                base_green.y * (1.0 - mix_factor) + brown.y * mix_factor,
                base_green.z * (1.0 - mix_factor) + brown.z * mix_factor,
            )
        };

        (land_color.x, land_color.y, land_color.z)
    } else {
        // Ocean colors
        let deep_blue = Vec3::new(0.0, 0.2, 0.5);
        let shallow_blue = Vec3::new(0.2, 0.4, 0.7);

        let ocean_color = Vec3::new(
            deep_blue.x * ocean_depth + shallow_blue.x * (1.0 - ocean_depth),
            deep_blue.y * ocean_depth + shallow_blue.y * (1.0 - ocean_depth),
            deep_blue.z * ocean_depth + shallow_blue.z * (1.0 - ocean_depth),
        );
        (ocean_color.x, ocean_color.y, ocean_color.z)
    };

    // Apply lighting with ambient
    let light_factor = intensity * 0.8 + 0.2;
    let final_color = Vec3::new(r * light_factor, g * light_factor, b * light_factor);

    Color::from_float(final_color.x, final_color.y, final_color.z)
}

pub fn azure_planet_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.1, 0.2, -1.0).normalize();
    let intensity = dot(&normal.normalize(), &light_dir).max(0.0);

    let polar_noise = fbm(
        Vec3::new(position.x * 4.0, position.y * 4.0, position.z * 4.0),
        4,
    );
    let ocean_noise = fbm(
        Vec3::new(position.x * 2.5, position.y * 2.5, position.z * 2.5),
        3,
    );

    let ice_caps = ((position.y.abs() / position.magnitude()).powi(4) + polar_noise * 0.3)
        .clamp(0.0, 1.0);
    let ocean_mix = (ocean_noise * 1.2 - 0.2).clamp(0.0, 1.0);

    let abyss = Vec3::new(0.02, 0.18, 0.4);
    let lagoon = Vec3::new(0.18, 0.66, 0.96);
    let aurora = Vec3::new(0.5, 0.9, 1.0);

    let base_water = abyss * (1.0 - ocean_mix) + lagoon * ocean_mix;
    let cloud_bands = fbm(
        Vec3::new(position.x * 6.0, position.y * 6.0, position.z * 6.0),
        5,
    )
        .powf(3.0);
    let cloud_color = Vec3::new(0.85, 0.95, 1.0);
    let mixed = base_water * (1.0 - cloud_bands) + cloud_color * cloud_bands;

    let ice_color = aurora * (0.6 + ice_caps * 0.4);
    let final_base = mixed * (1.0 - ice_caps) + ice_color * ice_caps;

    let highlight = (normal.y * 0.5 + 0.5).powf(8.0) * 0.3;
    let final_color = final_base * (intensity * 0.75 + 0.25) + Vec3::new(highlight, highlight, highlight * 0.8);

    Color::from_float(
        final_color.x.clamp(0.0, 1.0),
        final_color.y.clamp(0.0, 1.0),
        final_color.z.clamp(0.0, 1.0),
    )
}

pub fn crimson_planet_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(-0.2, 0.4, -1.0).normalize();
    let intensity = dot(&normal.normalize(), &light_dir).max(0.0);

    let basalt_noise = fbm(
        Vec3::new(position.x * 3.5, position.y * 3.5, position.z * 3.5),
        4,
    );
    let fissure_noise = fbm(
        Vec3::new(position.x * 8.0, position.y * 8.0, position.z * 8.0),
        5,
    );

    let crater_mask = (basalt_noise - 0.45).abs();
    let lava_threshold = (fissure_noise * 1.4 - 0.5).clamp(0.0, 1.0);

    let basalt = Vec3::new(0.2, 0.05, 0.05);
    let ember = Vec3::new(0.74, 0.16, 0.08);
    let lava_core = Vec3::new(1.0, 0.42, 0.18);

    let lava_mix = lava_threshold.powf(1.6);
    let surface_color = basalt * (1.0 - lava_mix) + ember * lava_mix;
    let molten_core = surface_color * (1.0 - lava_mix) + lava_core * lava_mix;

    let crater_color = surface_color * (0.5 + crater_mask * 0.4);
    let final_base = crater_color * (1.0 - lava_mix) + molten_core * lava_mix;

    let rim_specular = (normal.y * 0.5 + 0.5).powf(8.0) * 0.3;
    let glow = lava_mix * 0.4;

    let shaded = final_base * (intensity * 0.8 + 0.2) + Vec3::new(glow, glow * 0.6, glow * 0.4);
    let final_color = Vec3::new(
        (shaded.x + rim_specular).clamp(0.0, 1.0),
        (shaded.y + rim_specular * 0.4).clamp(0.0, 1.0),
        shaded.z.clamp(0.0, 1.0),
    );

    Color::from_float(final_color.x, final_color.y, final_color.z)
}

// Gas Giant Shader (Jupiter-like)
pub fn gas_giant_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let intensity = dot(&normal, &light_dir).max(0.0);

    // Use latitude for banding
    let lat = position.y / position.magnitude();

    // Layer 1: Base band structure
    let band_freq = 8.0;
    let band = (lat * band_freq).sin() * 0.5 + 0.5;

    // Layer 2: Turbulence for swirls
    let turbulence = fbm(
        Vec3::new(position.x * 3.0, position.y * 3.0, position.z * 3.0),
        4,
    );
    let swirl = (turbulence * 2.0 - 1.0) * 0.3;

    // Layer 3: Color variation within bands
    let color_variation = fbm(
        Vec3::new(position.x * 5.0, position.y * 5.0, position.z * 5.0),
        3,
    ) * 0.2;

    // Layer 4: Great Red Spot-like feature
    let spot_pos = Vec3::new(0.0, 0.3, 0.8);
    let spot_dist = (position.normalize() - spot_pos).magnitude();
    let spot = if spot_dist < 0.3 {
        (1.0 - spot_dist / 0.3).powf(2.0) * 0.4
    } else {
        0.0
    };

    // Jupiter-like colors: browns, oranges, whites
    let dark_band = Vec3::new(0.5, 0.3, 0.2);
    let light_band = Vec3::new(0.8, 0.7, 0.6);
    let red_spot = Vec3::new(0.8, 0.3, 0.2);

    // Mix bands
    let base_color = dark_band * (1.0 - band) + light_band * band;

    // Add swirl
    let swirled_color = base_color + Vec3::new(swirl, swirl * 0.5, -swirl * 0.3);

    // Add color variation
    let varied_color = swirled_color
        + Vec3::new(
            color_variation,
            color_variation * 0.5,
            -color_variation * 0.3,
        );

    // Add red spot
    let final_base = varied_color * (1.0 - spot) + red_spot * spot;

    // Apply lighting
    let final_color = final_base * (intensity * 0.7 + 0.3);

    Color::from_float(
        final_color.x.clamp(0.0, 1.0),
        final_color.y.clamp(0.0, 1.0),
        final_color.z.clamp(0.0, 1.0),
    )
}

// Moon Shader (simple gray with craters)
pub fn moon_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let intensity = dot(&normal, &light_dir).max(0.0);

    // Base gray color
    let base_gray = 0.5;

    // Add crater-like noise
    let craters = fbm(
        Vec3::new(position.x * 8.0, position.y * 8.0, position.z * 8.0),
        4,
    );
    let crater_depth = (craters - 0.5).abs() * 2.0;
    let crater = if crater_depth > 0.7 {
        crater_depth * 0.3
    } else {
        0.0
    };

    let gray = (base_gray - crater).clamp(0.2, 0.8);

    // Apply lighting
    let final_gray = gray * (intensity * 0.9 + 0.1);

    Color::from_float(final_gray, final_gray, final_gray)
}

// Ring Shader (simple gradient)
pub fn ring_shader(
    v1: &Vertex,
    v2: &Vertex,
    v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.0, 0.0, -1.0);
    let intensity = dot(&normal, &light_dir).max(0.0);

    // Use texture coordinates for radial gradient
    let radial = tex_coords.y; // 0.0 = inner, 1.0 = outer

    // Dusty brown-gray color
    let inner_color = Vec3::new(0.4, 0.35, 0.3);
    let outer_color = Vec3::new(0.5, 0.45, 0.4);

    let color = Vec3::new(
        inner_color.x * (1.0 - radial) + outer_color.x * radial,
        inner_color.y * (1.0 - radial) + outer_color.y * radial,
        inner_color.z * (1.0 - radial) + outer_color.z * radial,
    );

    // Add some variation
    let variation = fbm(
        Vec3::new(position.x * 10.0, position.y * 10.0, position.z * 10.0),
        2,
    ) * 0.1;
    let final_color = Vec3::new(
        color.x + variation,
        color.y + variation,
        color.z + variation,
    );

    // Apply lighting with transparency effect
    let light_factor = intensity * 0.6 + 0.4;
    let ring_final = Vec3::new(
        final_color.x * light_factor,
        final_color.y * light_factor,
        final_color.z * light_factor,
    );

    Color::from_float(
        ring_final.x.clamp(0.0, 1.0),
        ring_final.y.clamp(0.0, 1.0),
        ring_final.z.clamp(0.0, 1.0),
    )
}

pub fn ship_shader(
    _v1: &Vertex,
    _v2: &Vertex,
    _v3: &Vertex,
    position: Vec3,
    normal: Vec3,
    _tex_coords: Vec2,
) -> Color {
    let light_dir = Vec3::new(0.3, -0.8, -0.5).normalize();
    let intensity = dot(&normal.normalize(), &light_dir).max(0.0);

    let base_gray = Vec3::new(0.58, 0.6, 0.63);
    let dark_plate = Vec3::new(0.25, 0.27, 0.3);
    let panel_variation = fbm(
        Vec3::new(position.x * 7.0, position.y * 7.0, position.z * 7.0),
        3,
    )
        .clamp(0.0, 1.0);
    let panel_color = dark_plate * (1.0 - panel_variation) + base_gray * panel_variation;

    let edge_highlight = (normal.y * 0.5 + 0.5).powf(6.0) * 0.25;
    let engine_glow = (position.y * 0.4).sin().abs() * 0.05;

    let specular = normal.normalize().z.max(0.0).powi(6) * 0.5;
    let lit = panel_color * (intensity * 0.65 + 0.35) + Vec3::new(edge_highlight, edge_highlight, edge_highlight);
    let final_color = Vec3::new(
        (lit.x + specular + engine_glow).clamp(0.0, 1.0),
        (lit.y + specular + engine_glow).clamp(0.0, 1.0),
        (lit.z + specular + engine_glow).clamp(0.0, 1.0),
    );

    Color::from_float(final_color.x, final_color.y, final_color.z)
}

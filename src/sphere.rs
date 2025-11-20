use crate::vertex::Vertex;
use nalgebra_glm::{Vec2, Vec3};

pub fn generate_sphere(radius: f32, segments: u32) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    let u_segments = segments;
    let v_segments = segments;

    for i in 0..=v_segments {
        let v = i as f32 / v_segments as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for j in 0..=u_segments {
            let u = j as f32 / u_segments as f32;
            let phi = u * 2.0 * std::f32::consts::PI;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            let position = Vec3::new(x, y, z);
            let normal = position.normalize();
            let tex_coords = Vec2::new(u, v);

            vertices.push(Vertex::new(position, normal, tex_coords));
        }
    }

    // Generate indices for triangles
    let mut indexed_vertices = Vec::new();

    for i in 0..v_segments {
        for j in 0..u_segments {
            let current = (i * (u_segments + 1) + j) as usize;
            let next = (i * (u_segments + 1) + j + 1) as usize;
            let below = ((i + 1) * (u_segments + 1) + j) as usize;
            let below_next = ((i + 1) * (u_segments + 1) + j + 1) as usize;

            // First triangle
            indexed_vertices.push(vertices[current].clone());
            indexed_vertices.push(vertices[below].clone());
            indexed_vertices.push(vertices[next].clone());

            // Second triangle
            indexed_vertices.push(vertices[next].clone());
            indexed_vertices.push(vertices[below].clone());
            indexed_vertices.push(vertices[below_next].clone());
        }
    }

    indexed_vertices
}

pub fn generate_ring(inner_radius: f32, outer_radius: f32, segments: u32) -> Vec<Vertex> {
    let mut vertices = Vec::new();

    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        // Outer vertex
        let outer_pos = Vec3::new(outer_radius * cos_a, 0.0, outer_radius * sin_a);
        let outer_normal = Vec3::new(0.0, 1.0, 0.0);
        let outer_tex = Vec2::new(i as f32 / segments as f32, 1.0);
        vertices.push(Vertex::new(outer_pos, outer_normal, outer_tex));

        // Inner vertex
        let inner_pos = Vec3::new(inner_radius * cos_a, 0.0, inner_radius * sin_a);
        let inner_normal = Vec3::new(0.0, 1.0, 0.0);
        let inner_tex = Vec2::new(i as f32 / segments as f32, 0.0);
        vertices.push(Vertex::new(inner_pos, inner_normal, inner_tex));
    }

    // Generate triangles
    let mut indexed_vertices = Vec::new();

    for i in 0..segments {
        let base = (i * 2) as usize;
        let next_base = ((i + 1) * 2) as usize;

        // First triangle
        indexed_vertices.push(vertices[base].clone());
        indexed_vertices.push(vertices[next_base].clone());
        indexed_vertices.push(vertices[base + 1].clone());

        // Second triangle
        indexed_vertices.push(vertices[base + 1].clone());
        indexed_vertices.push(vertices[next_base].clone());
        indexed_vertices.push(vertices[next_base + 1].clone());
    }

    indexed_vertices
}

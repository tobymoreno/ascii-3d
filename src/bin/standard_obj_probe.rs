use std::{env, process};

#[path = "../math/mod.rs"]
mod math;

#[path = "../mesh.rs"]
mod mesh;

#[path = "../obj.rs"]
mod obj;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "assets/models/cube.obj".to_string());

    match obj::load_obj(&path) {
        Ok(mesh) => {
            println!("path={path}");
            println!("vertices={}", mesh.vertices.len());
            println!("faces={}", mesh.faces.len());
            println!("unique_edges={}", mesh.unique_edges().len());

            println!();
            println!("first_vertices:");
            for (index, vertex) in mesh.vertices.iter().take(5).enumerate() {
                println!("{index}: {}, {}, {}", vertex.x, vertex.y, vertex.z);
            }

            println!();
            println!("first_edges:");
            for (a, b) in mesh.unique_edges().iter().take(12) {
                println!("{a}-{b}");
            }
        }
        Err(error) => {
            eprintln!("standard_obj_probe error: {error}");
            process::exit(1);
        }
    }
}

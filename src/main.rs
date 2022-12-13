use std::{path::Path, thread::sleep, time::Duration};

use crate::mm_io::Mapping;

mod mm_io;

#[derive(Debug, Clone)]
struct Dummy8192Page {
    id: i32,
    items: [i32; 2047],
}

impl Default for Dummy8192Page {
    fn default() -> Self {
        Self {
            id: Default::default(),
            items: [0; 2047],
        }
    }
}

fn main() {
    let mut mapping = Mapping::new(Path::new("database.db")).unwrap();
    let p = Dummy8192Page {
        id: 1,
        items: [0; 2047],
    };
    let idx = 384756;

    mapping.insert_at(&p, idx).unwrap();

    for i in 0..100 {
        mapping.insert_at(&p, i * 3800).unwrap();
        sleep(Duration::from_secs(1));
    }

    let p1 = Dummy8192Page::default();
    let p1 = mapping.read_at(p1, idx).unwrap();

    println!("p1={{id={}, items_size={}}}", p1.id, p1.items.len());
}

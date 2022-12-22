use std::{mem, path::Path, thread::sleep, time::Duration};

use crate::mm_io::Mapping;

mod mm_io;
mod page;

#[derive(Debug, Clone)]
struct Dummy8KbPage {
    id: i32,
    items: [i32; 2047],
}

impl Default for Dummy8KbPage {
    fn default() -> Self {
        Self {
            id: Default::default(),
            items: [0; 2047],
        }
    }
}

fn main() {
    assert_eq!(mem::size_of::<Dummy8KbPage>(), 8192);

    let mut mapping = Mapping::new(Path::new("database.db")).unwrap();

    let p = Dummy8KbPage {
        id: 1,
        items: [0; 2047],
    };
    let idx = 384756;

    mapping.insert_at(&p, idx).unwrap();

    for i in 0..100 {
        mapping.insert_at(&p, i * 3800).unwrap();
        sleep(Duration::from_secs(1));
    }

    let p1 = mapping.read_at(idx).unwrap();

    println!("p1={{id={}, items_size={}}}", p1.id, p1.items.len());
}

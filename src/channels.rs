use std::{thread::spawn, sync::Mutex, ops::AddAssign};

pub fn test_threads() {
    let mut x = 0u128;
    for i in 1..500 {
        x += i;
    }
    println!("{x}");
}

pub fn spawn_thread() {
    let value = 500;
    let thread_fn = move || {
        let mut x = 0u128;
        for i in 1..value {
            x += i;
        }
        println!("{x}");
    };
    let handle = spawn(thread_fn).join();
    println!("handle: {:?}", handle);
}

pub fn test_mutex() {
    let m = Mutex::new(5u16);
    {
        let lock = m.try_lock()
        .and_then(|mut data| {
            *data += 5;
            Ok(data)
        });
        lock.unwrap().add_assign(12);
    }

    println!("m = {:?}", m.into_inner());
}

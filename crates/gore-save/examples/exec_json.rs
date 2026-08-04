//! Run one core request against the library and print the raw JSON response.
//!
//! Usage: exec_json <request.json>   (or pipe the request on stdin)

use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let request = match args.get(1) {
        Some(path) => std::fs::read_to_string(path).expect("read request file"),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("read stdin");
            buf
        }
    };
    println!("{}", gore_save::execute_json(&request));
}

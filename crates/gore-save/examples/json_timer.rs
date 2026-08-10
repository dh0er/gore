//! Scratch research driver: run one `execute_json` request from a file and print
//! how long it took. Used to measure save-editor write latency. Not shipped.

fn main() {
    let path = std::env::args().nth(1).expect("usage: json_timer <request.json>");
    let input = std::fs::read_to_string(&path).expect("read request");
    let repeats: usize = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let mut last = String::new();
    for i in 0..repeats {
        let start = std::time::Instant::now();
        last = gore_save::execute_json(&input);
        eprintln!("run {i}: {:?}", start.elapsed());
    }
    println!("{last}");
}

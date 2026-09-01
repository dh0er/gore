fn main() {
    let path = std::env::args().nth(1).expect("save path");
    let request = serde_json::json!({
        "command": "private.factions.list",
        "payload": {"path": path}
    });
    println!("{}", gore_save::execute_json(&request.to_string()));
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: exec_json <request.json>");
    let request = std::fs::read_to_string(&path).expect("read request");
    let response = gore_save::execute_json(&request);
    let parsed: serde_json::Value = serde_json::from_str(&response).expect("json response");
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
}

fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&device_manager::scan(&device_manager::SystemRunner)).unwrap()
    );
}

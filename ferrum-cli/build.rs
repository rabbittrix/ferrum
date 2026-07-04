fn main() {
    let date = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    println!("cargo:rustc-env=FERRUM_BUILD_DATE={date}");
    println!("cargo:rerun-if-changed=build.rs");
}

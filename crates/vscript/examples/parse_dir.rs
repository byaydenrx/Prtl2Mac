//! Parse every `.nut` file in a directory and report errors:
//! `cargo run -p vscript --example parse_dir -- DIR`
fn main() {
    let dir = std::env::args().nth(1).expect("directory");
    let (mut ok, mut bad) = (0, 0);
    for e in std::fs::read_dir(&dir).unwrap().flatten() {
        let p = e.path();
        let Ok(src) = std::fs::read_to_string(&p) else { continue };
        match vscript::parser::parse(&src) {
            Ok(_) => ok += 1,
            Err(err) => {
                bad += 1;
                println!("{}: {err}", p.file_name().unwrap().to_string_lossy());
            }
        }
    }
    println!("{ok} parsed, {bad} failed");
}

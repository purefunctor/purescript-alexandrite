fn main() {
    if std::env::args().nth(1).as_deref() == Some("nextest") {
        return;
    }

    eprintln!("snapshot inspection failed");
    std::process::exit(19);
}

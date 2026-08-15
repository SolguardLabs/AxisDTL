fn main() {
    if let Err(error) = axis_dtl::run_cli() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

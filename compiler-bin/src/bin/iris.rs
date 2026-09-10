fn main() {
    // SAFETY: Startup is single-threaded; no worker threads have been started.
    unsafe { purescript_iris::cli::initialize_terminal_width() };

    purescript_iris::run();
}

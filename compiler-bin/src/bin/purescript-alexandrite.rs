fn main() {
    // SAFETY: Startup is single-threaded; no worker threads have been started.
    unsafe { purescript_alexandrite::cli::initialize_terminal_width() };

    purescript_alexandrite::run();
}

fn main() {
    // SAFETY: Startup is single-threaded; no worker threads have been started.
    unsafe { purescript_alexandrite::cli::initialize_terminal_width() };

    eprintln!(
        "warning: `purescript-analyzer` is deprecated; use `purescript-alexandrite` instead.",
    );
    purescript_alexandrite::run();
}

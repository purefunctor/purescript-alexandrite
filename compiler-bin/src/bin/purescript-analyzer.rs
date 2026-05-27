fn main() {
    eprintln!(
        "warning: `purescript-analyzer` is deprecated; use `purescript-alexandrite` instead.",
    );
    purescript_alexandrite::run();
}

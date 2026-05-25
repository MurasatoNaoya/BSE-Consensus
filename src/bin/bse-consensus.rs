fn main() {
    std::process::exit(bse_consensus::cli::run(
        std::env::args(),
        &mut std::io::stdin().lock(),
        &mut std::io::stdout().lock(),
    ));
}

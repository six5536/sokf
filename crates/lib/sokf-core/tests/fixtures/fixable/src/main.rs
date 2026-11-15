//! A fixture source file.

// sokf:begin cli
fn cli() -> &'static str {
    "sokf validate"
}
// sokf:end cli

fn main() {
    println!("{}", cli());
}

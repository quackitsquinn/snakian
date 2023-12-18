
const CHAR_RS_PATH: &str = "src/display/chars.rs";

/// Create chars.rs from font8x8_basic.h
fn main() {
    println!("cargo:rerun-if-changed=chars/font8x8_basic.h");
    let fnt8x8 = include_str!("chars/font8x8_basic.h");

    // find the first { and the last }
    let start = fnt8x8.find('{').unwrap();
    let end = fnt8x8.rfind('}').unwrap();

    // get the chars with the { and }
    let chars = &fnt8x8[start..end + 1];
    // replace curly braces with square braces
    let chars = chars.replace('{', "[");
    let chars = chars.replace('}', "]");

    // load chars.rs
    let chars_rs = String::from(std::fs::read_to_string(CHAR_RS_PATH).unwrap());

    // preserve the top of chars.rs for any declarations
    let mut before_header = chars_rs
        .split("// _BEGIN_CHARS_")
        .next()
        .unwrap()
        .to_string();

    // re-add the header to the top of chars.rs

    before_header.push_str("// _BEGIN_CHARS_ \n");

    let mut content = String::new();

    // add char declr
    content.push_str("pub const CHARS: [[u8; 8]; 128] = ");

    // add the chars
    content.push_str(&chars);

    // add the semicolon
    content.push_str(";\n");

    // write the chars.rs file
    std::fs::write(CHAR_RS_PATH, before_header + &content).unwrap();
}

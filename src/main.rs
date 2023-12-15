use std::path::PathBuf;


/// Make the bootimage for both BIOS and UEFI
pub fn main() {
    println!("UEFI disk image at {}", env!("UEFI_IMAGE"));
    println!("BIOS disk image at {}", env!("BIOS_IMAGE"));
}   
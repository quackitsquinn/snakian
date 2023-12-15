use std::{path::{PathBuf, Path}, env, fs};

use bootloader::{UefiBoot, BiosBoot, BootConfig};


fn main() {
    // get out directory
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let kdir = env::var("CARGO_BIN_FILE_SNAKIAN_KERNEL").unwrap();

    let uefipath = Path::new(&out_dir).join("snakian-uefi.img");
    let biospath = Path::new(&out_dir).join("snakian-bios.img");

    let uefi = UefiBoot::new(Path::new(&kdir));
    uefi.create_disk_image(&uefipath).unwrap();
    let bios = BiosBoot::new(Path::new(&kdir));
    bios.create_disk_image(&biospath).unwrap();

    let target_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("target").join(env::var("PROFILE").unwrap());

    let uefi_out = target_dir.join("snakian-uefi.img");
    let bios_out = target_dir.join("snakian-bios.img");

    fs::copy(&uefipath, &uefi_out).unwrap();
    fs::copy(&biospath, &bios_out).unwrap();

    println!("cargo:rustc-env=UEFI_IMAGE={}", uefi_out.display());
    println!("cargo:rustc-env=BIOS_IMAGE={}", bios_out.display());


    
}
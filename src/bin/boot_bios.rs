use std::process::{Command, exit};




pub fn main()  {
    let mut qcom = Command::new("qemu-system-x86_64")
        .arg("-drive")
        .arg(format!("format=raw,file={}", env!("BIOS_IMAGE")))
        .arg("-serial")
        .arg("stdio")
        .arg("-device")
        .arg("isa-debug-exit,iobase=0xf4,iosize=0x04")
        .spawn()
        .expect("failed to start qemu");
    exit(qcom.wait().unwrap().code().unwrap());
}
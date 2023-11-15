use crate::{print, println};



#[cfg(test)]
pub fn test_runner(tests: &[&dyn Fn()]) {

    use crate::{test_util, println};

    println!("Running {} tests", tests.len());
    for mut test in tests {
        test.run();
    } 
    test_util::exit_qemu(test_util::QemuExitCode::Success);
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T where T:Fn() {
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}
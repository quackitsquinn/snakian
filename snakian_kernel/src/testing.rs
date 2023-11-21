use core::panic::PanicInfo;

use crate::{print, println, serial_println};



pub fn test_runner(tests: &[&dyn Fn()]) {

    use crate::{testing, println};

    serial_println!("Running {} tests", tests.len());
    for mut test in tests {
        test.run();
    } 
    testing::exit_qemu(testing::QemuExitCode::Success);
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

pub fn panic_handler(panic: &PanicInfo) -> ! {

    serial_println!("Kernal Panic in file {} at line {}", panic.location().unwrap().file(), panic.location().unwrap().line());
    serial_println!("Reason:{}", panic.message().unwrap());
    exit_qemu(QemuExitCode::Failed);
    loop {} // if qemu doesn't exit
}

#[macro_export]
macro_rules! test_main {
    () => {
        #[no_mangle]
        pub extern "C" fn _start() -> ! {
            test_main();
            loop {}
        }
    };
}
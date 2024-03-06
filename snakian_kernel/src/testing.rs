use core::panic::PanicInfo;

use bootloader_api::{config::Mapping, BootInfo, BootloaderConfig};

use crate::{print, println, serial_println};

pub fn test_runner(tests: &[&dyn Fn()]) {
    use crate::testing;

    serial_println!("Running {} tests", tests.len());
    for test in tests {
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

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn panic_handler(panic: &PanicInfo) -> ! {
    serial_println!(
        "Kernal Panic in file {} at line {}",
        panic.location().unwrap().file(),
        panic.location().unwrap().line()
    );
    serial_println!("Reason:{}", panic.message().unwrap());
    exit_qemu(QemuExitCode::Failed);
    loop {} // if qemu doesn't exit
}

pub static TEST_BOOT_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 512 * 1024; // we need a lot of space for the vga buffer
    config
};

#[macro_export]
macro_rules! test_setup {
    () => {
        #[no_mangle]
        pub fn snakian_test_entry(_: &'static mut ::bootloader_api::BootInfo) -> ! {
            test_main();
            loop {}
        }

        #[panic_handler]
        fn panic(info: &PanicInfo) -> ! {
            snakian_kernel::testing::panic_handler(info)
        }

        bootloader_api::entry_point!(
            snakian_test_entry,
            config = &snakian_kernel::testing::TEST_BOOT_CONFIG
        );
    };
}

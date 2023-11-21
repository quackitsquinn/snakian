#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(snakian::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

snakian::test_main!();

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    snakian::testing::panic_handler(info)
}

#[test_case]
fn test_test_runner() {
    assert_eq!(1, 1);
}

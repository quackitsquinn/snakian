#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(snakian_kernel::testing::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

snakian_kernel::test_setup!();

#[test_case]
fn test_test_runner() {
    assert_eq!(1, 1);
}

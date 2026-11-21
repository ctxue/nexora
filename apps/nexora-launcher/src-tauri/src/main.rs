#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if nexora_lib::run_native_host_if_requested() {
        return;
    }
    nexora_lib::run();
}

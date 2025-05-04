#![no_std]
#![no_main]
#![feature(offset_of)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use core::panic::PanicInfo;
use x86_64::structures::paging::FrameAllocator;

mod font;
mod graphics;
use graphics::{FrameBuffer, COLOR_BLACK, COLOR_BLUE, COLOR_GREEN, COLOR_RED, COLOR_WHITE};

mod efi;
use efi::{EfiHandle, EfiSystemTable, framebuffer, MemoryMapHolder, EfiStatus};

mod gdt;
mod interrupts;
mod memory;

use alloc::vec::Vec;
use memory::BitmapFrameAllocator;
use alloc::format;

// ------------------------------------------------------------
// 簡易 UI モジュール（暫定）
mod ui {
    use crate::graphics::{FrameBuffer, COLOR_BLUE, COLOR_WHITE, COLOR_RED, COLOR_GREEN};

    /// ホーム画面を描画
    pub fn home(fb: &mut FrameBuffer) {
        // 背景
        fb.clear(COLOR_BLUE);

        // テキスト
        let label = "Hello, World!";
        let text_width = label.len() * 8 + (label.len() - 1) * 2;
        let x = (fb.width - text_width) / 2;
        let y = fb.height / 2 - 4;
        fb.draw_text(x, y, label, COLOR_WHITE);

        // 図形例 (円・矩形など)
        fb.fill_rect(20, 20, 80, 50, COLOR_RED);
        fb.stroke_rect(fb.width - 140, 30, 120, 80, COLOR_GREEN);
        fb.fill_circle((fb.width / 2) as isize, (fb.height * 3 / 4) as isize, 40, COLOR_GREEN);
    }
}
// ------------------------------------------------------------

#[no_mangle]
fn efi_main(image_handle: EfiHandle, system_table: &EfiSystemTable) {
    let mut fb = framebuffer(system_table).expect("GOP unavailable");

    // ホーム画面を描画
    ui::home(&mut fb);
    
    // 1 秒待機（1,000,000 マイクロ秒）
    let _ = (system_table.boot_services.stall)(1_000_000usize);

    // BootServices ポインタ (ExitBootServices 前)
    let _bs_before = system_table.boot_services as *const _ as usize;

    // BootServices との決別: ExitBootServices を呼び出す
    let mut mmap = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, system_table, &mut mmap);


    fb.clear(COLOR_WHITE);
    // CPU 初期化: GDT/TSS・IDT 設定
    gdt::init();
    fb.draw_text(10, 10, "GDT OK", COLOR_BLACK);
    interrupts::init();
    fb.draw_text(10, 20, "IDT OK", COLOR_BLACK);
    unsafe { memory::init_paging(); }
    fb.draw_text(10, 30, "Paging Init OK", COLOR_BLACK);
    unsafe { paging_smoke_test(&mut fb); }
    fb.draw_text(10, 40, "Paging Test Done", COLOR_BLACK);
    unsafe { memory::init_heap(); }
    fb.draw_text(10, 50, "Heap Init OK", COLOR_BLACK);
    fb.draw_text(10, 60, "Heap Test Done", COLOR_BLACK);

    // 動的確保テスト: reserve 1KiB 分の Vec
    let mut test_vec: Vec<u64> = Vec::new();
    let msg;
    let color;
    match test_vec.try_reserve_exact(1024) {
        Ok(_) => {
            msg = "Heap OK";
            color = COLOR_GREEN;
        }
        Err(_) => {
            msg = "Heap NG";
            color = COLOR_RED;
        }
    }

    // fb.clear(COLOR_RED);
    // 結果を描画
    let msg_w = msg.len() * 8 + (msg.len() - 1) * 2;
    let hx = (fb.width - msg_w) / 2;
    let hy = fb.height / 2 - 4 + 16;
    fb.draw_text(hx, hy, msg, color);
    fb.draw_text(10, 70, "Heap Draw Done", COLOR_BLACK);

    // 物理フレームアロケータテスト
    fb.draw_text(10, 110, "Allocator Init Start", COLOR_BLACK); // 目印

    let mut fa_ok = false;
    let mut fa; // unsafe ブロックの外で宣言
    unsafe {
        // fa = BitmapFrameAllocator::new(&mmap); // <<< &mut fb が必要
        fa = BitmapFrameAllocator::new(&mmap, &mut fb);
        let f1 = fa.allocate_frame();
        let f2 = fa.allocate_frame();
        fa_ok = f1.is_some() && f2.is_some() && f1 != f2;
    }

    let msg = if fa_ok { "FrameAlloc OK" } else { "FrameAlloc NG" };
    let color = if fa_ok { COLOR_GREEN } else { COLOR_RED };
    let msg_w = msg.len()*8 + (msg.len()-1)*2;
    let fx = (fb.width - msg_w)/2;
    let fy = fb.height/2 - 4 + 32;
    fb.draw_text(fx, fy, msg, color);

    // 以降は Non-UEFI 世界。画面をクリアしてメッセージ表示
    // fb.clear(COLOR_RED);
    // let label = "Hello, NonUEFI!";
    // let text_width = label.len() * 8 + (label.len() - 1) * 2;
    // let x = (fb.width - text_width) / 2;
    // let y = fb.height / 2 - 4;
    // fb.draw_text(x, y, label, COLOR_WHITE);

    loop {}
}

/// ExitBootServices を安全に呼び出すヘルパ
fn exit_from_efi_boot_services(
    image_handle: EfiHandle,
    system_table: &EfiSystemTable,
    map_holder: &mut MemoryMapHolder,
) {
    loop {
        let st = system_table.boot_services;
        let status = st.call_get_memory_map(map_holder);
        assert_eq!(status, EfiStatus::Success);

        let status = (st.exit_boot_services)(image_handle, map_holder.map_key);
        if status == EfiStatus::Success {
            break;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    x86_64::instructions::hlt();
    loop {}
}

// ===== テスト関数 =====
unsafe fn paging_smoke_test(fb: &mut FrameBuffer) {
    x86_64::instructions::interrupts::disable();
    let test_addr = 0x3FF0_0000 as *mut u64; // 4GiB-1MiB
    test_addr.write_volatile(0xDEAD_BEEF_DEAD_BEEF);
    let ok = test_addr.read_volatile() == 0xDEAD_BEEF_DEAD_BEEF;
    x86_64::instructions::interrupts::enable();

    let msg = if ok { "Paging OK" } else { "Paging NG" };
    let color = if ok { COLOR_GREEN } else { COLOR_RED };
    let msg_w = msg.len()*8 + (msg.len()-1)*2;
    let x = (fb.width - msg_w)/2;
    let y = fb.height/2 - 4 - 16; // 既存メッセージ上
    fb.draw_text(x, y, msg, color);
}

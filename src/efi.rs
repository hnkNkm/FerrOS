#![allow(dead_code)]
//! UEFI 関連の FFI 構造体と GPU 初期化ラッパ。
//! 
//! - SystemTable から Graphics Output Protocol(GOP) を取得し、
//!   フレームバッファを `graphics::FrameBuffer` として返すユーティリティを提供。
//! - 低レベル構造体は public にしているため、`efi_main` のシグネチャにも再利用可能。

use core::mem::{offset_of, size_of};
use core::ptr::null_mut;
use crate::graphics::FrameBuffer;

pub type Result<T> = core::result::Result<T, &'static str>;

pub type EfiVoid = u8;
/// UEFI が渡すイメージハンドル型
pub type EfiHandle = u64;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EfiGuid {
    pub data0: u32,
    pub data1: u16,
    pub data2: u16,
    pub data3: [u8; 8],
}

/// Graphics Output Protocol GUID
pub const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x9042a9de,
    data1: 0x23dc,
    data2: 0x4a38,
    data3: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[must_use]
#[repr(u64)]
pub enum EfiStatus {
    Success = 0,
}

#[repr(C)]
pub struct EfiBootServicesTable {
    _reserved0: [u64; 40],
    pub locate_protocol: extern "win64" fn(
        protocol: *const EfiGuid,
        registration: *const EfiVoid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
}
const _: () = assert!(offset_of!(EfiBootServicesTable, locate_protocol) == 320);

#[repr(C)]
pub struct EfiSystemTable {
    _reserved0: [u64; 12],
    pub boot_services: &'static EfiBootServicesTable,
}
const _: () = assert!(offset_of!(EfiSystemTable, boot_services) == 96);

#[repr(C)]
#[derive(Debug)]
pub struct EfiGraphicsOutputProtocolPixelInfo {
    version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    _padding0: [u32; 5],
    pub pixels_per_scan_line: u32,
}
const _: () = assert!(size_of::<EfiGraphicsOutputProtocolPixelInfo>() == 36);

#[repr(C)]
#[derive(Debug)]
pub struct EfiGraphicsOutputProtocolMode<'a> {
    pub max_mode: u32,
    pub mode: u32,
    pub info: &'a EfiGraphicsOutputProtocolPixelInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: usize,
    pub frame_buffer_size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct EfiGraphicsOutputProtocol<'a> {
    reserved: [u64; 3],
    pub mode: &'a EfiGraphicsOutputProtocolMode<'a>,
}

/// `SystemTable` から GOP を検索し、`FrameBuffer` を生成して返す
pub fn framebuffer<'a>(system_table: &'a EfiSystemTable) -> Result<FrameBuffer<'a>> {
    // GOP へのポインタ
    let mut gop_ptr = null_mut::<EfiGraphicsOutputProtocol>();
    let status = (system_table.boot_services.locate_protocol)(
        &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
        null_mut::<EfiVoid>(),
        &mut gop_ptr as *mut *mut EfiGraphicsOutputProtocol as *mut *mut EfiVoid,
    );

    if status != EfiStatus::Success {
        return Err("Failed to locate graphics output protocol");
    }
    // Safety: UEFI が有効な GOP を返したと仮定
    let gop = unsafe { &*gop_ptr };

    let vram_addr = gop.mode.frame_buffer_base;
    let vram_byte_size = gop.mode.frame_buffer_size;
    let width = gop.mode.info.horizontal_resolution as usize;
    let height = gop.mode.info.vertical_resolution as usize;

    // Safety: フレームバッファ領域は UEFI により確保済みで、32bit ピクセルが連続して並ぶ
    let vram_slice = unsafe {
        core::slice::from_raw_parts_mut(
            vram_addr as *mut u32,
            vram_byte_size / size_of::<u32>(),
        )
    };

    Ok(FrameBuffer::new(vram_slice, width, height))
} 
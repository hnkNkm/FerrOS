use bitvec::vec::BitVec;
use bitvec::prelude::*;
use x86_64::structures::paging::{PhysFrame, FrameAllocator, Size4KiB};
use x86_64::PhysAddr;
use crate::efi::{MemoryMapHolder, EfiMemoryDescriptor};
use crate::graphics::{FrameBuffer, COLOR_BLACK, COLOR_RED, COLOR_GREEN};

const BITMAP_STORAGE_SIZE_BYTES: usize = 102400; // 100 KiB
static mut BITMAP_STORAGE: [u8; BITMAP_STORAGE_SIZE_BYTES] = [0; BITMAP_STORAGE_SIZE_BYTES];

// BitVec の代わりにライフタイム付きの BitSlice を保持
pub struct BitmapFrameAllocator<'a> {
    bitmap: &'a mut BitSlice<u8, Lsb0>,
    frame_count: usize, // アロケーション時に境界チェックするため保持
}

impl<'a> BitmapFrameAllocator<'a> {
    /// Safety: `holder` は ExitBootServices 前に取得したメモリマップであること。
    pub unsafe fn new(holder: &MemoryMapHolder, fb: &mut FrameBuffer) -> Self { // fb を再度使う

        // --- Debug Start ---
        if holder.descriptor_size == 0 {
             panic!("EFI descriptor_size is zero!");
        }
        // --- Debug End ---

        let num_desc = holder.memory_map_size / holder.descriptor_size;
        let mut max_addr = 0u64;
        let mut walk_desc = |mut f: &mut dyn FnMut(&EfiMemoryDescriptor)| {
            for i in 0..num_desc {
                let ptr = holder.memory_map_buffer.as_ptr().add(i * holder.descriptor_size)
                    as *const EfiMemoryDescriptor;
                // Safety: buffer contains at least descriptor_size bytes for each entry
                let desc = unsafe { &*ptr };
                f(desc);
            }
        };


        walk_desc(&mut |d| {
            let end = d.physical_start + d.number_of_pages * 4096;
            // 暫定対応: 4GiB を超えるアドレスは無視
            if end <= 0x1_0000_0000 && end > max_addr {
                 max_addr = end;
            }
        });
        let frame_count = (max_addr as usize / 4096) as usize;

        
        // 静的ストレージから BitVec を作成
        let bytes_needed = (frame_count + 7) / 8;
        if bytes_needed > BITMAP_STORAGE_SIZE_BYTES {
            // ここで panic する代わりにエラー処理をするのが望ましい
            panic!("Bitmap storage too small! Needed: {}, Available: {}", bytes_needed, BITMAP_STORAGE_SIZE_BYTES);
        }
        // Safety: BITMAP_STORAGE は static mut だが、この new 関数が呼び出されるのは
        // OS 初期化中のシングルスレッド環境であり、他から同時にアクセスされることはない。
        // また、サイズチェックにより境界外アクセスは発生しない。
        let storage_slice = unsafe { &mut BITMAP_STORAGE[..bytes_needed] };
        // ミュータブルな BitSlice を作成
        let bitmap = BitSlice::from_slice_mut(storage_slice);

        // 全ビットを true (使用中) で初期化
        bitmap.fill(true);

        const EFI_CONVENTIONAL: u32 = 7;

        // 利用可能なフレーム (EfiConventionalMemory) のビットを false (未使用) に設定
        walk_desc(&mut |d| {
            if d.memory_type == EFI_CONVENTIONAL {
                let start_frame = (d.physical_start / 4096) as usize;
                let end_frame = start_frame + d.number_of_pages as usize;
                for i in start_frame..end_frame {
                    if i < frame_count { // 念のため境界チェック (4GiB キャップで不要かもしれないが一応)
                         // 範囲外アクセスチェック (デバッグ用、以前有効化していたもの)
                         // if i >= bitmap.len() {
                         //     panic!("Bitmap index out of bounds: {} >= {}", i, bitmap.len());
                         // }
                        bitmap.set(i, false); // <<< 問題があった箇所のはず
                    }
                }
            }
        });

        // fb.draw_text(10, 160, "Bitmap Init Loop Done", COLOR_GREEN); // ループ完了確認 (前)
        // x86_64::instructions::hlt(); // <<< ここに hlt を移動 (ループ完了確認用)

        // fb.draw_text(10, 180, "HLT Executed (Should not see)", COLOR_RED); // hlt 実行確認 (後)

        Self { bitmap, frame_count }
    }

    pub fn count_free_frames(&self) -> usize {
        self.bitmap.iter().filter(|b| !**b).count()
    }

    fn mark_used(&mut self, index: usize) {
        self.bitmap.set(index, true);
    }
}

unsafe impl<'a> FrameAllocator<Size4KiB> for BitmapFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        for idx in 0..self.frame_count { // bitmap.len() の代わりに保持している frame_count を使う
            if !self.bitmap[idx] {
                self.mark_used(idx);
                let addr = (idx as u64) * 4096;
                return Some(PhysFrame::containing_address(PhysAddr::new(addr)));
            }
        }
        None
    }
} 
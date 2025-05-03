#!/bin/bash -e
PROJ_ROOT="$(dirname $(dirname ${BASH_SOURCE:-$0}))"
cd "${PROJ_ROOT}"

PATH_TO_EFI="$1"
rm -rf mnt
mkdir -p mnt/EFI/BOOT/
cp ${PATH_TO_EFI} mnt/EFI/BOOT/BOOTX64.EFI
qemu-system-x86_64 \
  -m 4G \
  -drive if=pflash,format=raw,readonly=on,file=third_party/ovmf/OVMF_CODE.fd \
  -drive if=pflash,format=raw,file=third_party/ovmf/OVMF_VARS.fd \
  -drive format=raw,file=fat:rw:mnt \
  -vga std \
  -net none \
  -global driver=cfi.pflash01,property=secure,value=on \
  -device isa-debug-exit,iobase=0xf4,iosize=0x01 
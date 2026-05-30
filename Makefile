LD = x86_64-elf-ld
NASM = nasm
CARGO = cargo +nightly

BUILD_DIR = build
ISO_DIR = isodir
BOOT_ASM = src/boot/boot.asm
BOOT_OBJ = $(BUILD_DIR)/boot.o
LINKER_SCRIPT = linker.ld
KERNEL_BIN = $(ISO_DIR)/boot/kernel.bin
RUST_LIB = target/x86_64-unknown-none/debug/libkernel.a
ISO = kernel.iso

.PHONY: all clean run rust

all: $(ISO)

$(BUILD_DIR)/boot.o: $(BOOT_ASM)
	mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 $< -o $@

rust:
	$(CARGO) build

$(KERNEL_BIN): $(BOOT_OBJ) rust
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ $(BOOT_OBJ) $(RUST_LIB)

$(ISO): $(KERNEL_BIN)
	grub-mkrescue -o $@ $(ISO_DIR)

run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -serial stdio

clean:
	rm -rf $(BUILD_DIR) $(KERNEL_BIN) $(ISO)
	$(CARGO) clean --package the-os

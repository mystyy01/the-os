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

# Userspace: build, strip to shrink, then patch e_ident[EI_OSABI] (offset 7) to
# 0xAE so the kernel's ELF loader recognises it as a the-os binary.
STRIP = x86_64-elf-strip
USER_DIR = user/hello
USER_RAW = $(USER_DIR)/target/x86_64-unknown-none/debug/hello
USER_ELF = $(USER_DIR)/hello.elf

.PHONY: all clean run rust user

all: $(ISO)

$(BUILD_DIR)/boot.o: $(BOOT_ASM)
	mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 $< -o $@

user:
	cd $(USER_DIR) && $(CARGO) build
	$(STRIP) -s -o $(USER_ELF) $(USER_RAW)
	printf '\xae' | dd of=$(USER_ELF) bs=1 seek=7 count=1 conv=notrunc status=none

rust: user
	$(CARGO) build

$(KERNEL_BIN): $(BOOT_OBJ) rust
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ $(BOOT_OBJ) $(RUST_LIB)

$(ISO): $(KERNEL_BIN)
	grub-mkrescue -o $@ $(ISO_DIR)

run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -serial stdio

clean:
	rm -rf $(BUILD_DIR) $(KERNEL_BIN) $(ISO) $(USER_ELF)
	$(CARGO) clean --package the-os
	cd $(USER_DIR) && $(CARGO) clean

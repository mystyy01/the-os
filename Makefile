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
# goodbye is built BEFORE hello because hello embeds goodbye.elf via include_bytes!.
STRIP = x86_64-elf-strip
TGT = target/x86_64-unknown-none/debug

HELLO_DIR = user/hello
GOODBYE_DIR = user/goodbye
HELLO_ELF = $(HELLO_DIR)/hello.elf
GOODBYE_ELF = $(GOODBYE_DIR)/goodbye.elf

.PHONY: all clean run rust user hello_bin goodbye_bin

all: $(ISO)

$(BUILD_DIR)/boot.o: $(BOOT_ASM)
	mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 $< -o $@

user: hello_bin

goodbye_bin:
	cd $(GOODBYE_DIR) && $(CARGO) build
	$(STRIP) -s -o $(GOODBYE_ELF) $(GOODBYE_DIR)/$(TGT)/goodbye
	printf '\xae' | dd of=$(GOODBYE_ELF) bs=1 seek=7 count=1 conv=notrunc status=none

# Depends on goodbye_bin so goodbye.elf exists when hello's include_bytes! runs.
hello_bin: goodbye_bin
	cd $(HELLO_DIR) && $(CARGO) build
	$(STRIP) -s -o $(HELLO_ELF) $(HELLO_DIR)/$(TGT)/hello
	printf '\xae' | dd of=$(HELLO_ELF) bs=1 seek=7 count=1 conv=notrunc status=none

rust: user
	$(CARGO) build

$(KERNEL_BIN): $(BOOT_OBJ) rust
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ $(BOOT_OBJ) $(RUST_LIB)

$(ISO): $(KERNEL_BIN)
	grub-mkrescue -o $@ $(ISO_DIR)

run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -serial stdio

clean:
	rm -rf $(BUILD_DIR) $(KERNEL_BIN) $(ISO) $(HELLO_ELF) $(GOODBYE_ELF)
	$(CARGO) clean --package the-os
	cd $(HELLO_DIR) && $(CARGO) clean
	cd $(GOODBYE_DIR) && $(CARGO) clean

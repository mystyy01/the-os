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

# Userspace workspace. build_user builds one member, strips it, then patches
# e_ident[EI_OSABI] (offset 7) to 0xAE so the kernel's ELF loader accepts it as a
# the-os binary. Output: user/dist/<name>.elf. Adding a program = one line in the
# `user` target below (plus the crate + workspace member entry).
STRIP = x86_64-elf-strip
USER_DIR = user
USER_TGT = $(USER_DIR)/target/x86_64-unknown-none/debug
USER_DIST = $(USER_DIR)/dist

# $(call build_user,<crate-name>) -> $(USER_DIST)/<crate-name>.elf
define build_user
	cd $(USER_DIR) && $(CARGO) build -p $(1)
	mkdir -p $(USER_DIST)
	$(STRIP) -s -o $(USER_DIST)/$(1).elf $(USER_TGT)/$(1)
	printf '\xae' | dd of=$(USER_DIST)/$(1).elf bs=1 seek=7 count=1 conv=notrunc status=none
endef

.PHONY: all clean run rust user

all: $(ISO)

$(BUILD_DIR)/boot.o: $(BOOT_ASM)
	mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 $< -o $@

# Build order matters here: hello embeds goodbye.elf via include_bytes!, so
# goodbye must exist before hello compiles.
user:
	$(call build_user,goodbye)
	$(call build_user,hello)

rust: user
	$(CARGO) build

$(KERNEL_BIN): $(BOOT_OBJ) rust
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ $(BOOT_OBJ) $(RUST_LIB)

$(ISO): $(KERNEL_BIN)
	grub-mkrescue -o $@ $(ISO_DIR)

run: $(ISO)
	qemu-system-x86_64 -cdrom $(ISO) -serial stdio

clean:
	rm -rf $(BUILD_DIR) $(KERNEL_BIN) $(ISO) $(USER_DIST)
	$(CARGO) clean --package the-os
	cd $(USER_DIR) && $(CARGO) clean

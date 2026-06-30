LD = x86_64-elf-ld
NASM = nasm
CARGO = cargo +nightly

BUILD_DIR = build
LWEXT4_DIR = vendors/lwext4
LWEXT4_BUILD = $(BUILD_DIR)/lwext4
LWEXT4_LIB = $(LWEXT4_BUILD)/src/liblwext4.a
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

.PHONY: all clean run rust user lwext4

all: $(ISO)

$(LWEXT4_LIB):
	mkdir -p $(LWEXT4_BUILD)
	cd $(LWEXT4_BUILD) && cmake $(CURDIR)/$(LWEXT4_DIR) \
		-DCMAKE_TOOLCHAIN_FILE=$(CURDIR)/$(LWEXT4_DIR)/toolchain/x86_64-none.cmake \
		-DLIB_ONLY=1 \
		-DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_POLICY_VERSION_MINIMUM=3.5
	$(MAKE) -C $(LWEXT4_BUILD) lwext4

lwext4: $(LWEXT4_LIB)

$(BUILD_DIR)/boot.o: $(BOOT_ASM)
	mkdir -p $(BUILD_DIR)
	$(NASM) -f elf64 $< -o $@

$(BUILD_DIR)/ap_trampoline.bin: src/boot/ap_trampoline_thingy_haha_i_love_the_word_trampoline.asm
	mkdir -p $(BUILD_DIR)
	$(NASM) -f bin $< -o $@

user: $(LWEXT4_LIB)
	$(call build_user,vfs)
	$(call build_user,kb_driver)
	$(call build_user,shell)
	$(call build_user,ata_pio_driver)
	$(call build_user,fs)
	$(call build_user,the-initializer)

rust: user
	$(CARGO) build

$(KERNEL_BIN): $(BOOT_OBJ) $(BUILD_DIR)/ap_trampoline.bin rust
	mkdir -p $(ISO_DIR)/boot
	$(LD) -n -T $(LINKER_SCRIPT) -o $@ $(BOOT_OBJ) $(RUST_LIB)

$(ISO): $(KERNEL_BIN)
	grub-mkrescue -o $@ $(ISO_DIR)

FSROOT = fsroot

disk.img: $(FSROOT)/hello.txt
	truncate -s 64M disk.img
	mke2fs -q -t ext4 -F -O ^64bit,^metadata_csum,^orphan_file,^has_journal -d $(FSROOT) disk.img

$(FSROOT)/hello.txt:
	mkdir -p $(FSROOT)
	printf 'hello from ext4!\n' > $(FSROOT)/hello.txt

run: $(ISO) disk.img
	qemu-system-x86_64 -smp 2 -cdrom $(ISO) -serial stdio -drive file=disk.img,format=raw,if=ide

clean:
	rm -rf $(BUILD_DIR) $(KERNEL_BIN) $(ISO) $(USER_DIST)
	$(CARGO) clean --package the-os
	cd $(USER_DIR) && $(CARGO) clean

ENTRY(_start)

/* One RWX load segment keeps the kernel's loader path simple: every page is
 * mapped 0x07, and .bss (memsz > filesz) is handled by zero-then-copy. */
PHDRS {
    load PT_LOAD FLAGS(7);
}

SECTIONS {
    /* High virtual base: pml4 index 1 (>= 0x8000000000), clear of the kernel's
     * shared low-512GB mapping (pml4[0]). Each program runs in its own address
     * space, so they may all share this base. Page aligned. */
    . = 0x8000000000;

    .text   : { *(.text .text.*) }          :load
    .rodata : { *(.rodata .rodata.*) }       :load
    .data   : { *(.data .data.*) }           :load
    .bss    : { *(.bss .bss.*) *(COMMON) }   :load

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
    }
}

ENTRY(_start)

/* One RWX load segment keeps the loader path simple: every page is mapped 0x07,
 * and .bss (memsz > filesz) is handled by the loader's zero-then-copy. */
PHDRS {
    load PT_LOAD FLAGS(7);
}

SECTIONS {
    /* Same high base as hello: that is fine, goodbye runs in its OWN address
     * space (sys_spawn calls create_address_space), so there is no collision. */
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

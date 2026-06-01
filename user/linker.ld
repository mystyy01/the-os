ENTRY(_start)

PHDRS {
    load PT_LOAD FLAGS(7);
}

SECTIONS {
    . = 0x400000;

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

ENTRY(_start)

SECTIONS {
    . = 1M;

    kernel_start = .;

    .boot :
    {
        KEEP(*(.multiboot2))
    }

    .text :
    {
        *(.text .text.*)
    }

    .rodata :
    {
        *(.rodata .rodata.*)
    }

    .data :
    {
        *(.data .data.*)
    }

    .bss :
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    kernel_end = .;

    /DISCARD/ : {
        *(.init_array*)
        *(.fini_array*)
        *(.eh_frame*)
        *(.note*)
    }
}

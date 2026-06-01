KERNEL_VMA = 0xFFFFFFFF80000000;

ENTRY(_start)

SECTIONS {
    . = 1M;
    kernel_start = .;

    .boot ALIGN(4K) :
    {
        KEEP(*(.multiboot2))
        *(.boot.text)
        *(.boot.data)
    }

    .boot_bss (NOLOAD) : ALIGN(4K)
    {
        *(.boot.bss)
    }

    . += KERNEL_VMA;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VMA)
    {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VMA)
    {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VMA)
    {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VMA)
    {
        *(.bss .bss.*)
        *(COMMON)
    }

    kernel_end = . - KERNEL_VMA;

    /DISCARD/ :
    {
        *(.init_array*)
        *(.fini_array*)
        *(.eh_frame*)
        *(.note*)
    }
}

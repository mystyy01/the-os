[BITS 16]
[ORG 0x8000]

cli

mov eax, [0x8FF0]
mov cr3, eax

mov eax, cr4
or eax, 1 << 5
mov cr4, eax

mov ecx, 0xC0000080
rdmsr
or eax, 1 << 8
wrmsr

mov eax, cr0
or eax, (1 << 0) | (1 << 31)
mov cr0, eax
lgdt [ap_gdt_desc]
jmp 0x08:ap_64

[BITS 64]
ap_64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, [0x8FF8]
    mov rax, [0x9000]
    jmp rax

align 16
ap_gdt_desc:
    dw ap_gdt_end - ap_gdt - 1
    dd ap_gdt

align 16
ap_gdt:
    dq 0
    dq 0x00AF9A000000FFFF
    dq 0x00AF92000000FFFF
ap_gdt_end:

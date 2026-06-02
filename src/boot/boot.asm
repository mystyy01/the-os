extern kernel_main
extern exception_handler
extern syscall_handler

section .multiboot2
header_start:
  dd 0xE85250D6
  dd 0x00000000

  dd header_end - header_start

  dd 0x100000000 - (0xE85250D6 + 0 + (header_end - header_start))

  dd 0x00000000
  dd 0x00000000
  
header_end:

section .boot.bss
  boot_stack_bottom:
    resb 4096
  boot_stack_top:

  global PML4
  align 4096
  PML4:
    resb 4096
  align 4096
  PDPT:
    resb 4096
  PD:
    resb 4096
  align 4096
  HHDM_PDPT:
    resb 4096
  align 4096
  KERNEL_PDPT:
    resb 4096
  KERNEL_PD:
    resb 4096

section .boot.data
  boot_gdt_start:
    dq 0
    dw 0xFFFF, 0x0000
    db 0x00, 0x9A, 0xAF, 0x00
    dw 0xFFFF, 0x0000
    db 0x00, 0x92, 0xAF, 0x00
  boot_gdt_end:
  boot_gdt_descriptor: 
    dw (boot_gdt_end - boot_gdt_start - 1)
    dd boot_gdt_start

section .bss
  global stack_top
  stack_bottom:
    resb 16384
  stack_top:
    
  align 16
  global tss
  tss:
    dd 0
    dq 0
    dq 0 
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dq 0
    dw 0
    dw 104
  tss_end:


%macro isr_no_err 1
isr_%1:
  push 0
  push %1
  jmp exception_common
%endmacro
%macro isr_err 1
isr_%1:
  push %1 
  jmp exception_common
%endmacro
section .boot.text
  [BITS 32]
  global _start
  _start:
    mov esp, boot_stack_top

    cmp eax, 0x36D76289
    jne hang

    mov eax, PDPT
    or eax, 0b11
    mov dword [PML4], eax

    mov eax, PD 
    or eax, 0b11
    mov dword [PDPT], eax
  
    mov ecx, 512
    mov eax, 0x83
    mov edi, PD

    .loop:
      mov dword [edi], eax
      mov dword [edi + 4], 0
      add eax, 0x200000
      add edi, 8
      loop .loop

    mov eax, HHDM_PDPT
    or eax, 0b11
    mov dword [PML4 + 256*8], eax
    mov dword [PML4 + 256*8 + 4], 0

    mov ecx, 512
    xor esi, esi
    mov edi, HHDM_PDPT
    .hhdm_loop:
      mov eax, esi
      shl eax, 30
      or eax, 0x83
      mov dword [edi], eax
      mov eax, esi
      shr eax, 2 
      mov dword [edi + 4], eax
      add edi, 8 
      inc esi 
      loop .hhdm_loop

    mov eax, KERNEL_PDPT
    or eax, 0b11
    mov dword [PML4 + 511*8], eax
    mov dword [PML4 + 511*8 + 4], 0

    mov eax, KERNEL_PD
    or eax, 0b11
    mov dword [KERNEL_PDPT + 510*8], eax
    mov dword [KERNEL_PDPT + 510*8 + 4], 0

    mov ecx, 512
    mov eax, 0x83
    mov edi, KERNEL_PD
    .khigh_loop:
      mov dword [edi], eax
      mov dword [edi + 4], 0
      add eax, 0x200000
      add edi, 8
      loop .khigh_loop

    mov eax, PML4
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5 
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 1 << 0 
    or eax, 1 << 31
    mov cr0, eax

    lgdt [boot_gdt_descriptor]
    jmp 0x08:long_mode_start
  hang:
    hlt
    jmp hang
  [BITS 64]
  long_mode_start:
    mov ax, 0
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    mov rax, high_start
    jmp rax
  section .text
  [BITS 64]
  high_start:
    mov rsp, stack_top
    mov rdi, rbx
    call kernel_main
    
    global isr_0
    isr_no_err 0

    global isr_8
    isr_err 8

    global isr_13
    isr_err 13

    global isr_14
    isr_err 14

    global isr_6
    isr_no_err 6

    global isr_32
    isr_no_err 32
    
    global context_switch

    global syscall_entry
    
    global switch_to

    global user_entry_bouncy_trampoline_lol
  switch_to:
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15

    mov [rdi], rsp
    mov rsp, rsi
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    ret 
  user_entry_bouncy_trampoline_lol:
    push 0x1b
    push r14
    push 0x202
    push 0x23
    push r15
    swapgs
    iretq
  syscall_entry:
    swapgs
    mov gs:[8], rsp
    mov rsp, gs:[0]
    push qword gs:[8]

    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14 
    push r15
  
    mov rsi, rdi
    mov rcx, r10
    mov rdi, rax

    call syscall_handler

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8 
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx

    mov rsp, [rsp + 8]
    swapgs
    o64 sysret
  exception_common:
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14 
    push r15

    mov rdi, [rsp + 15*8]
    mov rsi, [rsp + 15*8 + 8]
  
    mov rdx, rsp

    call exception_handler

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8 
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16
    iretq

  context_switch:
    mov rsp, [rdi + 0]

    push qword [rdi + 128]
    push qword [rdi + 136]

    push qword [rdi + 120]  ; rax
    push qword [rdi + 80] ; rdi

    mov r15, [rdi + 8]
    mov r14, [rdi + 16]
    mov r13, [rdi + 24]
    mov r12, [rdi + 32]
    mov r11, [rdi + 40]
    mov r10, [rdi + 48]
    mov r9, [rdi + 56]
    mov r8, [rdi + 64]
    mov rbp, [rdi + 72]
    mov rsi, [rdi + 88]
    mov rdx, [rdi + 96]
    mov rcx, [rdi + 104]
    mov rbx, [rdi + 112]

    pop rdi
    pop rax
    popfq
    ret

section .data 
  global gdt_start
  global gdt_end
  gdt_start:
      dq 0
      dw 0xFFFF, 0x0000
      db 0x00, 0x9A, 0xAF, 0x00
      dw 0xFFFF, 0x0000
      db 0x00, 0x92, 0xAF, 0x00
      dw 0xFFFF, 0x0000
      db 0x00, 0xF2, 0xAF, 0x00
      dw 0xFFFF, 0x0000
      db 0x00, 0xFA, 0xAF, 0x00
      dq 0
      dq 0
  gdt_end:
  


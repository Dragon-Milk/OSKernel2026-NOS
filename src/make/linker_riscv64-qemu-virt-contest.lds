OUTPUT_ARCH(riscv)

KERNEL_BASE_PADDR = 0x80200000;
KERNEL_BASE_VADDR = 0xffffffc080200000;
PHYS_VIRT_OFFSET = KERNEL_BASE_VADDR - KERNEL_BASE_PADDR;

BASE_ADDRESS = KERNEL_BASE_VADDR;

ENTRY(_start_phys)
SECTIONS
{
    . = BASE_ADDRESS;
    _skernel = .;

    .text ALIGN(4K) : AT(ADDR(.text) - PHYS_VIRT_OFFSET) {
        _stext = .;
        KEEP(*(.text.boot))
        *(.text .text.*)
        . = ALIGN(4K);
        _etext = .;
    }
    _start_phys = _start - PHYS_VIRT_OFFSET;

    _srodata = .;
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - PHYS_VIRT_OFFSET) {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
        *(.sdata2 .sdata2.*)
    }

    .init_array ALIGN(0x10) : AT(ADDR(.init_array) - PHYS_VIRT_OFFSET) {
        __init_array_start = .;
        *(.init_array .init_array.*)
        __init_array_end = .;
    }

    debug_abbrev : AT(ADDR(debug_abbrev) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_abbrev); }
    debug_addr : AT(ADDR(debug_addr) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_addr); }
    debug_aranges : AT(ADDR(debug_aranges) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_aranges); }
    debug_info : AT(ADDR(debug_info) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_info); }
    debug_line : AT(ADDR(debug_line) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_line); }
    debug_line_str : AT(ADDR(debug_line_str) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_line_str); }
    debug_ranges : AT(ADDR(debug_ranges) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_ranges); }
    debug_rnglists : AT(ADDR(debug_rnglists) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_rnglists); }
    debug_str : AT(ADDR(debug_str) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_str); }
    debug_str_offsets : AT(ADDR(debug_str_offsets) - PHYS_VIRT_OFFSET) { . += SIZEOF(.debug_str_offsets); }

    . = ALIGN(4K);
    _erodata = .;

    .data ALIGN(4K) : AT(ADDR(.data) - PHYS_VIRT_OFFSET) {
        _sdata = .;
        *(.data.boot_page_table)
        . = ALIGN(4K);
        *(.data .data.*)
        *(.sdata .sdata.*)
        *(.got .got.*)

        . = ALIGN(0x10);
        _sdriver = .;
        KEEP(*(.driver.register*))
        _edriver = .;

        . = ALIGN(0x10);
        _ex_table_start = .;
        KEEP(*(__ex_table))
        _ex_table_end = .;
    }

    .tdata ALIGN(0x10) : AT(ADDR(.tdata) - PHYS_VIRT_OFFSET) {
        _stdata = .;
        *(.tdata .tdata.*)
        _etdata = .;
    }

    .tbss ALIGN(0x10) : AT(ADDR(.tbss) - PHYS_VIRT_OFFSET) {
        _stbss = .;
        *(.tbss .tbss.*)
        *(.tcommon)
        _etbss = .;
    }

    linkme_IRQ ALIGN(0x10) : AT(ADDR(linkme_IRQ) - PHYS_VIRT_OFFSET) { KEEP(*(linkme_IRQ)) }
    linkm2_IRQ ALIGN(0x10) : AT(ADDR(linkm2_IRQ) - PHYS_VIRT_OFFSET) { KEEP(*(linkm2_IRQ)) }
    linkme_PAGE_FAULT ALIGN(0x10) : AT(ADDR(linkme_PAGE_FAULT) - PHYS_VIRT_OFFSET) { KEEP(*(linkme_PAGE_FAULT)) }
    linkm2_PAGE_FAULT ALIGN(0x10) : AT(ADDR(linkm2_PAGE_FAULT) - PHYS_VIRT_OFFSET) { KEEP(*(linkm2_PAGE_FAULT)) }
    scope_local ALIGN(0x10) : AT(ADDR(scope_local) - PHYS_VIRT_OFFSET) { KEEP(*(scope_local)) }

    . = ALIGN(4K);
    _percpu_start = .;
    _percpu_end = _percpu_start + SIZEOF(.percpu);
    .percpu 0x0 : AT(_percpu_start - PHYS_VIRT_OFFSET) {
        _percpu_load_start = .;
        *(.percpu .percpu.*)
        _percpu_load_end = .;
        . = _percpu_load_start + ALIGN(64) * 1;
    }
    . = _percpu_end;

    . = ALIGN(4K);
    _edata = .;

    .bss ALIGN(4K) : AT(ADDR(.bss) - PHYS_VIRT_OFFSET) {
        boot_stack = .;
        *(.bss.stack)
        . = ALIGN(4K);
        boot_stack_top = .;

        _sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
        *(COMMON)
        . = ALIGN(4K);
        _ebss = .;
    }

    _ekernel = .;

    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}

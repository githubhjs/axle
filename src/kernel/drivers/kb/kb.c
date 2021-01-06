#include "kb.h"
#include <std/common.h>
#include <kernel/interrupts/interrupts.h>
#include <kernel/util/amc/amc.h>
#include <kernel/util/adi/adi.h>
#include <kernel/util/vfs/fs.h>
#include <kernel/drivers/ps2/ps2.h>
#include <kernel/multitasking/tasks/task_small.h>

char kgetch() {
	Deprecated();
	return '\0';
}

char getchar() {
	Deprecated();
	return '\0';
}

bool haskey() {
	Deprecated();
	return false;
}

char kb_modifiers() {
	Deprecated();
	return 0;
}

void kb_callback(registers_t* regs) {
	adi_interrupt_dispatch(regs->int_no);
}

void ps2_keyboard_enable(void) {
	printf_info("[PS2] Enabling keyboard...");
	// Setup an interrupt handler to receive IRQ1's
	interrupt_setup_callback(INT_VECTOR_IRQ1, &kb_callback);

	// Get the current scancode set
	ps2_write_device(0, KBD_SSC_CMD);
	ps2_expect_ack();
	ps2_write_device(0, KBD_SSC_GET);
	ps2_expect_ack();
	uint8_t scancode_set = ps2_read(PS2_DATA);
	printf("Scan code set %d\n", scancode_set);
	assert(scancode_set == KBD_SSC_2, "Wrong keyboard scancode set\n");

	// Ask the PS/2 keyboard to start sending events
	ps2_write_device(0, PS2_DEV_ENABLE_SCAN);
	// TODO(PT): Is this ack actually sent as an interrupt?
	ps2_expect_ack();
}

void ps2_keyboard_driver_launch(void) {
	// TODO(PT): Refactored method to launch a driver
    const char* program_name = "kb_driver";
    FILE* fp = initrd_fopen(program_name, "rb");
    char* argv[] = {program_name, NULL};
    elf_load_file(program_name, fp, argv);
	panic("noreturn");
}
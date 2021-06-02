#include <stdbool.h>
#include <stddef.h>
#include <std/array_m.h>

#include <kernel/boot_info.h>

#include "amc_internal.h"
#include "core_commands.h"

static void _amc_core_copy_amc_services(const char* source_service) {
    printf("Request to copy services\n");
   
    array_m* services = amc_services();
    uint32_t response_size = sizeof(amc_service_list_t) + (sizeof(amc_service_description_t) * services->size);
    amc_service_list_t* service_list = calloc(1, response_size);
    service_list->event = AMC_COPY_SERVICES_RESPONSE;
    service_list->service_count = services->size;

    for (int i = 0; i < services->size; i++) {
        amc_service_description_t* service_desc = &service_list->service_descs[i];
        amc_service_t* service = array_m_lookup(services, i);
        //printf("Service desc 0x%08x, amc service 0x%08x %s -> desc 0x%08x 0x%08x\n", service_desc, service->name, service->name, service_desc->service_name, &service_desc->service_name);
        strncpy(&service_desc->service_name, service->name, AMC_MAX_SERVICE_NAME_LEN);
        service_desc->unread_message_count = service->message_queue->size;
    }
    amc_message_construct_and_send__from_core(source_service, service_list, response_size);
    kfree(service_list);
    return true;
}

static void _amc_core_awm_map_framebuffer(const char* source_service) {
    // Only awm is allowed to invoke this code!
    assert(!strncmp(source_service, "com.axle.awm", AMC_MAX_SERVICE_NAME_LEN), "Only AWM may use this syscall");

    amc_service_t* current_service = amc_service_with_name(source_service);
    spinlock_acquire(&current_service->spinlock);
    framebuffer_info_t* framebuffer_info = &boot_info_get()->framebuffer;
    //  Map VESA framebuffer into proc's address space
    vmm_identity_map_region(
        (vmm_page_directory_t*)vmm_active_pdir(), 
        framebuffer_info->address,
        framebuffer_info->size
    );
    spinlock_release(&current_service->spinlock);

    // And set the pages as accessible to user-mode
    uint32_t framebuf_start_addr = framebuffer_info->address;
    uint32_t framebuf_end_addr = framebuffer_info->address + framebuffer_info->size;
    // Pad to page size
    framebuf_end_addr = (framebuf_end_addr + (PAGE_SIZE - 1)) & PAGING_PAGE_MASK;
    printf("Framebuffer: 0x%08x - 0x%08x (%d pages)\n", framebuf_start_addr, framebuf_end_addr, ((framebuf_end_addr - framebuf_start_addr) / PAGE_SIZE));
    for (uint32_t addr = framebuf_start_addr; addr < framebuf_end_addr; addr += PAGE_SIZE) {
        vmm_set_page_usermode(vmm_active_pdir(), addr);
    }

    amc_framebuffer_info_t msg = {.event = AMC_AWM_MAP_FRAMEBUFFER_RESPONSE};
    // Copy the framebuffer_info_t into the structure subfields that exactly match its layout
    memcpy(&msg.type, framebuffer_info, sizeof(framebuffer_info_t));
    amc_message_construct_and_send__from_core(source_service, &msg, sizeof(amc_framebuffer_info_t));
}

static void _amc_core_put_service_to_sleep(const char* source_service, uint32_t ms, bool awake_on_message) {
    amc_service_t* service = amc_service_with_name(source_service);

    uint32_t now = ms_since_boot();
    uint32_t wake = now + ms;
    service->task->blocked_info.wake_timestamp = wake;
    char* extra_msg = (awake_on_message) ? "or message arrives" : "(time only)";
    //printf("Core blocking %s [%d %s] at %d until %d %s (%dms)\n", source_service, service->task->id, service->task->name, now, wake, extra_msg, ms);

    array_m_insert(amc_sleeping_procs(), service);
    uint32_t block_reason = (awake_on_message) ? (AMC_AWAIT_TIMESTAMP | AMC_AWAIT_MESSAGE) : AMC_AWAIT_TIMESTAMP;
    tasking_block_task(service->task, block_reason);
}

static void _amc_core_file_manager_map_initrd(const char* source_service) {
    // Only file_manager is allowed to invoke this code!
    assert(!strncmp(source_service, "com.axle.file_manager", AMC_MAX_SERVICE_NAME_LEN), "Only File Manager may use this syscall");

    amc_service_t* current_service = amc_service_with_name(source_service);
    spinlock_acquire(&current_service->spinlock);

    // Map the ramdisk into the proc's address space
    boot_info_t* bi = boot_info_get();
    vmm_identity_map_region(
        (vmm_page_directory_t*)vmm_active_pdir(),
        bi->initrd_start,
        bi->initrd_size
    );
    spinlock_release(&current_service->spinlock);

    // And mark the pages as accessible to usermode
    printf("Ramdisk: 0x%08x - 0x%08x (%d pages)\n", bi->initrd_start, bi->initrd_end, bi->initrd_size / PAGE_SIZE);
    for (uint32_t addr = bi->initrd_start; addr < bi->initrd_end; addr += PAGE_SIZE) {
        vmm_set_page_usermode(vmm_active_pdir(), addr);
    }

    amc_initrd_info_t msg = {
        .event = AMC_FILE_MANAGER_MAP_INITRD_RESPONSE,
        .initrd_start = bi->initrd_start,
        .initrd_end = bi->initrd_end,
        .initrd_size = bi->initrd_size,
    };
    amc_message_construct_and_send__from_core(source_service, &msg, sizeof(amc_initrd_info_t));
}


static void _trampoline(const char* program_name, void* buf, uint32_t buf_size) {
    char* argv[] = {program_name, NULL};
    elf_load_buffer(program_name, buf, buf_size, argv);
	panic("noreturn");
}

static void _amc_core_file_manager_exec_buffer(const char* source_service, void* buf, uint32_t buf_size) {
    // Only file_manager is allowed to invoke this code!
    assert(!strncmp(source_service, "com.axle.file_manager", AMC_MAX_SERVICE_NAME_LEN), "Only File Manager may use this syscall");

    amc_exec_buffer_cmd_t* cmd = (amc_exec_buffer_cmd_t*)buf;
    printf("program name %s\n", cmd->program_name);

    task_spawn__with_args(
        _trampoline, 
        cmd->program_name, 
        cmd->buffer_addr, 
        cmd->buffer_size, 
        cmd->program_name
    );
}

static void _amc_core_handle_profile_request(const char* source_service) {
    amc_system_profile_response_t resp = {0};
    resp.event = AMC_SYSTEM_PROFILE_RESPONSE;
    resp.pmm_allocated = pmm_allocated_memory();
    resp.kheap_allocated = kheap_allocated_memory();
    amc_message_construct_and_send__from_core(source_service, &resp, sizeof(resp));
}

static void _amc_core_handle_notify_service_died(const char* source_service, void* buf, uint32_t buf_size) {
    amc_service_t* source = amc_service_with_name(source_service);
    assert(source != NULL, "Failed to find service that sent the message...");

    amc_notify_when_service_dies_cmd_t* cmd = (amc_notify_when_service_dies_cmd_t*)buf;
    amc_service_t* remote = amc_service_with_name(&cmd->remote_service);
    if (!remote) {
        printf("Dropping request to notify on %s's death because it doesn't exist\n", cmd->remote_service);
        return;
    }

    array_m_insert(remote->services_to_notify_upon_death, source);
}

void amc_core_handle_message(const char* source_service, void* buf, uint32_t buf_size) {
    //printf("Message to core from %s\n", source_service);
    uint32_t* u32buf = (uint32_t*)buf;
    if (u32buf[0] == AMC_COPY_SERVICES) {
        _amc_core_copy_amc_services(source_service);
    }
    else if (u32buf[0] == AMC_AWM_MAP_FRAMEBUFFER) {
        _amc_core_awm_map_framebuffer(source_service);
    }
    else if (u32buf[0] == AMC_SLEEP_UNTIL_TIMESTAMP) {
        _amc_core_put_service_to_sleep(source_service, u32buf[1], false);
    }
    else if (u32buf[0] == AMC_SLEEP_UNTIL_TIMESTAMP_OR_MESSAGE) {
        _amc_core_put_service_to_sleep(source_service, u32buf[1], true);
    }
    else if (u32buf[0] == AMC_FILE_MANAGER_MAP_INITRD) {
        _amc_core_file_manager_map_initrd(source_service);
    }
    else if (u32buf[0] == AMC_FILE_MANAGER_EXEC_BUFFER) {
        _amc_core_file_manager_exec_buffer(source_service, buf, buf_size);
    }
    else if (u32buf[0] == AMC_SHARED_MEMORY_DESTROY) {
        assert(false, "shmem destroy amccmd");
        //_amc_core_shared_memory_destroy(source_service, buf, buf_size);
    }
    else if (u32buf[0] == AMC_SYSTEM_PROFILE_REQUEST) {
        _amc_core_handle_profile_request(source_service);
    }
    else if (u32buf[0] == AMC_REGISTER_NOTIFICATION_SERVICE_DIED) {
        _amc_core_handle_notify_service_died(source_service, buf, buf_size);
    }
    else {
        printf("Unknown message: %d\n", u32buf[0]);
        assert(0, "Unknown message to core");
        return;
    }
}
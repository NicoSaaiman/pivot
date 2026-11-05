#include <stdio.h>
#include <stddef.h>

#include "ffi_lib.h"
#include "creation.h"
#include "monitoring.h"
#include "deletion.h"

void ffi_print_process_details(const CProcess *process) {
    printf("Command: %s\n", process->command);

    printf("Arguments:\n");
    for (size_t i = 0; i < process->argc; i++) {
        printf("  [%zu]: %s\n", i, process->args[i]);
    }

    printf("Environment:\n");
    for (size_t i = 0; i < process->env_count; i++) {
        printf("  %s=%s\n",
            process->env_keys[i],
            process->env_values[i]);
    }

    printf("Working directory: %s\n",
        process->working_dir != NULL
            ? process->working_dir
            : "(none)");
}

pid_t ffi_create_process(const CProcess *process) {
	return create_process(process);	
}

ProcessState ffi_get_process_state(pid_t pid) {
	return get_process_state(pid);
}

int ffi_terminate_process(pid_t pid) {
	return terminate_process(pid);
}

int ffi_wait_process_blocking(pid_t pid) {
	return wait_process_blocking(pid);
}

#include <stddef.h>

#include "ffi_lib.h"
#include "creation.h"
#include "monitoring.h"
#include "deletion.h"

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

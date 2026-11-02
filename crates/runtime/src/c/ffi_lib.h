#ifndef FFI_LIB_H
#define FFI_LIB_H

#include <stddef.h>
#include <unistd.h>

typedef struct {
    const char *command;
    const char **args;
    size_t argc;
    const char **env_keys;
    const char **env_values;
    size_t env_count;
    const char *working_dir;
} CProcess;

void ffi_print_process_details(const CProcess *process); 
pid_t ffi_create_process(const CProcess *process);

#endif

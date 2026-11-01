#include <stdio.h>
#include <stddef.h>

void ffi_print_process_details(
    const char *command,
    const char **args,
    size_t argc,
    const char **env_keys,
    const char **env_values,
    size_t env_count,
    const char *working_dir
) {
    printf("Command: %s\n", command);

    printf("Arguments:\n");
    for (size_t i = 0; i < argc; i++) {
        printf("  [%zu]: %s\n", i, args[i]);
    }

    printf("Environment:\n");
    for (size_t i = 0; i < env_count; i++) {
        printf("  %s=%s\n", env_keys[i], env_values[i]);
    }

    printf("Working directory: %s\n",
        working_dir != NULL ? working_dir : "(none)");
}

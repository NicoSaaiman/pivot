#ifndef CREATION_H
#define CREATION_H

#include <sys/types.h>

#include "ffi_lib.h"

pid_t create_process(const CProcess *process);
int create_processes(int num_to_create, pid_t *child_pids, char **argv);

#endif

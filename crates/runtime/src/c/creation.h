#ifndef CREATION_H
#define CREATION_H

#include <unistd.h>
#include <sys/types.h>

int create_processes(int num_to_create, pid_t *child_pids, char **argv);

#endif

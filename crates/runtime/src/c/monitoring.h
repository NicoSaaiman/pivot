#ifndef MONITORING_H
#define MONITORING_H

#include <unistd.h>
#include <sys/types.h>

int get_process_state(pid_t pid);
int monitor_processes(int num_to_create, int created_total, pid_t *child_pids); 

#endif

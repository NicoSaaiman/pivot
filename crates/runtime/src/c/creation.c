#include "creation.h"
#include "ffi_lib.h"

#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

pid_t create_process(const CProcess *process) {
	pid_t child_pid = fork();

	if (child_pid == -1) {
		perror("fork");
		return -1;
	}

	if (child_pid == 0) {
		char *arg_vec[process->argc + 2];

		arg_vec[0] = (char *)process->command;

		for (size_t i = 0; i < process->argc; i++) {
			arg_vec[i + 1] = (char *)process->args[i];
		}

		arg_vec[process->argc + 1] = NULL;

		execvp(process->command, arg_vec);
		perror("execvp");
		_exit(EXIT_FAILURE);
	}
	
	return child_pid;
}

// Returns number of successfully created processes
int create_processes(int num_to_create, pid_t *child_pids, char **argv) {
	int created_total = 0;

	for (int i = 0; i < num_to_create; i++) {
		child_pids[i] = fork();

		char *arg_vec[] = {
			"./basic",
			argv[i + 1],
			NULL
		};

		if (child_pids[i] == 0) {
			execvp(arg_vec[0], arg_vec);
			perror("execvp");
			_exit(EXIT_FAILURE);
		}

		if (child_pids[i] == -1) {
			perror("fork");
			child_pids[i] = 0;
			continue;
		}

		created_total++;
	}

	return created_total;
}

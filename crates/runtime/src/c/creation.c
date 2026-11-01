#include "creation.h"

#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>

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

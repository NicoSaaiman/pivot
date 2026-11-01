#include "monitoring.h"

#include <unistd.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include <sys/wait.h>

int monitor_processes(int num_to_create, int created_total, pid_t *child_pids) {
	int failed = 0;
	int remaining = created_total;
	while (remaining > 0) {
		for (int i = 0; i < num_to_create; i++) {
			if (child_pids[i] == 0 || child_pids[i] == -1) { // skip over entries marked as success or failure
				continue;
			}

			int status;
			pid_t result = waitpid(child_pids[i], &status, WNOHANG);

			if (result == 0) { // child hasn't changed state, move on to next
				continue;
			}

			if (result == -1) {
				if (errno != EINTR) { // mark all other as failures for now
					perror("waitpid");
					child_pids[i] = -1; // silently flag as an issue
					remaining--; // mark as checked
					failed++;
				}

				continue;
			}

			if (result == child_pids[i]) { // child's state has changed, investigate
				if(WIFEXITED(status)) {
					int child_exit_status = WEXITSTATUS(status);
					if (child_exit_status != 0) {
						fprintf(stderr, "Child exited with status: %d\n", child_exit_status);
						child_pids[i] = -1;
						failed++;
					} else {
						child_pids[i] = 0;
					}
				} else if (WIFSIGNALED(status)) {
					printf("Child terminated by signal %d\n", WTERMSIG(status));
					child_pids[i] = -1;
					failed++;
				} else {
					fprintf(stderr, "Child terminated with unexpected status\n");
					child_pids[i] = -1; // silently flag as an issue
					failed++;
				}

				remaining--;
			}
		}

		if (remaining > 0) {
			sleep(1);
		}
	}

	return failed;
}

#include "creation.h"
#include "monitoring.h"

#include <stdio.h>
#include <stdlib.h>

#define MAX_CHILDREN 3

int main(int argc, char *argv[]) {
	if (argc != MAX_CHILDREN + 1) {
		fprintf(stderr, "Unexpected number of arguments. %d int expected\n", MAX_CHILDREN + 1);
		return EXIT_FAILURE;
	}

	pid_t child_pids[MAX_CHILDREN];

	int created_total = create_processes(MAX_CHILDREN, child_pids, argv);

	int failed = monitor_processes(MAX_CHILDREN, created_total, child_pids);

	if (failed > 0) {
		fprintf(stderr, "%d processes failed for some reason...\n", failed);
		return EXIT_FAILURE;
	}
	
	return EXIT_SUCCESS;
}

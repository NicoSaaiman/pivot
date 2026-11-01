#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <sys/types.h>

int main(int argc, char *argv[]) {
	if (argc != 2) {
		fprintf(stderr, "Unexpected number of arguments provided\n");
		return EXIT_FAILURE;
	}

	pid_t pid = getpid();
	int duration = atoi(argv[1]);

	printf("Running a new program! PID: %d\n", pid);
	sleep(duration);
	printf("Done! PID: %d\n", pid);
}

#include <sys/types.h>
#include <signal.h>

#include "deletion.h"

int terminate_process(pid_t pid) {
	return kill(pid, SIGTERM);
}

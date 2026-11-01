#include <stdio.h>

#include "runtime.h"

int main(void) {
	int result = runtime_example();
	printf("Runtime returned: %d\n", result);
	return 0;
}

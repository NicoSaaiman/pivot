#ifndef PROCESS_STATE_H
#define PROCESS_STATE_H

typedef enum {
    PROCESS_ERROR = -1,
    PROCESS_RUNNING = 0,
    PROCESS_EXITED = 1,
    PROCESS_SIGNALED = 2
} ProcessState;

#endif

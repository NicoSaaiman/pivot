# Flow

## Purpose
Examine the flow of operations and spend some time thinking through some of the behavior that's expected vs what's currently happening. 

I'll start by outlining public interfaces to determine the entrypoints into the system and also highlight the internal boundary interfaces to keep track of the user request throughout the journey to eventual result.

## System Interfaces
### Controller API
| Method | Endpoint       | Function         | Payload  | Return                |
|--------|----------------|------------------|----------|-----------------------|
| POST   | /execute       | execute_workload | Workload | -                     |
| GET    | /workload/{id} | get_workload     | -        | WorkloadDetails       |
| DELETE | /workload/{id} | delete_workload  | -        | -                     |
| POST   | /nodes         | update_nodes     | Node     | -                     |
| GET    | /nodes         | get_nodes        | -        | HashMap<NodeId, Node> |

---

## Internal Interfaces
### Agent API
| Method | Endpoint       | Function         | Payload  | Return                              |
|--------|----------------|------------------|----------|-------------------------------------|
| POST   | /execute       | execute_workload | Workload | -                                   |
| GET    | /workloads     | get_workloads    | -        | HashMap<WorkloadId, ExecutionState> |
| GET    | /workload/{id} | get_workload     | -        | WorkloadStatus                      |
| DELETE | /workload/{id} | delete_workload  | -        | -                                   |
| GET    | /info          | get_node_info    | -        | NodeInfo                            |
| GET    | /metrics       | get_node_metrics | -        | NodeMetrics                         |

### Process Runtime
| Function         | Payload  | Return |
|------------------|----------|--------|
| execute_workload | Workload | -      |

### Docker Container Runtime
| Function                | Payload                                   | Return         |
|-------------------------|-------------------------------------------|----------------|
| docker_create_container | &str, Option<String>, Option<Vec<String>> | String         |
| docker_start_container  | &str                                      | -              |
| docker_get_state        | &str                                      | ExecutionState |
| docker_delete_container | &str                                      | -              |

### Custom Container Runtime FFI
| Function (Rust)       | Payload (Rust) | Return (Rust)  | Function (C)              | Payload (C)     | Return (C) |
|-----------------------|----------------|----------------|---------------------------|-----------------|------------|
| create_process        | &Process       | c_int          | ffi_create_process        | *const CProcess | c_int      |
| get_process_state     | c_int          | ExecutionState | ffi_get_process_state     | c_int           | c_int      |
| terminate_process     | c_int          | -              | ffi_terminate_process     | c_int           | c_int      |
| wait_process_blocking | c_int          | -              | ffi_wait_process_blocking | c_int           | c_int      |

---

## Routes
### Normal
1. User submits workload to external interface /execute that specifies type of workload and desired state (among other things).
2. 

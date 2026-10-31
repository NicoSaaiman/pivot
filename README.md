# Pivot

## Introduction
A Rust-based workload orchestrator that schedules workloads across different nodes and continuously reconciles actual state against user desired state.

## Why Pivot?
My pivot towards systems-level software engineering and Linux's `pivot_root`, a mechanism used in setting up isolated filesystem environments, including containers.

## Motivation and Goals
The main motivation behind Pivot is creating a system that's capable of handling a wide range of workload types using the same interface and ultimately ensuring that those workload states stay in sync with the user specified desired state.

## Current Capabilities
TODO

## Architecture
Add link to v1 design

## Technology Stack
Rust,
...

## Getting Started

Controller and Agent(s) should run in different terminal windows.
Ensure that Docker Desktop is running.

1) Start Controller:

```bash
cargo run -p pivot-controller
```

Note: Controller uses port 3000 by default

2) Start Agent(s):

```bash
cargo run -p pivot-agent <node_name> <port>
```

3) Send desired workload:

3.1) Process:
```cmd
curl -X POST localhost:3000/execute -H "Content-Type: application/json" -d "{\"id\":\"test\",\"group_id\":\"default\",\"desired_state\":\"Exited\",\"execution_type\":{\"Process\":{\"command\":\"sleep\",\"args\":[\"30\"]}}}"
```

3.2) Docker:
```cmd
curl -X POST http://localhost:3000/execute -H "Content-Type: application/json" -d "{\"id\":\"workload-1\",\"group_id\":\"group-1\",\"desired_state\":\"Running\",\"execution_type\":{\"DockerContainer\":{\"name\":\"pivot-nginx\",\"image\":\"nginx\",\"cmd\":null}}}"
```

## Project Structure
pivot/
    design/
        architecture/
        decisions/
        diagrams/
    src/
        agent/
        controller/
        shared/

## Design Documentation
Add links to documentation

## Roadmap
TODO

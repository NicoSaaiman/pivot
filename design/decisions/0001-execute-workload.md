# ADR-0001: Workload Execution Failures

Status: Accepted

## Context
Pivot keeps track of workloads and their desired states on ControllerState.

The insert happens prior to the interaction with the Agent and Runtime. What happens when there is an issue downstream? Should the workload entry remain on ControllerState? Does it depend on the type of error? Should I retain workload but have some sort of mechanism to track reconiliation eligibility?

## Options Considered
### Option 1: Remove workload if error occurs on initial flow
- Easiest option to implement
- User will immediately be notified of error type and failure to submit workload (also won't be saved to state)
- User will have to resubmit workload until it succeeds

### Option 2: Retain workload entry and have reconciler manage workload reconiliation eligibility
- Move the responsibility of dealing with problems to the component thats purpose for existing is problem solving
- Using a reconiliation eligibility state mechanism can reduce impact of persistent errors on overhead
- More complicated state management logic to keep track of problematic workloads

## Decision
Reconciler will deal with downstream issues and attempt to recover to the user specified desired state. For errors that are not currently recoverable, the workload will be retained in state but managed in some way to indicate a reduced reconciliation eligibility.

## Rationale
Option 2 leaves the complicated problem solving logic (depending on a myriad of different potential problems) to a single component that tracks the change in state of each individual workload and attempts to resolve the issue (depending on severity) without needing a user to constantly resubmit workloads in the hopes that it no longer errors out. By keeping track of workload reconciliation eligibility, problematic workloads will also be retried less often. 

## Consequences
Reconiliation eligibility needs to be explicitly tracked
Retry behavior needs to deal with repeated failures
Error classification needs to distinguish between transient and permanent failures
A mechanism is required to determine when a workload becomes eligible for reconiliation again

## References
controller::handlers::execute_workload
ScheduleManager::schedule_workload

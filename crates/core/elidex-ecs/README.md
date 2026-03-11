# elidex-ecs

ECS-based DOM storage for the elidex browser engine.

Uses [hecs](https://crates.io/crates/hecs) as the underlying Entity Component
System. Provides tree-manipulation API (`append_child`, `insert_before`,
`replace_child`, etc.) with cycle detection and sibling-link consistency
guarantees.

See the [workspace README](../../README.md) for project-wide details.

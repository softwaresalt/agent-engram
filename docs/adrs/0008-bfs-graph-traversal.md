# ADR-0008: Application-Level BFS for Graph Traversal

## Status

Accepted

## Date

2025-02-16

## Context

The `map_code` tool needs to return a symbol's definition plus its graph neighborhood — all nodes reachable within N hops via dependency edges (calls, imports, defines, inherits_from, concerns). SurrealDB supports recursive graph queries via `->` and `<-` operators, but these operators do not natively support breadth-first traversal with configurable depth limits, node caps, or bidirectional edge collection in a single query.

## Decision

Implement BFS traversal at the application level in `src/db/queries.rs` rather than relying on SurrealDB's recursive graph operators. The BFS loop:

1. Starts from a resolved root node (e.g., `function:uuid`).
2. At each depth level, queries outbound and inbound edges for all frontier nodes.
3. Collects discovered neighbors into the visited set.
4. Stops when `max_depth` is reached or `max_nodes` is exceeded.

Depth and node limits are clamped to `CodeGraphConfig` maximums (depth ≤ 5, nodes ≤ 50) to bound query cost.

## Consequences

- **Pro**: Full control over traversal order, truncation behavior, and edge metadata collection.
- **Pro**: Each BFS level issues exactly 2 queries (outbound + inbound edges), keeping round-trip count predictable.
- **Pro**: `truncated` flag in response tells the caller when the neighborhood was capped.
- **Con**: Multiple sequential DB round-trips (2 per depth level) instead of a single recursive query.
- **Mitigation**: With max depth 5, worst case is 10 queries — acceptable for an embedded database.

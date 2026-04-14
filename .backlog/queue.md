# Queue

- Need to consider how the agent can use engram tooling to first ensure that a file has been loaded into the engram database.
- Need to add bug logging into the agent harness for issue tracking and CE learning/loop refactor of agent harness
- The shim should be able to handle multiple concurrent agent sessions such that only one instance of the server is running but able to meet the requests of multiple agents.
- Consider how to output research, bugs, and other content first to file rather than loading it directly into the context window.  Then use the graph, vector, and document store to more efficiently consume the information needed, especially in subagent processes with cheaper models, to make decisions and produce outcomes.
- Multi-language Tree Sitter Graph Capability: need to enable graph capabilities for multiple languages to make the engram server useful for many more project types.  Also need to consider how to support multi-language project workspaces, such as front-end and back-end combined workspaces or mono-repos.  Languages to support:
	- Python
	- Typescript
	- CSharp
	- Go
	- Powershell
	- Java
	- Swift
	- Kotlin
	- SQL
	- C
	- C++

- For the agent harness, need to determine what the correct workflow should be without exception. Should the agent always decompose research into a plan, a plan into features, and features into tasks and subtasks? And should each feature be developed on a unique branch through to completion and PR/merge before starting on the next feature?  If so, this should be a distinct policy applied to the agent harness composition.

# Native lifecycle initialization

Selected lifecycle argument capture reads the native guard declarations when
normalization runs, after the importing module graph has evaluated. It preserves
the same fixed field lists, profile checks and ordinary refusal semantics.
No public operation, authority role, owner, queue or registry is added.

An eager top-level read of those declarations fails for some native entry points:
the import cycle reaches the normalizer before the lifecycle catalog initializes.
The clean-process regression imports each of the 115 modules in the selected engine/reference
static graph, including discovery, its descriptors and query/result validation, in isolation, then imports
the graph in forward and reverse order.
This supplements the existing frontend-exclusion check; an acyclic graph is not
required, but a supported entry point must initialize before accepting operations.

A copied-graph negative fixture restores the eager read and must fail with the
specific catalog-before-initialization error through the lifecycle-products entry
point. That intentional construction failure is an initialization control, not a
detected behavioral mutant. Existing argument, native publication and fault
regressions retain their separate obligations.

This evidence covers the selected static local JavaScript graph and two ordered
traversals. It does not prove arbitrary dynamic import orders, production Rust
initialization, public F17 completeness or independent assurance. The failed
broader qualification remains failed; only a new full frozen pass can qualify
the repaired candidate for adoption.

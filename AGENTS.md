# Working on rForsp

## Approach

- Start with the source, tests, and history relevant to the current task.
- Let design and implementation inform each other. Resolve discrepancies by
  examining intended behaviour and concrete cases; neither existing code nor
  documentation wins automatically.
- Make decisions when the current phase needs them. Avoid committing later
  phases to structures that have not been exercised.
- Prefer the smallest design that handles the known requirements and leaves
  credible extensions possible.
- Keep changes focused on the requested outcome and preserve unrelated work.
- State important trade-offs and failure modes directly.

Assume familiarity with Rust, compiler construction, and concatenative
semantics. Explain project-specific reasoning rather than general background.

## Documentation

Read `docs/agent/index.org` for the document map and Org formatting
conventions. Consult only the documents relevant to the current task.

Treat exploratory designs and plans as context, not settled requirements.
Record durable language behaviour, phase interfaces, and non-obvious rationale
after they survive implementation. Keep active phase planning in a disposable
phase document.

## Implementation conventions

- Use kernel-style Git commits: an imperative subject and, when useful, a body
  explaining why rather than restating the diff.
- Prefer `#[expect(..., reason = "...")]` to `#[allow(...)]`.
- Run `./scripts/verify` for the complete project check.
- Use differential testing against the C reference interpreter when it can
  settle a semantic question.
- Write comments as short declarative sentences in active voice and present
  tense. Keep a comment only when it adds information not apparent from the
  code.
- Commit only files belonging to the requested task.

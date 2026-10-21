# Conformance suite

This directory is part of the [Drawing Format Standard](../drawing-format-standard.md), clauses 16
and 17. It holds data only. No program is needed to read it.

- `suite.json` lists every case.
- `documents/<id>/input.json` is a drawing document. `documents/<id>/expected.json` is the result a
  runner must give (clause 17.4).
- `units/<function>.json` holds unit checks for one function of clause 17.5.
- `result.schema.json` is the JSON Schema of a runner result and of an expected file.

To run it, write a runner as clause 17.3 says. Give it each `input.json`. Compare its output with
`expected.json` by the rules of clause 17.4. Apply each unit function of clause 17.5 to the inputs
of its unit file and compare with `expected`.

Rules for the files (clause 17.6):

- Expected values are written by hand, or worked out and checked by hand. They are never copied
  from the output of an implementation. If an implementation disagrees with a file, check the file
  first, then the implementation.
- A change to an expected value needs a written reason in the pull request.
- Every number is made up. There is no private data here.

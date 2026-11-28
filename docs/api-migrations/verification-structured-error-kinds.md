### Verification tool refusals carry structured `error.kind` envelopes (minor release)

Refusals from `set_predefined_sizes`, `get_predefined_sizes`, `launch_kicad_ui`,
`open_board`, `copy_routing_pattern` and `check_clearance` now return structured
JSON error bodies (`{"message": ..., "error": {"kind": ...}}`) instead of
plain-text `handler_error` messages (#741). Every refusal keeps its
human-readable text verbatim in `message`.

- `file_not_found`: `set_predefined_sizes` and `get_predefined_sizes` when the
  sibling `.kicad_pro` does not exist. `path` and `absolute_path` name the
  missing project file.
- `invalid_argument`:
  - `set_predefined_sizes` when neither `track_widths` nor `via_dimensions` is
    given (`field: "track_widths"`), when either list is not an array, when an
    entry has the wrong type or is non-positive/non-finite (`track_widths[i]`,
    `via_dimensions[i]`, `via_dimensions[i].diameter`,
    `via_dimensions[i].drill`), or when a via's diameter is not greater than
    its drill (`via_dimensions[i]`).
  - `check_clearance` when `mode` is neither `"anchor"` nor `"courtyard"`
    (`field: "mode"`).
- `editor_unavailable`:
  - `launch_kicad_ui` (`editor: "kicad"`) when spawning KiCad fails or the
    launched process exits during startup or IPC polling.
  - `open_board` (`editor: "pcb"`) when spawning KiCad's PCB editor fails or
    the launched editor exits before reporting the board.
- `malformed_file`: `copy_routing_pattern` when the board has an unbalanced or
  unparseable S-expression, or a top-level segment, arc or via lacks a UUID or
  readable defining coordinates. `path` names the board file and `reason`
  describes the structural defect.

Callers matching tool error text should read `message` or branch on
`error.kind`.

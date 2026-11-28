### Net label rename refusals carry structured `error.kind` envelopes (minor release)

`rename_schematic_net_label` refusals that were plain-text `handler_error` messages
now return structured JSON error bodies (`{"message": ..., "error": {"kind": ...}}`)
(#741). The human-readable text is unchanged in `message`.

- `stale_target`: the sheet holds no label the call names. `reason` repeats
  `message`. `target` names what the call asked for:
  - `label <uuid>` for a `uuid` selection;
  - `label '<net>' at (x, y)` for a `net` with `x` and `y`, whether no label has
    the name or none of them stands there;
  - `label '<net>'` for a `scope` selection.
- `malformed_file`:
  - a sheet of the hierarchy has a label or sheet pin with no readable name or
    position. `path` is that sheet and `reason` the defect;
  - the project's `.kicad_pro` could not be read or parsed for its bus aliases.
    `path` is the project file.
- `conflict`: the sheet's saved hierarchy does not place the given schematic,
  although its project's ownership check did. `paths` holds the schematic and
  the root. The ownership check already refuses an unplaced sheet as `conflict`.
- `file_not_found`: the schematic could not be resolved to a canonical path.

Callers matching tool error text should read `message` or branch on `error.kind`.

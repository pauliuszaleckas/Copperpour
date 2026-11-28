### Library footprint pad metadata is carried through live placement and refresh (patch release)

`update_footprints_from_library`, `place_component`, `place_component_array`,
`duplicate_component` and `update_pcb_from_schematic` now carry the
library's pad-local drill offsets, pad copper clearance, chamfered roundrects,
trapezoid deltas and unconnected annular-ring modes through KiCad's typed
footprint API. Explicit zero pad clearance follows the source KiCad file
version. Rerun a previous refresh dry run to obtain a new plan revision.

`repair_corrupted_footprints` carries this metadata on front-side repairs.
A legacy back-side repair that would rebuild this metadata returns `conflict`.

A refresh still reports `conflict` for unsupported library padstack clauses,
including per-layer padstacks, nested drill clauses other than `offset`, and
annular-ring removal on pads that KiCad cannot save with that mode.
If fresh post-commit readback cannot confirm the planned pads of a footprint
with this metadata, refresh reports `mutation_outcome_uncertain`.

# Stock pad metadata for issue #85 C2

The `issue_85_pad_*.kicad_mod` files come from stock KiCad footprints,
resaved with `kicad-cli fp upgrade --force <scratch-library.pretty>` on KiCad
10.0.8. The corresponding `issue_85_pad_<reference>.ipc.bin` files are the
unmodified `FootprintInstance` payloads from a live KiCad 10.0.8
`GetItems(KOT_PCB_FOOTPRINT)` response.

The original four-layer board was generated using KiCad's `pcbnew` Python bindings.
Each footprint was loaded with `FootprintLoad`, given the library nickname and
entry name with `SetFPID`, and added to the board. Two-digit references were
flipped with `FLIP_DIRECTION_TOP_BOTTOM` and rotated to 37°. The other
references were placed on the front at 0°. KiCad saved the board with
`SaveBoard`. The PCB Editor opened that save in an isolated profile under
headless Weston. The captures contain stock descriptions, paths with KiCad
environment variable names, and generated KIIDs.

| File suffix | Stock library entry | Captured references | Native pad observation |
|---|---|---|---|
| `offset` | `Package_TO_SOT_THT:TO-92_HandSolder` | Q1, Q10 | Pad 1 has a 400,000 nm Y offset between copper and the drill centre on the front and −400,000 nm on the back. |
| `clearance` | `Fiducial:Fiducial_0.5mm_Mask1mm` | R1 | The unnumbered pad has a 350,000 nm copper clearance override. |
| `chamfer` | `Package_LGA:ST_CCLGA-7L_2.8x2.8mm_P1.15mm_H1.95mm` | L1, L10 | Pad 1 has a 0.3 bottom-right chamfer on the front and a 0.3 top-right chamfer on the back. |
| `trapezoid` | `Diode_SMD:D_SMB-SMC_Universal_Handsoldering` | D1, D10 | Pads 1 and 2 have a 1,000,000 nm X delta and a 90° thermal spoke angle on both sides. |
| `trapezoid_y` | `Package_LGA:AMS_LGA-10-1EP_2.7x4mm_P0.6mm` | U10 | The trapezoid for pad 11 has a −300,000 nm Y delta on the back. |
| `annular` | `Connector_Coaxial:SMA_Amphenol_901-143_Horizontal` | J1 | All five pads remove annular rings on unconnected layers except at their start and end layers. |

A second board covers KiCad's default spellings. U51 is stock
`Package_DIP:Fairchild_LSOP-8`. KiCad writes the offset on pad 1 as
`(drill (offset 0 0.266))` because the pad has no hole. For D51, the Python
bindings set both stock diode-pad deltas to zero before
`PCB_IO_KICAD_SEXPR().FootprintSave`. KiCad leaves out `rect_delta` in the
file and includes a zero vector in IPC. For U52, the same writer saved the
stock chamfered footprint with pad 1's chamfer ratio set to zero and pad 2
set to a ratio of 0.3 with every chamfer corner cleared. KiCad preserved both
as chamfered pad shapes when reading the saved board.

The test compares the complete normalized pads of each rebuild with the capture,
including the actual drill shape, dimensions and offsets, pad geometry, zone
properties and annular-ring mode. The stock pad metadata and its side-dependent
transforms are also observable using `pcbnew` pad getters on the saved board.

## Live refresh

The installed 0.17.0 build returned ten `unsupported_library_footprint`
diagnostics on the stock board. The C2 build planned zero pad changes on the
same board and a separate `noop` for U10. For an apply, a scratch copy changed
each metadata group using KiCad's pad setters and added the `C2_TEST` net to one
pad on Q1 and Q10. All ten footprints applied in one commit and the next plan
returned `noop`. After `save_project`, KiCad's Python bindings read all 36 pad
records back with the expected stock geometry and both net assignments intact.
KiCad's DRC reported zero library-footprint mismatches. After reopening the saved
board, nine footprints returned `noop`. D10 reported only a graphics difference
after KiCad shifted two line endpoints by 1 nm on the X axis during save/reopen
([#1001](https://github.com/Copperpour/Copperpour/issues/1001)).

On the second board, the C2 build applied all three default-spelling
cases, dry-ran to `noop`, and saved. KiCad's Python board reader confirmed that
all 17 pad records in that save matched the native oracle.

The live oracle covered KiCad 10.0.8 on Linux. Earlier KiCad 10 patch releases
and the macOS and Windows editor builds were outside the live check.

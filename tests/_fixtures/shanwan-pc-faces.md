# ShanWan PC replay fixture

146 events from phase `pad1_dpad_faces_select` in
`events-02.jsonl`, capture `receiver-pc-bd4ney0a` of the hardware project.
The JSONL keeps only `elapsed_ms`, `type`, `code` and `value`, preserving
their values and event order; unused capture metadata is omitted.
The full capture is not part of this package; replay needs only the adjacent
[fixture](shanwan-pc-faces.jsonl).
Source SHA-256: `a84e244ce35b32e99b572edf40ccc37b472a5c794bdcb591d792dfa805ed6fbb`.
Fixture SHA-256: `c600d3fcda429776b7ff69d778f5dfa887e369ba20d861f872452418efee3091`.

These are hardware events recorded on 17 September 2026, not synthetic inputs.
The offline test supplies a synthetic neutral snapshot before replay and samples
at each recorded SYN_REPORT; it does not reproduce real frame timing or prove
that the new GUI works on hardware. Both observed sequences contain Up, Down,
Left, Right, WEST, NORTH, EAST, SOUTH and Select. The second sequence also
contains fresh Start presses after each action; they are replayed as logical
Start, not discarded. The test uses the public receiver/navigation behavior.

The test descriptor in
[shanwan-2563-0555.bin](https://github.com/idleboxed/imanlib/blob/044a989218813a1636847289d3530e1b264513eb/tests/fixtures/shanwan-2563-0555.bin)
is the unchanged 202-byte `hid-report-descriptor.bin` from the same capture;
SHA-256 `cabd2b0847144b2596be53a7fdc5d281512357ca1bb3b44ee29816b126b5cf26`.

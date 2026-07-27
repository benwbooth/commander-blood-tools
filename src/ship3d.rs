use crate::vm;

/// The depth scroll's ceiling, appearing three times in `0xB75C`:
/// `cmp ax,0x41` @`0xB768` (already there — stop opening), `cmp ax,0x41`
/// @`0xB771` (would overshoot) and `mov ax,0x41` @`0xB776` (clamp to it).
///
/// The second compare is `jl`, a SIGNED test, which matters because the add
/// before it is `add al,[0x2531]` — eight-bit, into the LOW BYTE only
/// (`add_to_low_byte`), so a step that wraps `al` past `0x7F` produces a value
/// the signed compare treats as negative and the clamp does NOT catch.
pub const SHIP_3D_MAX_DEPTH_OFFSET: u16 = 0x41;
/// `mov byte [0x2531],4` @`0xB6A0` — the OPEN step, written once the hold timer
/// passes the threshold below.
pub const SHIP_3D_TRANSITION_OPEN_STEP: u8 = 4;
/// `mov byte [0x2531],8` @`0xB6B8` — the CLOSE step, written when the timer
/// reaches zero while armed. Closing steps twice as fast as opening.
pub const SHIP_3D_TRANSITION_CLOSE_STEP: u8 = 8;
/// `cmp word [0xb3b],0x78` @`0xB699` — 120 ticks. `jbe` falls through, so the
/// transition arms only once the timer is ABOVE it, not at it.
pub const SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD: u16 = 120;
/// `dl = 0x50` @`0xB721`, the row stride the band copy multiplies by.
pub const SHIP_3D_PLANE_ROW_BYTES: usize = 80;
/// `mov ax,0x1f40` @`0xB728` — 8000 bytes, one unchained-VGA plane page
/// (80 bytes/row x 100 rows), used again as the destination stride @`0xB742`.
pub const SHIP_3D_PLANE_PAGE_BYTES: usize = 8000;
/// `add ax,0x23` @`0xB71E` — 35 rows added to the depth offset before the
/// multiply, so a zero depth still copies 35 rows.
pub const SHIP_3D_PLANE_BASE_ROWS: usize = 35;
/// `mov si,0xc000` @`0xB718` — the source page the copy reads from.
pub const SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET: usize = 0xc000;
/// `mov si,0xdf40` @`0xB746` — the SECOND source page. The band copy is two
/// `rep movsb` passes (@`0xB73D` and @`0xB750`), one per page, with the
/// destination advanced a whole page between them by `add di,0x1f40` @`0xB742`.
pub const SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET: usize = 0xdf40;
/// The destination spans BOTH pages, because the routine writes at `di` and then
/// again at `di + 0x1F40` (@`0xB742`) — derived from the copy's shape rather than
/// being an immediate of its own.
pub const SHIP_3D_PLANE_DEST_BYTES: usize = SHIP_3D_PLANE_PAGE_BYTES * 2;
/// `cmp word [0x524d],0xa / je 0xB70B` @`0xB6F0` — mode 10 HOLDS the scroll: the
/// `je` jumps past `mov [0x524f],ax` @`0xB708`, so the band still copies but the
/// scroll value is left where it was. Hence the port's `!=` gate on the update
/// rather than on the copy.
pub const SHIP_3D_SCROLL_MODE_HOLD: u16 = 10;
/// `0xFFFF` terminates the widget's word list, tested as `cmp ax,-1 / je` @`0x8456`
/// in the layout pass and again as `cmp si,-1 / je` @`0x856E` in the draw pass — a
/// SIGNED -1 compare in both (audit-fixes #492). A zero entry ends the list too
/// (`or ax,ax / je` @`0x8452`), so the two terminators are not interchangeable.
pub const SHIP_3D_TARGET_EXIT_SENTINEL: u16 = 0xffff;
/// `add ax,4` @`0x7292` in the DEB candidate walker (`0x7259`): the record's
/// offset plus four is what gets stored into the output list
/// (`mov word ptr [bp],ax` @`0x7295`), which is why the port adds the same 4
/// to reach a candidate's handler record (audit-fixes #554).
///
/// The walker's shape, for the two constants that are NOT settled here: it
/// filters on `test byte ptr es:[di+2],2` @`0x7284` and on
/// `cmp di,gs:[0x6752]` @`0x728B` (the current target), and terminates its
/// output with `mov word ptr [bp],0xffff` @`0x729D`. Those map plausibly onto
/// `SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG` and the selector return,
/// and PLAUSIBLY IS NOT ENOUGH — the port uses them in a different routine's
/// logic, so they stay UNVERIFIED until that routine is the one being read
/// (#509's rule).
pub const SHIP_3D_TARGET_RECORD_HEADER_BYTES: u16 = 4;
pub const SHIP_3D_TARGET_OPEN_STEP: u8 = 6;
/// Four, and there is NO loop counter to read it from: `ship_3d_interpolation_gate`
/// @`0x1E5D` is UNROLLED, one block per word — `[di]` @`0x1E72`, `[di+2]`
/// @`0x1E80`, `[di+4]` @`0x1E8F`, `[di+6]` @`0x1E9E`, then `pop bx` @`0x1EAC`.
/// The count IS the number of blocks, which is also why the layout rect it
/// interpolates is four words wide (`[si]`..`[si+6]`, stored @`0x84A4`-`0x84C3`
/// and copied by the `movsd / movsd` pair @`0x8892`).
pub const SHIP_3D_INTERPOLATION_WORDS: usize = 4;
/// The list widget's layout seeds, all from `list_widget_layout_unified`
/// @`0x8428` (audit-fixes #489). The two widths are ALTERNATIVES, not a base and
/// an addend — `test byte [0xadd],1 / je 0x8448` picks one pair or the other:
///
/// ```text
///   0x8436  xor bp, bp        height seed 0 ...
///   0x8438  mov dx, 0x64      ... and width seed 100   (flag CLEAR)
///   0x843B  test byte [0xadd], 1 / je 0x8448
///   0x8442  mov bp, 0xa       height seed 10 ...
///   0x8445  mov dx, 0x37      ... and width seed 55    (flag SET)
/// ```
///
/// `dx` is then a running MAXIMUM (`cmp ax,dx / jb / mov dx,ax` @`0x8472`), so
/// these are floors the widest label can only raise — which is why the smaller
/// seed goes with the flag that adds an extra row.
pub const SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH: u16 = 100;
/// The width seed when `[0xadd]` bit 0 is SET — `mov dx,0x37` @`0x8445`. The same
/// 55 is stored again @`0x8486` as the appended row's own width.
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH: u16 = 55;
/// `add dx,0x14` @`0x84A1` — widened by 20 before the box is centred.
pub const SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING: u16 = 20;
/// `add bp,0xb` @`0x847A` — 11 rows of height per list entry.
pub const SHIP_3D_TARGET_LAYOUT_ROW_STEP: u16 = 11;
/// The height seed when `[0xadd]` bit 0 is SET — `mov bp,0xa` @`0x8442`, against
/// `xor bp,bp` @`0x8436` when it is clear.
pub const SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT: u16 = 10;
/// `add bp,8` @`0x84A7` — heightened by 8 before the box is centred.
pub const SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING: u16 = 8;
/// `sub bp,0xc8` @`0x84B9` — the 200-row screen the box is centred in.
pub const SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT: u16 = 200;
pub const SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN: u16 = 0xffff;
/// `add cx,4` @`0x84E6` — the row origin's inset below the box top, shared by the
/// hit-test and the draw.
pub const SHIP_3D_TARGET_HIT_TEST_TOP_INSET: u16 = 4;
/// `sub bp,8` @`0x84FF` — the hit area is `height - 8`, so the 8px of chrome the
/// layout adds (`add bp,8` @`0x84A7`) is not clickable.
pub const SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET: u16 = 8;
/// `cmp word [0xa34],6` @`0x850F` — the HOVER presentation mode the hit-test
/// requests when the cursor is over a row.
pub const SHIP_3D_TARGET_HOVER_PRESENTATION_MODE: u16 = 6;
/// `mov word [0xa32],7` @`0x8529`, reached only when `[0xa3e]` bit 0 is set
/// @`0x8522` — the clicked state, one above hover (audit-fixes #492).
pub const SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE: u16 = 7;
/// `mov word [0xa32],1` @`0x8548`, the cursor-outside-the-box branch, guarded by
/// `cmp [0xa34],1 / je` @`0x853B` so it is written once per entry (audit-fixes #492).
pub const SHIP_3D_TARGET_IDLE_PRESENTATION_MODE: u16 = 1;
/// `add cx,0xa` @`0x855C` — the text origin sits 10 right of the box's left edge
/// `[0x2aab]` (audit-fixes #492).
pub const SHIP_3D_TARGET_DRAW_X_INSET: u16 = 10;
/// `mov al,0xe8` @`0x8565`, the colour every row starts with before the hover and
/// active tests below can raise it (audit-fixes #492).
pub const SHIP_3D_TARGET_DEFAULT_TEXT_COLOR: u8 = 0xe8;
/// `mov al,0xef` @`0x858B`, selected when `dec byte gs:[0x27c7]` @`0x8584` reaches
/// zero — i.e. THIS row is the one under the cursor (audit-fixes #492).
pub const SHIP_3D_TARGET_HOVER_TEXT_COLOR: u8 = 0xef;
/// `mov al,0xfe` @`0x8595` — the hovered row when `[0xa3e]` bit 0 is also set
/// @`0x858D`. The three colours are a LADDER, not three independent states:
/// 0xE8 default, 0xEF hovered, 0xFE hovered-and-active (audit-fixes #492).
pub const SHIP_3D_TARGET_ACTIVE_TEXT_COLOR: u8 = 0xfe;
/// `mov si,0x174` @`0x85B3`, drawn only when `[0xadd]` bit 0 is set @`0x85AC` —
/// the same extra-entry flag that picks the narrower layout seeds in #489
/// (audit-fixes #492).
pub const SHIP_3D_TARGET_EXTRA_LABEL_OFFSET: u16 = 0x0174;
/// `mov si,0x273b` @`0x8579`, substituted when the entry equals `[0x2734]`
/// (`cmp si,[0x2734] / jne` @`0x8573`) — an ALIAS for one particular label, done
/// in the draw pass and again in the layout pass @`0x8467` (audit-fixes #492).
pub const SHIP_3D_TARGET_ALIAS_LABEL_OFFSET: u16 = 0x273b;
/// `cmp ax,0x28 / jl 0x8705` @`0x861E` (audit-fixes #493). The gated value is
/// `[0x2795]` @`0x8614` — the BRIDGE PANORAMA FRAME — so this is not an abstract
/// gate: the nav choice exists only while the view faces frames 40..60.
pub const SHIP_3D_NAV_CHOICE_MIN_GATE: u16 = 40;
/// `cmp ax,0x3c / jg 0x8705` @`0x8617`, the upper half of the panorama-frame
/// window described on [`SHIP_3D_NAV_CHOICE_MIN_GATE`] (audit-fixes #493).
pub const SHIP_3D_NAV_CHOICE_MAX_GATE: u16 = 60;
/// `sub ax,0x2d` @`0x8642` — the dynamic axis is biased by 45 before the `shl ax,3`
/// @`0x864B` that turns a choice index into a column (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_AXIS_BIAS: u16 = 45;
/// 287, and it is NEVER AN IMMEDIATE — the routine COMPUTES it with two adds,
/// `add ax,0xe8` @`0x8650` then `add ax,0x37` @`0x8653` (232 + 55). Searching the
/// image for `0x011F` in every instruction form returns zero hits, which is why
/// this looked unsourced (audit-fixes #491; same shape as #484's descriptor).
/// The bound is then `cmp bx,ax / jg` @`0x8656`.
pub const SHIP_3D_NAV_CHOICE_RIGHT_BASE: u16 = 287;
/// `sub ax,0x6e` @`0x865C` — the left edge is 110 left of the right edge, tested
/// by `cmp bx,ax / jl` @`0x8663` after a `js` guard @`0x865F` (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_X_WIDTH: u16 = 110;
/// `mov bx,0x48` @`0x8674` — the list's y origin, but only its BASE: the routine
/// then adds `|axis - 45|` @`0x8677` and a further `>> 2` of it @`0x867E`, while
/// the row pitch shrinks by HALF THAT QUARTER — `shr al,1 / sub cl,al`
/// @`0x8680`, i.e. `18 - (|axis-45| >> 3)`. The shift is on AL, the quarter's LOW
/// BYTE, so the port truncates to `u8` and then shifts, in that order.
/// Looking away from centre slides the list down and compresses its rows — a
/// perspective effect, not a fixed layout (audit-fixes #493).
pub const SHIP_3D_NAV_CHOICE_Y_BASE: u16 = 72;
/// `mov cl,0x12` @`0x8679` — a BASE, reduced by `shr al,1 / sub cl,al` @`0x8680`
/// before `div cl` @`0x868B` converts a cursor y into a row (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE: u8 = 18;
/// `cmp al,5 / jge 0x8705` @`0x868D` rejects any row at or past 5, and `mov cx,5`
/// @`0x8633` is the matching palette-write loop count (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_COUNT: u8 = 5;
/// `mov al,0x7b` @`0x862E`, written to the DAC index port `0x3C8` — the first of
/// the five choice colours, and the same 0x7B is added to the selected row
/// @`0x8697` to address that row's own entry (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_PALETTE_FIRST: u8 = 0x7b;
/// `mov word [0xa32],5` @`0x86AB` (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_PRESENTATION_MODE: u16 = 5;
/// `or byte [0x2793],0xc` @`0x86B6` — bits 2 and 3 raised together (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS: u8 = 0x0c;
/// `test byte [0x2793],8 / jne 0x8705` @`0x86F1` — bit 3 BLOCKS the handler call
/// @`0x8700`, which is why it is separate from the 0x0C the same routine raised
/// @`0x86B6` (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG: u8 = 0x08;
/// `mov word [0x279b],0x5a` @`0x86BB` — 90 ticks (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_HOLD_TICKS: u16 = 90;
/// `mov byte [0x2565],1` @`0x86C1` — the same widget phase cell the list driver
/// tests @`0x8874` (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_HANDLER_PHASE: u8 = 1;
/// `add ax,0x50` @`0x86CE` — 80, the offset added after the per-row multiply
/// (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_BASE: u16 = 80;
/// `mov cl,0x12 / mul cl` @`0x86CA` — the row pitch, applied to `bl - 1`
/// (`dec al` @`0x86C8`), so row 1 lands exactly on the base (audit-fixes #491).
pub const SHIP_3D_NAV_CHOICE_TARGET_Y_STEP: u16 = 18;
/// `mov word [0xac6],0x64` @`0x86D9` — the centring cell the layout pass reads as
/// `sub dx,[0xac6]` @`0x84AF`, so the nav choice re-centres the widget on x=100
/// before it opens (audit-fixes #494).
pub const SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X: u16 = 100;
/// `mov byte [0xada],0xa` @`0x86E4` — and `[0xada]` is exactly the DURATION
/// `ship_3d_interpolation_gate` divides by (`mov bl,[0xada]` @`0x1E63`), so the
/// nav choice's box animates over 10 ticks (audit-fixes #494).
pub const SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION: u8 = 10;
/// `mov ax,4` @`0x86E9`, the argument to `lcall 0xb1b:0x11d` @`0x86EC`
/// (audit-fixes #494).
pub const SHIP_3D_NAV_CHOICE_SELECT_SOUND: u16 = 4;
/// VM record type `0xC3`, TESTED at `cmp ax,0xc3` @`0x5D37` (the head of the
/// post-update chain, falling through to the `0xC4` test @`0x5D8F`) and again
/// @`0x6F21` (audit-fixes #509).
///
/// NOTE THE ASYMMETRY with [`SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE`]: that one
/// is also WRITTEN into a record (`mov word ptr [di],0xc4` @`0x5E13`), while no
/// immediate-form write of `0xC3` was found. That is NOT a claim that none exists
/// — #502 established that this binary builds values with shifts and adds often
/// enough that an immediate scan proves little — but the port sets this as a
/// `deferred_record_type`, and only the read side is evidenced here.
pub const SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE: u16 = 0x00c3;
pub const SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG: u8 = 0x04;
/// `test byte [0x2565],2` @`0x889C` — phase bit 1 of the same cell whose bit 0 is
/// [`SHIP_3D_NAV_CHOICE_HANDLER_PHASE`]; bit 1 means the box is mid-interpolation,
/// and the handler runs the gate `lcall 0x8b:0xfad` @`0x88AA` while it is set
/// (audit-fixes #494).
pub const SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING: u8 = 2;
/// `mov si,0xd16` @`0x8860` — `sn\radio.snd` (`DS:0x0D16`, file `0xE136`), handed
/// to the bank loader `lcall 0xb1b:0x855` @`0x8866` with **AX=1**, where the
/// `3D.snd`/`tb.snd` loads of #484/#506 pass AX=0. The whole routine is four
/// instructions ending `ret` @`0x886B`, immediately before nav-choice handler 4
/// (#494) — audit-fixes #510.
pub const SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET: u16 = 0x0d16;
/// `mov si,0x2567` @`0x8871`, the word list handler 4 hands to the layout widget
/// (audit-fixes #494). Handler 4 is the FIFTH entry of the dispatch table at
/// `cs:0x0F29` (file `0x8709`) — see [`SHIP_3D_NAV_CHOICE_COUNT`].
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TARGET_LIST_OFFSET: u16 = 0x2567;
/// `mov ax,0x2578 / mov [0x2569],ax` @`0x88F0` — the label installed when the
/// music is switched OFF, i.e. `MUSIC_ON`, the row that now offers to turn it back
/// on. The list at `DS:0x2567` is SELF-MODIFYING: slot 1 is patched in place
/// rather than there being two rows (audit-fixes #464, wired in #498).
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET: u16 = 0x2578;
/// `mov ax,0x2581 / mov [0x2569],ax` @`0x8907` — the `MUSIC_OFF` face, installed
/// when music is switched ON, and the one the SHIPPED list already carries
/// (audit-fixes #498).
pub const SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET: u16 = 0x2581;
/// `mov si,0xd3d` @`0x8914` — `mu	ablo2.voc`, played on the switch-ON branch
/// only, after `test byte [0xade],1` @`0x890D` (audit-fixes #498).
pub const SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET: u16 = 0x0d3d;
pub const SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS: u8 = 2;
/// `mov byte [0xada],6` @`0xB3CD` — the same duration cell the nav choice sets to
/// 10 (#494) and `ship_3d_interpolation_gate` divides by @`0x1E63`. The navigation
/// box therefore opens FASTER than the nav-choice box (audit-fixes #495).
pub const SHIP_3D_NAVIGATION_INTERPOLATION_DURATION: u8 = 6;
/// VM record type `0xC4`, both TESTED (`cmp ax,0xc4` @`0x5D8F`) and WRITTEN
/// (`mov word ptr [di],0xc4` @`0x5E13`, into the field selector `0x13` resolves
/// @`0x5E0B`). The write is what the port's `deferred_record_type` models — the
/// handler marks a record deferred by stamping its own opcode into it
/// (audit-fixes #509).
pub const SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE: u16 = 0x00c4;
pub const SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE: u16 = 2;
pub const SHIP_3D_NAVIGATION_RECORD_ACTIVE_FLAG: u8 = 0x01;
pub const SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG: u8 = 0x02;
pub const SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG: u16 = 0x0080;
/// `or byte [0x2793],4` @`0xB3C3` — bit 2 of the same HUD word the nav choice ORs
/// 0x0C into @`0x86B6` (audit-fixes #495).
pub const SHIP_3D_NAVIGATION_TARGET_LIST_FLAG: u8 = 0x04;
/// `mov si,0x253b` @`0xB3D7`, the word list passed to `list_widget_layout_unified`
/// by `lcall 0x71e:0xc48` @`0xB3DA` — which resolves to file `0x8428`, the widget
/// itself (audit-fixes #495).
pub const SHIP_3D_NAVIGATION_LAYOUT_TARGET_LIST_OFFSET: u16 = 0x253b;
/// 35 (`0x23`), written to BOTH `[0x1fa7]` @`0xB3FA` and `[0x5239]` @`0xB407`
/// (audit-fixes #495). CONSUMED by the plot clip test at `0x9B04`, which reads
/// `[0x5239]` @`0x9B19` as the top bound — so this is a real clip rectangle, not
/// a bookkeeping copy (#500).
pub const SHIP_3D_NAVIGATION_SCENE_BAND_TOP: u16 = 35;
/// `mov word [0x523b],0xa5` @`0xB40D` — 165, the clip bottom in force for the
/// `lcall 0x299:0xe2f` @`0xB415` that follows (audit-fixes #495). The same cell
/// is the bottom bound of the plot clip test, `cmp bx,[0x523b]` @`0x9B1F` (#500).
pub const SHIP_3D_NAVIGATION_RENDER_CLIP_BOTTOM: u16 = 165;
/// `mov word [0x523b],0xc8` @`0xB41D` — 200, RESTORED immediately after that
/// call, so the narrowed band applies to one draw only (audit-fixes #495).
pub const SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM: u16 = 200;
pub const SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP: u8 = 2;
/// `test word [0x2793],8` @`0x9733` (audit-fixes #496). Bit 3 of the HUD word —
/// the same bit the nav-choice dispatcher tests as a BLOCK @`0x86F1` (#491).
pub const SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG: u16 = 0x0008;
/// `test word [0x2793],4` @`0x975A` — bit 2, which selects between two otherwise
/// identical wrap paths (`0x976A` vs `0x97AD`) that differ only in step size
/// (audit-fixes #496).
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG: u16 = 0x0004;
/// `cmp ax,0xb4 / jl` @`0x9748`, the fold that turns a raw difference into the
/// SHORTEST angular distance: past 180 the routine does `sub ax,0x168 / neg ax`
/// @`0x974D` (audit-fixes #496).
pub const SHIP_3D_PROCEDURAL_HALF_TURN: u16 = 180;
/// `0x168` — 360, the panorama's modulus, appearing throughout this routine both
/// as the fold above @`0x974D` and as the wrap on every accumulated angle
/// (`cmp dx,0x168 / sub dx,0x168` @`0x976E`, and again @`0x97B1`, `0x977F`,
/// `0x97D7`) — audit-fixes #496.
pub const SHIP_3D_PROCEDURAL_FULL_TURN: u16 = 360;
/// `add cx,0x5a0` @`0x979D` — 1440, added to `angle * 4` (`shl bp,2` @`0x9794`)
/// and handed to `int 0x33` with AX=4 @`0x97A8`, the SET CURSOR POSITION call. So
/// the pointer rides a ring four units per degree, offset by a full 360*4 so the
/// ring never runs negative (audit-fixes #496).
pub const SHIP_3D_PROCEDURAL_MOUSE_RING: u16 = 1440;
/// The same 1440 as [`SHIP_3D_PROCEDURAL_MOUSE_RING`] in its SECOND role: the ring
/// MODULUS, `add bx,0x5a0` @`0x9807` when a delta goes negative and `cmp bx,0x5a0`
/// @`0x980B`. One value, two jobs — origin and wrap (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_MOUSE_CENTER_X: u16 = 1440;
/// `and word [0xa2a],0xfff8` @`0x97F6` — the cursor x is snapped to a multiple of
/// 8 after every auto-turn step, so the ring advances in whole 8-unit notches
/// (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_MOUSE_ALIGN_MASK: u16 = 0xfff8;
/// `cmp ax,0x1f / jle 0x97FC` @`0x9752` — within 31 of the target the routine
/// stops turning (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_CLOSE_ANGLE_THRESHOLD: u16 = 31;
/// `cmp ax,0x28 / jl 0x97FC` @`0x9762` — the wider 40 threshold used only on the
/// target-list branch, i.e. when `[0x2793]` bit 2 is set (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD: u16 = 40;
/// `add bp,0x28` @`0x977C` / `sub bp,0x28` @`0x978B` — the target-list branch
/// turns 40 at a time (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_TARGET_LIST_STEP: u16 = 40;
/// `sub bx,0x1e` @`0x97C4` / `add bx,0x1e` @`0x97D4` — the plain auto-rotate
/// branch turns 30 at a time, the SLOWER of the two (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP: u16 = 30;
/// `sub ax,0xa0` @`0x97F0`, applied after `shl ax,3` @`0x97ED`, so the cursor
/// target is `frame * 8 - 160` (audit-fixes #497).
pub const SHIP_3D_PROCEDURAL_ROTATION_OFFSET_BIAS: u16 = 0x00a0;
/// `mov bp,0x4f45` @`0x98CB`, the trig table the matrix build indexes with the
/// three angle cells read immediately after it (`[0x2f71]` @`0x98D1`, `[0x2f6d]`
/// @`0x98EF`, `[0x2f6f]` @`0x990C`) — audit-fixes #499.
pub const SHIP_3D_MATRIX_ANGLE_TABLE_OFFSET: u16 = 0x4f45;
/// Touched by the game at `mov di, word ptr [0x2f71]` @`0x098D1`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_MATRIX_ANGLE_A_OFFSET: u16 = 0x2f71;
/// Touched by the game at `mov di, word ptr [0x2f6d]` @`0x098EF`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_MATRIX_PROJECTION_ANGLE_OFFSET: u16 = 0x2f6d;
/// Touched by the game at `mov di, word ptr [0x2f6f]` @`0x0990C`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_MATRIX_ANGLE_C_OFFSET: u16 = 0x2f6f;
/// `mov si,0x2f7d` @`0x98CE` — the scratch the angle terms are written to before
/// the 3x3 compose reads them back as `[si]`..`[si+0x14]` (audit-fixes #499).
pub const SHIP_3D_MATRIX_TEMP_OFFSET: u16 = 0x2f7d;
/// `mov di,0x2f95` @`0x992D` — the destination the composed cells are `stosd`'d
/// into, nine dwords (audit-fixes #499).
pub const SHIP_3D_PROJECTION_MATRIX_OFFSET: u16 = 0x2f95;
/// Q15: every product in the compose is `imul` followed by `sar e_x,0xf` — first
/// at `0x9941`, then throughout (`0x996D`, `0x9982`, `0x9999`, `0x99A5`, ...).
/// The shift is 15, so the cells are 1.15 fixed point, which is what makes the
/// neutral value `0x8000` (audit-fixes #499).
pub const SHIP_3D_MATRIX_FIXED_SHIFT: u8 = 0x0f;
/// `mov ax,[0x2f65]` @`0x9A3F`, inside the point projector at `0x9A10`
/// (audit-fixes #500).
pub const SHIP_3D_PROJECTION_CAMERA_X_OFFSET: u16 = 0x2f65;
/// `mov ax,[0x2f67]` @`0x9A44` (audit-fixes #500).
pub const SHIP_3D_PROJECTION_CAMERA_Y_OFFSET: u16 = 0x2f67;
/// `mov ax,[0x2f69]` @`0x9A4A` — the three camera cells are read consecutively,
/// one per axis, before the matrix at `[0x2f95]` is applied (`mov bp,0x2f95`
/// @`0x9A31`) — audit-fixes #500.
pub const SHIP_3D_PROJECTION_CAMERA_Z_OFFSET: u16 = 0x2f69;
/// `mov word ptr [0x2f77],0x3e8` @`0x9A1D` — 1000, written as the projector's
/// LOOP COUNTER at entry, so the star field is a fixed-size cloud rather than a
/// list with a terminator (audit-fixes #500).
pub const SHIP_3D_POINT_CLOUD_COUNT: usize = 1000;
/// `mov si,0x2fc1` @`0x9A23` — the source the projector walks (audit-fixes #500).
pub const SHIP_3D_POINT_BUFFER_OFFSET: u16 = 0x2fc1;
/// `mov di,0x4f01` @`0x9A26` — the scratch vector each point is transformed
/// through, set up beside the source and matrix pointers (audit-fixes #500).
pub const SHIP_3D_PROJECTION_WORK_VECTOR_OFFSET: u16 = 0x4f01;
/// `mov word ptr [bp+0x24],ax` @`0x9AB0`, where `bp` is the PROJECTION MATRIX
/// base `0x2f95` (`mov bp,0x2f95` @`0x9A31`) — so `0x2f95 + 0x24 = 0x2FB9`. The
/// matrix is nine dwords (0x24 bytes) and the projected output begins IMMEDIATELY
/// after it; these are fields of one structure, not a separate buffer
/// (audit-fixes #501). The stored value is already screen-centred: `add ax,0xa0`
/// @`0x9AAD` adds 160 after the perspective divide.
pub const SHIP_3D_PROJECTED_X_OFFSET: u16 = 0x2fb9;
/// `mov word ptr [bp+0x26],ax` @`0x9AE5` = `0x2f95 + 0x26`, centred by
/// `add ax,0x64` @`0x9AE2` (100) — audit-fixes #501.
pub const SHIP_3D_PROJECTED_Y_OFFSET: u16 = 0x2fbb;
/// `mov word ptr [bp+0x28],cx` @`0x9AE8` = `0x2f95 + 0x28`. `cx` is the divisor
/// the two `idiv ecx` @`0x9AAA`/`0x9ADF` used, so the stored depth is the value
/// the perspective divide was performed BY (audit-fixes #501).
pub const SHIP_3D_PROJECTED_DEPTH_OFFSET: u16 = 0x2fbd;
/// Touched by the game at `sub ax, word ptr [0x5235]` @`0x033B4`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_PROJECTION_VIEWPORT_LEFT_OFFSET: u16 = 0x5235;
/// Touched by the game at `sub ax, word ptr [0x5237]` @`0x033C8`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_PROJECTION_VIEWPORT_RIGHT_OFFSET: u16 = 0x5237;
/// Touched by the game at `mov word ptr [0x5239], 0x23` @`0x0B19B`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_PROJECTION_VIEWPORT_TOP_OFFSET: u16 = 0x5239;
/// Touched by the game at `mov word ptr [0x523b], 0xa5` @`0x0B1A1`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_PROJECTION_VIEWPORT_BOTTOM_OFFSET: u16 = 0x523b;
/// `add ax,0xa0` @`0x9AAD` — applied to the projected x AFTER the perspective
/// divide, so the stored value is already screen-relative (audit-fixes #502).
pub const SHIP_3D_PROJECTION_SCREEN_CENTER_X: u16 = 160;
/// `add ax,0x64` @`0x9AE2`, the same treatment for y (audit-fixes #502).
pub const SHIP_3D_PROJECTION_SCREEN_CENTER_Y: u16 = 100;
/// 320, and like #491's 287 it is NEVER AN IMMEDIATE — the plot routine builds
/// the row stride out of two shifts (audit-fixes #502):
///
/// ```text
///   0x9B25  mov di, bx      di = y
///   0x9B27  xchg bh, bl     bx = y * 256   (bh was zero)
///   0x9B29  shl di, 6       di = y * 64
///   0x9B2C  add di, bx      di = y * 320
///   0x9B2E  add di, ax      + x
/// ```
///
/// So searching the image for `0x140` to source this would return nothing, and
/// the stride is a fact about two instructions rather than a stored constant.
pub const SHIP_3D_PROJECTION_SCREEN_WIDTH: usize = 320;
/// `sar eax,7` @`0x9AA4` and again @`0x9AD9` — the pre-divide scale applied to
/// both axes before `idiv ecx` (audit-fixes #502).
pub const SHIP_3D_PROJECTION_AXIS_SHIFT: u8 = 7;
/// `shr ax,0xc` @`0x9B3A`, applied to the DEPTH read back from `[bp+0x28]`
/// @`0x9B37` — the cell #501 sourced as `SHIP_3D_PROJECTED_DEPTH_OFFSET`
/// (audit-fixes #502).
pub const SHIP_3D_PROJECTION_SHADE_SHIFT: u8 = 12;
/// `neg al` @`0x9B3D` then `add al,0xef` @`0x9B3F`: the plotted colour is
/// `0xEF - (depth >> 12)`, so points DARKEN with distance down from 239. A base
/// plus a subtraction, not a palette index chosen by eye (audit-fixes #502).
pub const SHIP_3D_PROJECTION_SHADE_BASE: u8 = 239;
/// `mov bx,0x4f09` @`0x9BA5`, the anchor table the object pass walks
/// (audit-fixes #503).
pub const SHIP_3D_OBJECT_ANCHOR_OFFSET: u16 = 0x4f09;
/// `mov word ptr [0x2f77],0xb` @`0x9BB4` — 11, written into the SAME loop-counter
/// cell the point projector loads with 1000 (#500). One counter, reused per pass,
/// so the two counts are not simultaneous state (audit-fixes #503).
pub const SHIP_3D_OBJECT_ANCHOR_COUNT: usize = 11;
/// `add bx,6` @`0x9CF5` — 6 bytes per anchor, three words.
///
/// The COPY IS WIDER THAN THE RECORD: `mov eax,[bx]` / `mov eax,[bx+4]`
/// @`0x9BC2`-`0x9BCC` move EIGHT bytes into the work vector, two dword moves
/// instead of three word moves, so each record's copy runs two bytes into the
/// next. Only three words are then used (`sub` on `[di]`, `[di+2]`, `[di+4]`
/// @`0x9BEB`-`0x9BF6`), which is why the over-read is harmless — but a port that
/// copies 8 bytes per 8-byte stride would silently skip every other anchor
/// (audit-fixes #503).
pub const SHIP_3D_OBJECT_ANCHOR_STRIDE: u16 = 6;
/// `add ax,0x6212` @`0x9BDA`, the last step of the descriptor address
/// computation (audit-fixes #503).
pub const SHIP_3D_OBJECT_DESCRIPTOR_BASE_OFFSET: u16 = 0x6212;
/// `shl ax,5` @`0x9BD7` — 32 bytes per descriptor, expressed as a SHIFT, so an
/// immediate scan for 32 would never find it (audit-fixes #503).
pub const SHIP_3D_OBJECT_DESCRIPTOR_STRIDE: u16 = 32;
/// `add ax,0x15` @`0x9BD4` — the loop index is biased by 21 BEFORE the stride
/// multiply, so anchor 0 addresses descriptor 21, not descriptor 0. The full
/// address is `(index + 21) * 32 + 0x6212` (audit-fixes #503).
pub const SHIP_3D_OBJECT_DESCRIPTOR_INDEX_BIAS: u16 = 21;
/// `test ax,0x80 / je 0x9CF4` @`0x9BE1` — bit 7 of the descriptor's first word
/// gates the whole per-object body; a clear bit skips straight to the loop foot
/// (audit-fixes #503).
pub const SHIP_3D_OBJECT_VISIBLE_FLAG: u16 = 0x0080;
/// `test al,1` @`0x41E2` (its `je` @`0x41E4`) in the slot-state entry (`0x299:0x1241`, file
/// `0x41D1`). Bit 0 is cleared and bit 1 set in the same breath — `and al,0xfe /
/// or al,2` @`0x41E6` — so ACTIVE and DIRTY are a HANDOFF, not two independent
/// flags (audit-fixes #505).
pub const SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG: u16 = 0x0001;
/// `test al,0x81` @`0x42DD` in the extent entry (`0x299:0x133D`, file `0x42CD`) —
/// bits 0 and 7 tested TOGETHER, i.e. active AND visible. The slot-state entry
/// reaches the same pair as two branches (`or al,al / jns` @`0x41DE` for bit 7,
/// then `test al,1`), which is why the port needs both the mask and the single
/// flag (audit-fixes #505).
pub const SHIP_3D_SPRITE_SLOT_ACTIVE_MASK: u16 = 0x0081;
/// `or al,2` @`0x41E8`, set as bit 0 is cleared (audit-fixes #505).
pub const SHIP_3D_SPRITE_SLOT_DIRTY_FLAG: u16 = 0x0002;
/// `btr ax,4` @`0x42ED` — and the code contains the BIT INDEX 4, never the mask
/// `0x10`, because `btr` is a 386 bit-test-and-reset. Searching the image for
/// `0x10` as an extent mask would find nothing relevant. It is CLEARED on the
/// path where the new extent equals the stored one (`cmp cx,[si]` @`0x42E4`,
/// `cmp dx,[si+2]` @`0x42E8` both falling through), so the flag means "extent
/// differs" (audit-fixes #505).
pub const SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG: u16 = 0x0010;
/// `add ecx,0x10000` @`0x9C29`, reached only through `jns 0x9C30` @`0x9C27` — so
/// the bias is applied ONLY to a negative depth, wrapping it positive before the
/// divide. A zero depth exits earlier (`je 0x9CF4` @`0x9C23`), which is what keeps
/// the `div ecx` @`0x9C3D` safe (audit-fixes #504).
pub const SHIP_3D_OBJECT_DEPTH_WRAP_BIAS: i32 = 0x0001_0000;
/// COMPUTED, not stored: `mov eax,0x8000000` @`0x9C30` then `shr eax,7` @`0x9C36`
/// gives `0x100000`. Searching the image for `0x00100000` finds nothing — the
/// fourth instance of this compiler's shift-instead-of-immediate habit (#502,
/// #503). The scale is then `0x100000 / depth` (audit-fixes #504).
pub const SHIP_3D_OBJECT_SCALE_NUMERATOR: u32 = 0x0010_0000;
/// `shrd ax,dx,0xa` @`0x9CBB` and again @`0x9CC8` — a 386 DOUBLE-PRECISION
/// shift, so the scaled value spans `dx:ax` and 10 bits come out of the pair, not
/// out of a single word. A plain `>> 10` on a 16-bit value would lose the high
/// half (audit-fixes #504).
pub const SHIP_3D_OBJECT_SCALE_SHIFT: u8 = 10;
/// `mov word ptr [bp+0x2a],ax` @`0x9C40` with `bp = 0x2f95`, so `0x2f95 + 0x2a =
/// 0x2FBF` — the field immediately after the projected x/y/depth triple (#501).
/// The object pass stores its per-object scale into the same matrix-based
/// structure the point projector writes (audit-fixes #504).
pub const SHIP_3D_OBJECT_PROJECTED_SCALE_OFFSET: u16 = 0x2fbf;
/// Nav-destination world positions, the projector's INPUT table at `DS:0x4F09`
/// (file `0x12329`). TEN records of three `i16` at stride 6 — verified
/// byte-for-byte against the shipped image and against live interpreter memory.
///
/// Every entry is the SAME point. That is the game's data, not a placeholder: a
/// write watch over the table's linear range (`runtime_boot NAVWRITE`, which
/// carries a positive control) records zero writes across a full run, and the
/// literal `0x4F09` is referenced only by the projector at `0x9B98`. So the
/// destinations genuinely COINCIDE on screen rather than being spread out.
///
/// The projector loops eleven times over these ten entries (see `0x9CF5`); the
/// eleventh read lands in the trig table at `DS:0x4F45` and is gated off by the
/// entity active bit, so it is not represented here.
pub const NAV_DESTINATION_POINTS: [[i16; 3]; 10] = [[10200, 12100, 900]; 10];
/// Touched by the game at `mov word ptr [0x5249], 1` @`0x0AFAB`, found by decoding forward
/// from a verified routine entry (`re/tools/refs_in_routine.py`).
pub const SHIP_3D_GLOBAL_CLIP_SNAPSHOT_FLAG_OFFSET: u16 = 0x5249;
/// `mov di,0x6612` @`0x787C`, immediately before `lcall 0x299,0x210d` @`0x787F` —
/// so DI IS the list handed to the dirty-rects copy (`RENDER_DIRTY_RECTS_COPY_OFFSET`,
/// #490). The same pairing occurs at `0x8E9D`/`0x8EA0` (audit-fixes #506).
pub const SHIP_3D_DIRTY_RECT_LIST_OFFSET: u16 = 0x6612;
/// The value the WRITER stores, but the READER does not compare against it: the
/// walker terminates on `or ax,ax / js 0x517B` @`0x50B7` — a SIGN TEST, so ANY
/// negative entry ends the list and `0xFFFF` is simply the negative the game
/// happens to write. A port that terminates on equality with `0xFFFF` is stricter
/// than the game and would run past any other negative (audit-fixes #506).
pub const SHIP_3D_DIRTY_RECT_SENTINEL: u16 = 0xffff;
pub const SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET: u16 = 0x0acc;
pub const SHIP_3D_TEMP_SND_CALLBACK_OFFSETS: [u16; 3] = [0x0087, 0x0090, 0x009c];
/// `mov si,0xd23` @`0xB5D7` — `sn\3D.snd`, loaded by the `lcall 0xb1b:0x855`
/// @`0xB5DC` that follows with AX=0 (audit-fixes #506; the routine is #484's).
pub const SHIP_3D_TEMP_SND_PATH_OFFSET: u16 = 0x0d23;
/// `mov si,0xcfc` @`0xB60B` — `sn\tb.snd`, RESTORED through the same loader
/// @`0xB610` once the temporary bank has been used. The pair makes the swap
/// symmetric, which is why both offsets are named (audit-fixes #506).
pub const SHIP_3D_TB_SND_PATH_OFFSET: u16 = 0x0cfc;
pub const SHIP_3D_TEMP_SND_PHASE_COUNT: u8 = 3;
/// `mov word ptr [0x1fa3],0xffff` @`0xB66D`, on the sequence-active branch of
/// #484's tail (audit-fixes #506).
pub const SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL: u16 = 0xffff;
/// UNEXPLAINED, and marked so rather than given a plausible story.
///
/// `tools/check_literal_tables.py` reports this ABSENT — these sixteen bytes are
/// in no shipped image — and unlike the other absentees there is no known reason.
/// `NAV_CAMERA_ORIGIN` is absent because the port WIDENS words to `i32`;
/// `STATION_REST_FRAMES` because it HALVES stored angles; `GAME_SCREEN_PALETTE_DAC`
/// because part of it is capture-sourced and already labelled APPROX. This one has
/// no citation, no derivation and no doc before now.
///
/// DECODED (audit-fixes #288). The guess above was right — it is assembled by
/// consecutive stores — and the routine is the temp-SND setup's tail,
/// `0xB629`..`0xB643`, reached through `SHIP_3D_TEMP_SND_SETUP_OFFSET`
/// (`0x0A9A:0x05F1` = `0xB591`):
///
/// ```text
/// 0xB629  les di, ptr [0x522d]   ; destination is a FAR POINTER, not a DS cell
/// 0xB62D  xor eax, eax
/// 0xB630  stosw                  ; [0] = 0x0000
/// 0xB631  inc ax                 ; ax = 1
/// 0xB632  stosw                  ; [1] = 0x0001
/// 0xB633  add ax, 3              ; ax = 4
/// 0xB636  stosd                  ; [2],[3] = dword 0x00000004
/// 0xB638  mov ax, 0x140 / stosw  ; [4] = 320
/// 0xB63C  mov ax, 0xc8  / stosw  ; [5] = 200
/// 0xB640  xor eax, eax
/// 0xB642  stosd                  ; [6],[7] = dword 0
/// ```
///
/// Two things follow that the values alone could not tell you. First, `0`, `1`
/// and `4` are COMPUTED (`xor`/`inc`/`add ax,3`) — only `0x140` and `0xC8` are
/// immediates — so neither a byte search nor an immediate scan could ever have
/// found this table, which is why #227's method left it unsourced.
///
/// Second, THIS IS NOT EIGHT INDEPENDENT WORDS. The two `stosd`s make indices 3
/// and 7 the HIGH HALVES of 32-bit fields, not fields of their own; the real
/// shape is `u16, u16, u32, u16, u16, u32`. The port stores it as `[u16; 8]`
/// because that is how it is copied, but anything that starts INTERPRETING
/// index 3 or 7 as a separate value is reading a high half.
pub const SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR: [u16; 8] = [
    0x0000, 0x0001, 0x0004, 0x0000, 0x0140, 0x00c8, 0x0000, 0x0000,
];
/// THE NAVIGATION FINAL RESET's immediates, found in the tail of
/// `ship_3d_navigation_update` (`0xB34E`) — the routine that performs it. The
/// family was undocumented and unsourced until audit-fixes #282; these four now
/// name the instruction that writes each.
///
/// `mov word ptr [0x2793],9` @`0xB505` — the HUD flag word.
pub const SHIP_3D_FINAL_RESET_HUD_FLAGS: u16 = 0x0009;
/// `mov word ptr [0x279d],0x32` @`0xB511` — 50 ticks.
pub const SHIP_3D_FINAL_RESET_NAV_TIMER: u16 = 50;
/// `mov word ptr [0x1fab], 0xffff` @`0xB529`. #282 left this UNSOURCED after an
/// image-wide scan for `0xFFFF` returned too many hits to attribute; the fix was
/// to stop scanning and DECODE THE ROUTINE, which contains exactly two `0xFFFF`
/// word stores (audit-fixes #287). `DS:0x1FAB` is `vm_text_selector`, the signed
/// per-line selector the 0xA6 TEXT handler writes from its third byte — hence
/// SELECTOR, and hence this constant rather than the one below.
pub const SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL: u16 = 0xffff;
/// `mov word ptr [0x6788], 0xffff` @`0xB52F`, the second of the two. `DS:0x6788`
/// is `vm_active_line`, the active dialogue-line id — the ACTIVE RECORD. The two
/// stores are six bytes apart and in the same order as the fields they set, which
/// is what distinguishes them: on value alone they are identical.
pub const SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL: u16 = 0xffff;
/// `mov byte ptr [0x5b52],0xff` @`0xB57B`.
pub const SHIP_3D_FINAL_RESET_DIRTY_MARKER: u8 = 0xff;
/// The final reset restores the HOLD mode, so this is deliberately an ALIAS of
/// [`SHIP_3D_SCROLL_MODE_HOLD`] (`cmp word [0x524d],0xa` @`0xB6F0`, #496) rather
/// than a second copy of 10 — one value, one citation (audit-fixes #506).
pub const SHIP_3D_FINAL_RESET_SCROLL_MODE: u16 = SHIP_3D_SCROLL_MODE_HOLD;
/// `and byte ptr [0x67aa],0xfc` @`0xB54D` — clears the low two bits, so it is a MASK applied to what is there rather than a value written.
pub const SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK: u8 = 0xfc;
/// `cmp ax,8 / je 0x61DF` @`0x61B2` (audit-fixes #508). This kind and the two
/// below all branch to the SAME target, so the three are one behaviour, not three.
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8: u16 = 8;
/// `cmp ax,0x10 / je 0x61DF` @`0x61B7` — the name is the HEX, the value decimal
/// 16 (audit-fixes #508).
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10: u16 = 16;
/// `cmp ax,0x40 / jne` @`0x6114`, in the outer resolver — this kind takes the
/// selector-11 path @`0x611B` rather than the shared direct branch
/// (audit-fixes #508).
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40: u16 = 64;
/// `cmp ax,0x100` @`0x60E5` and again @`0x61AD` — the only kind with its own
/// two-word comparison path (selectors 12 and 14, then 9 or 10 by the result),
/// which is why it is named for the kind rather than as a DIRECT_* variant
/// (audit-fixes #508).
pub const SHIP_3D_OBJECT_KIND_POSITION_KIND100: u16 = 256;
/// `cmp ax,0x200 / je 0x61DF` @`0x61BC` — the third kind sharing the direct
/// branch (audit-fixes #508).
pub const SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200: u16 = 512;
/// `mov ax,0xb` @`0x611B`, passed to `vm_field_offset` @`0x611E` (`0x6023`, the
/// `bsf` matrix). Selector 11 is the POSITION row; the kind supplies the column
/// (audit-fixes #508).
pub const SHIP_3D_FIELD_SELECTOR_POSITION: u8 = 11;
/// `mov ax,9` @`0x6101` — the selector used when the kind-100 words MATCH.
pub const SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH: u8 = 9;
/// `inc ax` @`0x6108` — 9 + 1, taken when the compare at `0x6104` does NOT fall through. The mismatch selector is not an independent constant in the game; it is the match selector plus one.
pub const SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH: u8 = 10;
/// `mov ax,0xc` @`0x60F9`, resolved with kind `0x100` (`mov bx,0x100` @`0x60F6`) and read from the SI record.
pub const SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD: u8 = 12;
/// `mov ax,0xe` @`0x60EC`, resolved with the DI record's own kind and read from it.
pub const SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD: u8 = 14;
/// `mov ax,0x11` @`0x625B`, inside the nav source-list builder (`0x624B`): the
/// selector whose field the walk compares against the current target to decide
/// whether an object is a CHILD of it.
///
/// SAME SELECTOR AS `vm::VM_FIELD_OFFSET_SELECTOR_C2`, which names `0x11` from
/// the same instruction. Two constants, two modules, one selector — recorded
/// rather than merged, because the ship-3D name describes what the field MEANS
/// there (a parent link) and the VM name describes which opcode family reaches
/// it, and neither is wrong (audit-fixes #285).
pub const SHIP_3D_FIELD_SELECTOR_PARENT_LINK: u8 = 17;
/// `cmp si, -1` @`0x61CD` in the position walk: a parent link of `0xFFFF` means
/// FALL BACK TO THE ARCHE OBJECT, which the next instructions load from the
/// engine's named-object global (`mov si, word ptr gs:[0x6752]` @`0x61D2`).
///
/// A separate constant from the other `0xFFFF`s in this file on purpose. #285's
/// rule cuts both ways: a shared VALUE is not a shared RULE, and this one has its
/// own instruction, its own routine and its own meaning — "no parent, use the
/// arche" — which none of the reset sentinels share (audit-fixes #291).
pub const SHIP_3D_PARENT_LINK_SENTINEL: u16 = 0xffff;
/// `mov ax,5` @`0x6229` in the object-table bit test (`0x6210`). The selector is FIXED here, not derived from the object — the routine's own label warns about that (audit-fixes #274).
pub const SHIP_3D_SOURCE_BITSET_SELECTOR: u8 = 0x05;
/// `mov bx,2` @`0x622C`, feeding the `call 0x6023` @`0x622F` one instruction after the selector's `mov ax,5` — kind and selector are both fixed at the call site.
pub const SHIP_3D_SOURCE_BITSET_KIND: u16 = 0x0002;
/// `cmp ax, 1` @`0x6C36`, the SECOND arm of the C1 kind-0x10 source-list scan
/// (`0x6C1C`): `ax` is the scanned record's kind word (`mov ax,es:[bx]` @`0x6C24`),
/// and a kind of 1 falls through to the operand-flag test rather than the bitset
/// test. Located by the routine that gates on it, never by value match — 1 is far
/// too common to attribute by coincidence (audit-fixes #263).
pub const SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG: u16 = 0x0001;
/// `cmp ax, 2` @`0x6C27`, the FIRST arm of the same scan: kind 2 means the record
/// carries an object-table bitset, so the handler calls the bit test (`0x6210`)
/// with the stored operand and takes the resolved branch on carry (`jb` @`0x6C32`).
pub const SHIP_3D_C1_SOURCE_KIND_BITSET: u16 = 0x0002;
/// `test byte ptr es:[bx + 2], 2` @`0x6C3F` — the kind-1 arm's test. `bx` is loaded
/// from `[0x6736]` one instruction earlier (`mov bx,word ptr [0x6736]` @`0x6C3B`),
/// the operand the handler stashed at `0x6B6D`, so the byte tested is the OPERAND
/// RECORD's `+2` flags. That is what pins `operand_state_flags` in
/// [`select_ship_3d_c1_source_record`] to a specific byte of a specific record
/// rather than to an unspecified caller-supplied flag word.
pub const SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG: u8 = 0x02;
pub const SHIP_3D_C1_KIND10_RECORD_KIND: u16 = 16;
/// `mov ax,0x13` @`0x6C48` — 19, resolved with kind `0x10` (`mov bx,0x10` @`0x6C4B`) on the C1 SET path, which `0x6C2F` reaches via `jb` when the bit test returns carry.
pub const SHIP_3D_C1_DESTINATION_SELECTOR: u8 = 19;
pub const SHIP_3D_C1_RECORD_STATE_OPCODE: u16 = vm::OP_RECORD_STATE_MIN as u16;
pub const SHIP_3D_C1_RECORD_STATE_AUX_WORD: u16 = 2;

/// Parameter/result shape for [`update_ship_3d_transition_state`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTransitionState {
    pub hold_ticks: u16,
    pub transition_armed: bool,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dDepthState {
    pub depth_offset: u16,
    pub opening: bool,
    pub closing: bool,
    pub depth_step: u8,
}

/// Parameter/result shape for [`copy_ship_3d_plane_bands`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dPlaneBandCopy {
    pub row_count: usize,
    pub byte_count: usize,
    pub first_source_start: usize,
    pub first_dest_start: usize,
    pub second_source_start: usize,
    pub second_dest_start: usize,
    pub new_scroll_value: Option<u16>,
}

/// The target selector's DS state, one field per byte the routines touch.
///
/// ```text
///   current_target        DS:0x251B   cmp ax,[0x251b]      @0xB21A
///   target_select_phase   DS:0x252B   test byte [0x252b],1 @0xB2DC  (bit 1 @0xB2FD)
///   target_fallback       DS:0x252C   mov byte [0x252c],0  @0xB2BE
///   opening               DS:0x252F   mov byte [0x252f],1  @0xB6A5
///   depth_step            DS:0x2531   mov byte [0x2531],4  @0xB6A0
/// ```
///
/// Grouping them in one struct is the PORT'S choice — the game keeps them as
/// separate DS bytes — but each field models a specific byte, named here so the
/// mapping is checkable rather than implied by the field names.
///
/// `target_animation_tick` is the exception and is marked as such: no address is
/// claimed for it, because none was decoded.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetSelectorState {
    pub current_target: u16,
    pub target_select_phase: u8,
    pub target_fallback: bool,
    pub target_animation_tick: u8,
    pub opening: bool,
    pub depth_step: u8,
}

/// Parameter/result shape for [`select_ship_3d_target_record`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetSelection {
    /// The chosen target's record value: a record offset, the retained current target, or the
    /// exit sentinel.
    pub selected_target: u16,
    pub used_fallback_table: bool,
    pub ran_layout_prepass: bool,
    pub phase_gate_blocked: bool,
}

/// Parameter/result shape for [`step_ship_3d_interpolation_gate`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dInterpolationGate {
    pub duration_ticks: u8,
    pub current_tick: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dInterpolationStep {
    Active([u16; SHIP_3D_INTERPOLATION_WORDS]),
    Complete,
}

/// Parameter/result shape for [`layout_ship_3d_target_list`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetListLayout {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub max_label_width: u16,
    pub label_count: usize,
    pub has_extra_entry: bool,
    pub selector_mode_return: u16,
}

/// Parameter/result shape for [`hit_test_ship_3d_target_list`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTargetHitState {
    pub hover_row: u8,
    pub selected_row: u8,
    pub presentation_state: u16,
    pub requested_presentation_state: u16,
}

/// Parameter/result shape for [`hit_test_ship_3d_target_list`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetHitResult {
    pub inside: bool,
    pub activated: bool,
    pub hover_row: u8,
    pub selected_row: u8,
    pub return_value: u16,
    pub play_select_sound: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ship3dTargetTextSegment {
    TargetList,
    GameData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawCommand {
    pub row_index: usize,
    pub string_segment: Ship3dTargetTextSegment,
    pub string_offset: u16,
    pub x: u16,
    pub y: u16,
    pub color: u8,
    pub measured_width: u16,
    pub extra_entry: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ship3dTargetDrawResult {
    pub commands: Vec<Ship3dTargetDrawCommand>,
    pub final_hover_counter: u8,
}

/// Parameter/result shape for [`update_ship_3d_nav_choice_dispatch`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceState {
    pub selected_choice: u16,
    pub hud_flags: u8,
    pub handler_phase: u8,
    pub requested_presentation_state: u16,
    pub hold_ticks: u16,
    pub target_y: u16,
    pub target_layout_preserve_widths: bool,
    pub target_layout_center_x: u16,
    pub target_layout_extra_entry: bool,
    pub interpolation_duration_ticks: u8,
    pub interpolation_current_tick: u8,
}

/// Parameter/result shape for [`update_ship_3d_nav_choice_dispatch`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceGates {
    pub c2_presentation_gate: bool,
    pub left_motion_gate: bool,
    pub right_motion_gate: bool,
    pub menu_gate: bool,
    pub sound_gate: bool,
    pub presentation_active: bool,
}

/// Parameter/result shape for [`update_ship_3d_nav_choice_dispatch`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavChoiceInput {
    pub gate_value: u16,
    pub dynamic_axis: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
    pub activate: bool,
}

/// Parameter/result shape for [`update_ship_3d_nav_choice_dispatch`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceResult {
    pub gated: bool,
    pub reset_palette_range: bool,
    pub hovered_choice: Option<u8>,
    pub highlighted_palette_index: Option<u8>,
    pub committed_choice: Option<u8>,
    pub dispatched_choice: Option<u8>,
    pub play_select_sound: Option<u16>,
}

/// Parameter/result shape for [`run_ship_3d_nav_choice_handler_0`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceHandlerEffect {
    pub deferred_record_type: Option<u16>,
    pub deferred_record_related: Option<u16>,
    pub cleared_handler_phase: bool,
    pub ran_layout_prepass: bool,
    pub copied_layout_rect_snapshot: bool,
    pub adjusted_target_records: bool,
    pub phase_gate_blocked: bool,
    pub cleared_selected_choice: bool,
    pub cleared_hud_target_list_flag: bool,
    pub load_snd_bank_path: Option<u16>,
    pub load_voc_path: Option<u16>,
    pub start_voc_playback: bool,
    pub reset_interpolation_tick: bool,
    pub rebuilt_target_records: bool,
    pub set_input_gate_b: bool,
}

/// Parameter/result shape for [`run_ship_3d_nav_choice_handler_4`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavChoiceHandler4State {
    pub layout_rect_snapshot: [u16; SHIP_3D_INTERPOLATION_WORDS],
    pub menu_gate: bool,
    pub secondary_menu_gate: bool,
    pub voc_enabled: bool,
    pub voc_stream_phase: u8,
    pub tablo2_voc_active: bool,
    pub tablo2_voc_reset_gate: bool,
    pub active_target_list_offset: u16,
    pub shared_motion_gate: bool,
    pub left_motion_gate: bool,
    pub right_motion_gate: bool,
    pub sound_gate: u8,
    pub target_activate_flag: bool,
    pub target_activate_secondary_flag: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSequenceState {
    pub exit_pending: bool,
    pub sequence_active: bool,
    pub opening: bool,
    pub interpolation_duration_ticks: u8,
    pub framebuffer_dirty: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSequenceEffect {
    pub ran_temp_snd_setup: bool,
    pub ran_procedural_update: bool,
    pub blocked_by_presentation_active: bool,
    pub copied_framebuffer: bool,
    pub interpolation_active: bool,
    pub queried_target_list: bool,
    pub armed_exit_pending: bool,
    pub armed_opening_exit: bool,
    pub final_reset_pending: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProceduralUpdateState {
    pub hud_flags: u16,
    pub angle: u16,
    pub mouse_x: u16,
    pub mouse_y: u16,
    pub hold_ticks: u16,
    pub nav_timer: u16,
    pub mouse_delta_accumulator: u16,
    pub mouse_button_state: u16,
    pub mouse_sector: u16,
    pub rotation_direction_positive: bool,
    pub projection_angle: u16,
    pub rotation_offset: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProceduralUpdateEffect {
    pub cleared_hud_active_flag: bool,
    pub initialized_nav_timer: bool,
    pub applied_hud_rotation: bool,
    pub adjusted_target_list_mouse: bool,
    pub auto_rotated_angle: bool,
    pub updated_projection_angle: bool,
    pub mouse_set_position: Option<(u16, u16)>,
    pub carry_set: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dAngleTableEntry {
    pub cosine: i16,
    pub sine: i16,
}

/// The recovered ship-3D rotation trig table at `DS:0x4F45` in BLOODPRG.EXE:
/// 180 `(cosine, sine)` pairs, one per **2 degrees** (index 45 = 90°, 90 = 180°,
/// wrapping at 180), stored at Q14 amplitude `0x4000`. `matrix_pair_for_angle`
/// doubles each value to Q15 before the fixed-point matrix math. Angle words
/// (`DS:0x2F71` etc.) index this table directly. Verified byte-exact against the
/// binary by `tests::angle_table_matches_binary`.
#[rustfmt::skip]
pub const SHIP_3D_ANGLE_TABLE: [Ship3dAngleTableEntry; 180] = {
    const fn e(cosine: i16, sine: i16) -> Ship3dAngleTableEntry {
        Ship3dAngleTableEntry { cosine, sine }
    }
    [
        e(16384, 0), e(16374, 571), e(16344, 1142), e(16294, 1712), e(16224, 2280), e(16135, 2845),
        e(16025, 3406), e(15897, 3963), e(15749, 4516), e(15582, 5062), e(15395, 5603), e(15190, 6137),
        e(14967, 6663), e(14725, 7182), e(14466, 7691), e(14188, 8191), e(13894, 8682), e(13582, 9161),
        e(13254, 9630), e(12910, 10086), e(12550, 10531), e(12175, 10963), e(11785, 11381), e(11381, 11785),
        e(10963, 12175), e(10531, 12550), e(10086, 12910), e(9630, 13254), e(9161, 13582), e(8682, 13894),
        e(8192, 14188), e(7691, 14466), e(7182, 14725), e(6663, 14967), e(6137, 15190), e(5603, 15395),
        e(5062, 15582), e(4516, 15749), e(3963, 15897), e(3406, 16025), e(2845, 16135), e(2280, 16224),
        e(1712, 16294), e(1142, 16344), e(571, 16374), e(0, 16384), e(-571, 16374), e(-1142, 16344),
        e(-1712, 16294), e(-2280, 16224), e(-2845, 16135), e(-3406, 16025), e(-3963, 15897), e(-4516, 15749),
        e(-5062, 15582), e(-5603, 15395), e(-6137, 15190), e(-6663, 14967), e(-7182, 14725), e(-7691, 14466),
        e(-8191, 14188), e(-8682, 13894), e(-9161, 13582), e(-9630, 13254), e(-10086, 12910), e(-10531, 12550),
        e(-10963, 12175), e(-11381, 11785), e(-11785, 11381), e(-12175, 10963), e(-12550, 10531), e(-12910, 10086),
        e(-13254, 9630), e(-13582, 9161), e(-13894, 8682), e(-14188, 8192), e(-14466, 7691), e(-14725, 7182),
        e(-14967, 6663), e(-15190, 6137), e(-15395, 5603), e(-15582, 5062), e(-15749, 4516), e(-15897, 3963),
        e(-16025, 3406), e(-16135, 2845), e(-16224, 2280), e(-16294, 1712), e(-16344, 1142), e(-16374, 571),
        e(-16384, 0), e(-16374, -571), e(-16344, -1142), e(-16294, -1712), e(-16224, -2280), e(-16135, -2845),
        e(-16025, -3406), e(-15897, -3963), e(-15749, -4516), e(-15582, -5062), e(-15395, -5603), e(-15190, -6137),
        e(-14967, -6663), e(-14725, -7182), e(-14466, -7691), e(-14188, -8191), e(-13894, -8682), e(-13582, -9161),
        e(-13254, -9630), e(-12910, -10086), e(-12550, -10531), e(-12175, -10963), e(-11785, -11381), e(-11381, -11785),
        e(-10963, -12175), e(-10531, -12550), e(-10086, -12910), e(-9630, -13254), e(-9161, -13582), e(-8682, -13894),
        e(-8192, -14188), e(-7691, -14466), e(-7182, -14725), e(-6663, -14967), e(-6137, -15190), e(-5603, -15395),
        e(-5062, -15582), e(-4516, -15749), e(-3963, -15897), e(-3406, -16025), e(-2845, -16135), e(-2280, -16224),
        e(-1712, -16294), e(-1142, -16344), e(-571, -16374), e(0, -16384), e(571, -16374), e(1142, -16344),
        e(1712, -16294), e(2280, -16224), e(2845, -16135), e(3406, -16025), e(3963, -15897), e(4516, -15749),
        e(5062, -15582), e(5603, -15395), e(6137, -15190), e(6663, -14967), e(7182, -14725), e(7691, -14466),
        e(8191, -14188), e(8682, -13894), e(9161, -13582), e(9630, -13254), e(10086, -12910), e(10531, -12550),
        e(10963, -12175), e(11381, -11785), e(11785, -11381), e(12175, -10963), e(12550, -10531), e(12910, -10086),
        e(13254, -9630), e(13582, -9161), e(13894, -8682), e(14188, -8192), e(14466, -7691), e(14725, -7182),
        e(14967, -6663), e(15190, -6137), e(15395, -5603), e(15582, -5062), e(15749, -4516), e(15897, -3963),
        e(16025, -3406), e(16135, -2845), e(16224, -2280), e(16294, -1712), e(16344, -1142), e(16374, -571),
    ]
};

/// Parameter/result shape for [`build_ship_3d_projection_matrix`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dMatrixAngles {
    pub angle_2f71: u16,
    pub projection_angle_2f6d: u16,
    pub angle_2f6f: u16,
}

/// Parameter/result shape for [`build_ship_3d_projection_matrix`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionMatrix {
    pub terms: [i32; 9],
}

/// Parameter/result shape for [`project_ship_3d_point`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionPoint {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

/// Parameter/result shape for [`project_ship_3d_point`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionOrigin {
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

/// Parameter/result shape for [`ship_3d_nav_entity_for_slot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectedPoint {
    pub x: u16,
    pub y: u16,
    pub depth: u16,
}

/// Parameter/result shape for [`ship_3d_nav_entity_for_slot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectionViewport {
    pub left: u16,
    pub right: u16,
    pub top: u16,
    pub bottom: u16,
}

/// Parameter/result shape for [`plot_ship_3d_projected_point`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dProjectedPixel {
    pub offset: usize,
    pub shade: u8,
}

/// The engine's entity table is 32 records of 32 bytes at `DS:0x6212`, bounded
/// by the layout identity `0x6212 + 32*32 = 0x6612` (the dirty-rect list). The
/// nav projector `0x9B98` drives the LAST ELEVEN of them: its loop counter runs
/// `0x0B-1..0` and the record address is `0x6212 + ((i + 0x15) << 5)`.
pub const SHIP_3D_ENTITY_COUNT: u16 = 32;
/// The entity id the nav projector's slot 0 maps to (`0x9B98`).
pub const SHIP_3D_NAV_ENTITY_BASE: u16 = 0x15;
/// `DS:0x6212` — the entity table's base, in DS-relative bytes.
pub const SHIP_3D_ENTITY_TABLE: u16 = 0x6212;
/// Bytes per entity record (re-confirmed by the `rep movsd cx=8` copy at `0x4316`).
pub const SHIP_3D_ENTITY_STRIDE: u16 = 32;

/// The entity id a nav projector slot writes to, and its record's DS offset.
/// Slots beyond the eleven the projector drives return `None` rather than
/// wrapping past the table (`0x6612`).
pub fn ship_3d_nav_entity_for_slot(slot: usize) -> Option<(u16, u16)> {
    let id = SHIP_3D_NAV_ENTITY_BASE.checked_add(u16::try_from(slot).ok()?)?;
    if id >= SHIP_3D_ENTITY_COUNT {
        return None;
    }
    Some((id, SHIP_3D_ENTITY_TABLE + id * SHIP_3D_ENTITY_STRIDE))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dObjectSpriteDescriptor {
    /// The entity id this slot writes into (`0x6212 + (id << 5)`), so a slot can
    /// be addressed the way the rest of the engine addresses entities — the
    /// hover status panel, for one, reads ENTITY `0x1F`'s record directly
    /// (`DS:0x65F2`, the last of the 32). `None` until the projector assigns it.
    pub entity_id: Option<u16>,
    pub flags: u16,
    pub source_width: u16,
    pub source_height: u16,
    pub draw_x: u16,
    pub draw_y: u16,
    pub extent_width: u16,
    pub extent_height: u16,
    pub committed_draw_x: u16,
    pub committed_draw_y: u16,
    pub committed_extent_width: u16,
    pub committed_extent_height: u16,
}

/// Parameter/result shape for [`ship_3d_nav_entity_for_slot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dObjectSpriteProjection {
    pub projected: Ship3dProjectedPoint,
    pub depth_scale: u16,
    pub scaled_width: u16,
    pub scaled_height: u16,
    pub draw_x: u16,
    pub draw_y: u16,
}

/// Parameter/result shape for [`ship_3d_nav_entity_for_slot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dSpriteSlotUpdateEffect {
    pub ran: bool,
    pub marked_dirty: bool,
    pub updated_position: bool,
    pub updated_extent: bool,
    pub cleared_extent_changed_flag: bool,
    pub committed_geometry: bool,
}

/// Parameter/result shape for [`ship_3d_nav_entity_for_slot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ship3dDirtyRectList {
    pub rects: Vec<Ship3dProjectionViewport>,
    pub sentinel: u16,
}

impl Default for Ship3dDirtyRectList {
    fn default() -> Self {
        Self {
            rects: Vec::new(),
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        }
    }
}

/// Parameter/result shape for [`commit_ship_3d_global_clip_snapshot`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dDirtyRectSnapshotEffect {
    pub ran: bool,
    pub wrote_clip_rect: bool,
    pub wrote_sentinel: bool,
    pub cleared_snapshot_flag: bool,
}

/// Parameter/result shape for [`collect_ship_3d_dirty_sprite_slot_render_commands`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dSpriteSlotRenderCommand {
    pub slot_index: usize,
    pub dispatch_index: u8,
    pub destination_remap_mode: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub slot_rect: Ship3dProjectionViewport,
    pub dirty_rect: Ship3dProjectionViewport,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTempSndState {
    pub trigger: bool,
    pub auxiliary_trigger: bool,
    pub phase: u8,
    pub sequence_active: bool,
    pub plane_copy_enabled: bool,
    pub scene_selector: u16,
    pub hold_ticks: u16,
    pub fullscreen_refresh: bool,
    pub setup_flag_a: bool,
    pub setup_flag_b: bool,
    pub viewport_descriptor: [u16; 8],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dTempSndEffect {
    pub ran: bool,
    pub selected_callback_offset: Option<u16>,
    pub next_phase: Option<u8>,
    pub load_snd_bank_path: Option<u16>,
    pub restore_snd_bank_path: Option<u16>,
    pub preserved_mouse_position: bool,
    pub reset_callback_bank_gate: bool,
    pub called_presentation_callback: bool,
    pub reset_hold_ticks: bool,
    pub wrote_viewport_descriptor: bool,
    pub sequence_branch: bool,
    pub non_sequence_branch: bool,
    pub temporarily_disabled_plane_copy: bool,
    pub enabled_plane_copy: bool,
    pub reset_scene_selector: bool,
    pub reset_setup_flags: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavigationFinalResetState {
    pub exit_pending: bool,
    pub opening: bool,
    pub hud_flags: u16,
    pub nav_choice_hold_ticks: u16,
    pub nav_choice_timer: u16,
    pub post_reset_gate: bool,
    pub navigation_gate: bool,
    pub dialogue_state: u16,
    pub scene_band_top: u16,
    pub scene_selector: u16,
    pub active_record: u16,
    pub presentation_gate: bool,
    pub pending_state_byte: bool,
    pub subtitle_gate: bool,
    pub presentation_defer_active: bool,
    pub secondary_presentation_defer_active: bool,
    pub plane_copy_enabled: bool,
    pub sequence_active: bool,
    pub status_flags: u8,
    pub secondary_status_flag: bool,
    pub dirty_marker: u8,
    pub scroll_value: u16,
    pub scroll_mode: u16,
}

impl Default for Ship3dNavigationFinalResetState {
    fn default() -> Self {
        Self {
            exit_pending: false,
            opening: false,
            hud_flags: 0,
            nav_choice_hold_ticks: 0,
            nav_choice_timer: 0,
            post_reset_gate: false,
            navigation_gate: false,
            dialogue_state: 0,
            scene_band_top: 0,
            scene_selector: 0,
            active_record: 0,
            presentation_gate: false,
            pending_state_byte: false,
            subtitle_gate: false,
            presentation_defer_active: false,
            secondary_presentation_defer_active: false,
            plane_copy_enabled: false,
            sequence_active: false,
            status_flags: 0,
            secondary_status_flag: false,
            dirty_marker: 0,
            scroll_value: 0,
            scroll_mode: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationFinalResetEffect {
    pub ran: bool,
    pub reentered_active_sequence: bool,
    pub cleared_dialogue_state: bool,
    pub reset_hud_state: bool,
    pub reset_presentation_gates: bool,
    pub reset_sequence_flags: bool,
    pub reset_status_flags: bool,
    pub copied_backbuffer_restore_block: bool,
    pub cleared_overlay_scratch: bool,
    pub reset_scroll_state: bool,
    pub called_render_clear: bool,
    pub called_input_reset: bool,
    pub called_target_cleanup: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationRuntimeRecord {
    pub offset: u16,
    pub kind_flags: u16,
    pub state_flags: u8,
    pub counter_link: u16,
    pub related_target: u16,
    pub source_parent: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationSourceEntry {
    pub record_offset: u16,
    pub entry_kind: u16,
}

/// One object record as the POSITION WALK (`0x60DD`) needs to see it.
///
/// NOT A BYTE-LAYOUT MIRROR, and the distinction matters. In the game a record
/// has no fixed field positions: every access goes through
/// `vm_field_offset(selector, kind)` (`0x6023`), which indexes a matrix by
/// SELECTOR and by the record's own KIND, so the same selector lands at
/// different offsets in records of different kinds. This struct pre-resolves the
/// three selectors the walk uses, so the port looks them up once instead of at
/// every step. `offset` is the record's ADDRESS; the rest are fetched VALUES.
///
/// WHICH KINDS HAVE WHICH COLUMNS is readable straight out of the matrix at
/// `DS:0x6D60`, and it confirms the naming here: selectors 9, 10 and 12 are
/// nonzero ONLY in column 8, and column 8 is kind `0x100` (the column is the
/// kind's lowest set bit, so column k ↔ kind 2^k). The `kind100_` fields really
/// do exist only for kind-`0x100` records; `None` for any other kind is the
/// table's own answer, not a gap in the port (audit-fixes #289).
///
/// A ZERO OFFSET IS NOT AUTOMATICALLY "ABSENT", which is the trap here.
/// `vm_field_offset` (`0x6023`) just returns `matrix[selector*16 + bsf(kind)]`
/// with no zero-handling of its own, and its callers differ: the distance
/// routine adds it unconditionally (`add ax,si` @`0x6121`, `add di,ax` @`0x6167`),
/// so for kind `0x40` — whose selector-11 column IS 0 — the position field
/// legitimately sits AT the record's start. Only code that explicitly tests the
/// result treats 0 as absence. See [`resolve_ship_3d_position_field`], where the
/// port's zero-test is flagged as having no instruction behind it.
///
/// Field → selector, each cited on its own constant:
/// - `kind_flags` — the kind word at the record's own start, `mov ax,[si]` @`0x60E3`
/// - `parent_link` — [`SHIP_3D_FIELD_SELECTOR_PARENT_LINK`] (0x11)
/// - `kind100_match_word` — [`SHIP_3D_FIELD_SELECTOR_KIND100_MATCH_WORD`] (12),
///   resolved with the fixed kind `0x100`, not the record's own
/// - `kind100_relation_word` — [`SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD`] (14)
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dPositionRecord {
    /// The record's own address, the value the walk carries in `si`/`di` and
    /// compares — not a field read out of it.
    pub offset: u16,
    /// `mov ax,[si]` @`0x60E3`, the kind word at the record's start. Also the
    /// second argument to every `vm_field_offset` call below, which is why a
    /// wrong kind here mis-resolves every other field rather than just one.
    pub kind_flags: u16,
    /// Selector-0x11 parent/reference link. `None` represents the binary's
    /// `0xffff` sentinel, which falls back to the named arche object.
    pub parent_link: Option<u16>,
    /// Selector-12 field, compared against the walk's inherited compare word to
    /// choose selector 9 (match) or 10 (mismatch) at `0x6104`.
    pub kind100_match_word: Option<u16>,
    /// Selector-14 field. `None` when the kind lacks column 14, in which case
    /// [`kind100_relation_word`] yields `kind_flags` instead.
    pub kind100_relation_word: Option<u16>,
}

/// Parameter/result shape for [`ship_3d_position_field_distance`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dPositionField {
    pub offset: u16,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ship3dRecordStateSlot {
    pub opcode: u16,
    pub operand: u16,
    pub aux_word: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dC1DestinationWrite {
    pub destination_record_offset: u16,
    pub slot: Ship3dRecordStateSlot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ship3dNavigationTriggerState {
    pub trigger_active: bool,
    pub current_target: u16,
    pub requested_presentation_state: u16,
    pub hud_flags: u8,
    pub interpolation_duration_ticks: u8,
    pub interpolation_current_tick: u8,
    pub target_query_mode: bool,
    pub layout_rect_snapshot: [u16; SHIP_3D_INTERPOLATION_WORDS],
    pub sequence_active: bool,
    pub scene_band_top: u16,
    pub render_clip_top: u16,
    pub render_clip_bottom: u16,
    pub active_dialogue_record: u16,
    pub closing: bool,
    pub depth_step: u8,
}

impl Default for Ship3dNavigationTriggerState {
    fn default() -> Self {
        Self {
            trigger_active: false,
            current_target: 0,
            requested_presentation_state: 0,
            hud_flags: 0,
            interpolation_duration_ticks: 0,
            interpolation_current_tick: 0,
            target_query_mode: false,
            layout_rect_snapshot: [0; SHIP_3D_INTERPOLATION_WORDS],
            sequence_active: false,
            scene_band_top: 0,
            render_clip_top: 0,
            render_clip_bottom: SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM,
            active_dialogue_record: 0,
            closing: false,
            depth_step: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ship3dNavigationTriggerEffect {
    pub candidate_records: Vec<u16>,
    pub copied_pending_presentation_state: bool,
    pub incremented_counter_record: Option<u16>,
    pub deferred_record_type: Option<u16>,
    pub deferred_record_related: Option<u16>,
    pub candidate_handler_record: Option<u16>,
    pub opened_target_list: bool,
    pub reset_interpolation_tick: bool,
    pub ran_layout_prepass: bool,
    pub copied_layout_x_and_width: bool,
    pub cleared_trigger: bool,
    pub started_sequence: bool,
    pub set_scene_band: bool,
    pub restored_render_clip: bool,
    pub cleared_active_dialogue_record: bool,
    pub requested_closing: bool,
}

/// The ship-3D transition updater, `ship_3d_transition_state_update` @`0xB692`:
///
/// ```text
///   0xB692  test byte [0x2533],1     ARMED?
///   0xB699  cmp word [0xb3b],0x78    not armed: the hold timer vs 120
///   0xB69E  jbe 0xb6dc               at or below -> nothing happens
///   0xB6A0  mov byte [0x2531],4      open step
///   0xB6A5  mov byte [0x252f],1      opening flag
///   0xB6AA  mov byte [0x2533],1      now armed
///   0xB6B1  cmp word [0xb3b],0       armed: timer exhausted?
///   0xB6B8  mov byte [0x2531],8      close step -- twice the open rate
///   0xB6BD  mov byte [0x2530],1      closing flag
///   0xB6C2  mov byte [0x2533],0      disarmed
/// ```
///
/// `DS:0x2533` is the armed latch, `0x2531` the step, `0x252F`/`0x2530` the
/// opening/closing flags, `0x0B3B` the hold timer. This function had NO doc and
/// was settled ASM regardless — one of the 91 such rows counted in #141.
pub fn update_ship_3d_transition_state(state: &mut Ship3dTransitionState, random_gate_zero: bool) {
    if !state.transition_armed {
        if state.hold_ticks > SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD {
            state.depth_step = SHIP_3D_TRANSITION_OPEN_STEP;
            state.opening = true;
            state.transition_armed = true;
        }
        return;
    }

    if state.hold_ticks == 0 {
        start_closing_transition(state);
        return;
    }

    if !state.opening && random_gate_zero {
        start_closing_transition(state);
    }
}

pub fn step_ship_3d_depth_scroll(state: &mut Ship3dDepthState) {
    if state.opening {
        if state.depth_offset == SHIP_3D_MAX_DEPTH_OFFSET {
            state.opening = false;
            return;
        }

        let next = add_to_low_byte(state.depth_offset, state.depth_step);
        state.depth_offset = if (next as i16) < SHIP_3D_MAX_DEPTH_OFFSET as i16 {
            next
        } else {
            SHIP_3D_MAX_DEPTH_OFFSET
        };
        return;
    }

    if !state.closing {
        return;
    }

    if state.depth_offset == 0 {
        state.closing = false;
        return;
    }

    let next_low = (state.depth_offset as u8).wrapping_sub(state.depth_step);
    state.depth_offset = if next_low & 0x80 == 0 {
        (state.depth_offset & 0xff00) | next_low as u16
    } else {
        0
    };
}

/// The planar band copy, `ship_3d_plane_band_copy` @`0xB6DD`:
///
/// ```text
///   0xB6E5  test byte [0x252e],1     the copy-enabled gate
///   0xB6EC  mov bx,[0x2527]          the depth offset
///   0xB6F0  cmp word [0x524d],0xa    scroll mode 10 SKIPS the scroll update
///   0xB6F7  ax = bx+bx / cmp ax,0x64 / jle / mov ax,0x64   clamp 2*depth to 100
///   0xB703  sub ax,0x64 / neg ax / mov [0x524f],ax         store 100 - that
///   0xB70B  mov dx,0x3c4 / mov ax,0xf02 / out dx,ax        map mask = all 4 planes
///   0xB712  les di,[0x5219]          destination
///   0xB718  mov si,0xc000            SOURCE PAGE 0
///   0xB71C  ax = bx+0x23 / dl=0x50 / mul dl                (depth + 35) * 80
/// ```
///
/// So the band is `(depth_offset + SHIP_3D_PLANE_BASE_ROWS) *
/// SHIP_3D_PLANE_ROW_BYTES` bytes — 35 rows of 80 — from
/// `SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET`, which is where those three constants come
/// from. `0x524F` holds `100 - min(2*depth, 100)`, and scroll mode `0xA` leaves it
/// untouched.
///
/// Cited here because this function was settled ASM with no doc (#141's queue);
/// `re/labels.csv` already named the routine, from the mis-anchored-label fix in
/// #101.
pub fn copy_ship_3d_plane_bands(
    dest: &mut [u8],
    video_segment: &[u8],
    depth_offset: u16,
    plane_copy_enabled: bool,
    scroll_mode: u16,
) -> Option<Ship3dPlaneBandCopy> {
    if !plane_copy_enabled {
        return None;
    }

    let byte_count = ship_3d_plane_band_byte_count(depth_offset);
    if byte_count > SHIP_3D_PLANE_PAGE_BYTES {
        return None;
    }

    let first_source_start =
        SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + (SHIP_3D_PLANE_PAGE_BYTES - byte_count);
    let first_source_end = first_source_start.checked_add(byte_count)?;
    let second_source_start = SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET;
    let second_source_end = second_source_start.checked_add(byte_count)?;
    let second_dest_start = SHIP_3D_PLANE_DEST_BYTES.checked_sub(byte_count)?;
    let second_dest_end = second_dest_start.checked_add(byte_count)?;

    let first_source = video_segment.get(first_source_start..first_source_end)?;
    let second_source = video_segment.get(second_source_start..second_source_end)?;
    dest.get_mut(0..byte_count)?.copy_from_slice(first_source);
    dest.get_mut(second_dest_start..second_dest_end)?
        .copy_from_slice(second_source);

    Some(Ship3dPlaneBandCopy {
        row_count: byte_count / SHIP_3D_PLANE_ROW_BYTES,
        byte_count,
        first_source_start,
        first_dest_start: 0,
        second_source_start,
        second_dest_start,
        new_scroll_value: (scroll_mode != SHIP_3D_SCROLL_MODE_HOLD)
            .then(|| ship_3d_scroll_value(depth_offset)),
    })
}

/// The four-word interpolation gate, `ship_3d_interpolation_gate` @`0x1E5D`
/// (reached far as `0x008B:0x0FAD`):
///
/// ```text
///   0x1E63  mov bl,[0xada]           the DURATION
///   0x1E67  cmp bl,[0xadb] / je      duration == current tick -> complete
///   0x1E6D  inc byte [0xadb]         advance the tick FIRST
///   0x1E71  lodsw / sub ax,[di]      delta = source - dest
///   0x1E74  idiv bl                  delta / duration   (SIGNED, 8-bit quotient)
///   0x1E76  imul byte [0xadb]        * the tick
///   0x1E7A  mov dx,[di] / add dx,ax  dest + that
/// ```
///
/// The ORDER is load-bearing: the game divides and THEN multiplies, so each step
/// carries the truncation of an 8-bit quotient. Multiplying first would give a
/// different value for most non-exact divisions, and it is the shape a port
/// naturally reaches for. `checked_i16_div_i8_to_i8` models `idiv bl` — AX by BL
/// into an 8-bit AL — including the overflow the CPU would trap on.
///
/// `DS:0x0ADA` is the duration and `DS:0x0ADB` the current tick; both are already
/// in `re/labels.csv`. Cited here because this was settled ASM with no doc
/// (#141's queue).
pub fn step_ship_3d_interpolation_gate(
    gate: &mut Ship3dInterpolationGate,
    source: [u16; SHIP_3D_INTERPOLATION_WORDS],
    dest: [u16; SHIP_3D_INTERPOLATION_WORDS],
) -> Option<Ship3dInterpolationStep> {
    if gate.duration_ticks == gate.current_tick {
        return Some(Ship3dInterpolationStep::Complete);
    }

    if gate.duration_ticks == 0 {
        return None;
    }

    gate.current_tick = gate.current_tick.wrapping_add(1);
    let mut interpolated = [0u16; SHIP_3D_INTERPOLATION_WORDS];
    for index in 0..SHIP_3D_INTERPOLATION_WORDS {
        let delta = source[index].wrapping_sub(dest[index]) as i16;
        let quotient = checked_i16_div_i8_to_i8(delta, gate.duration_ticks as i8)?;
        let step = (quotient as i16).wrapping_mul(gate.current_tick as i8 as i16);
        interpolated[index] = dest[index].wrapping_add(step as u16);
    }
    Some(Ship3dInterpolationStep::Active(interpolated))
}

/// The unified list widget's box layout, `0x84A1..0x84C6` inside
/// `list_widget_layout_unified` (`0x8428`) — the same widget the OPTION menu, the
/// contact menu and the concept list all enter:
///
/// ```text
///   0x84A1  add dx,0x14                    width  = widest + 20
///   0x84A4  mov [si+4],dx
///   0x84A7  add bp,8                       height = rows*pitch + 8
///   0x84AA  mov [si+6],bp
///   0x84AD  shr dx,1 / sub dx,[0xac6] / neg dx     x = anchor - width/2
///   0x84B9  sub bp,0xc8 / neg bp / shr bp,1        y = (200 - height)/2
/// ```
///
/// `0xAC6` is the anchor the caller sets — `0x64` for the console box, `0xE1` for
/// the in-window concept list — and `0xC8` is 200, the screen height, so the box
/// is vertically centred by its own height and horizontally placed by the anchor.
///
/// This is the arithmetic `engine::choice_box_top_y` and the list menu's `x0`
/// implement, and the formula two independent captures agree with at two
/// different values (audit-fixes #112). Cited here because the function was
/// settled ASM with no doc (#141's queue).
pub fn layout_ship_3d_target_list(
    measured_label_widths: &[u16],
    center_x: u16,
    has_extra_entry: bool,
) -> Ship3dTargetListLayout {
    let mut max_label_width = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH
    } else {
        SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH
    };
    let mut height_accumulator = if has_extra_entry {
        SHIP_3D_TARGET_LAYOUT_EXTRA_HEIGHT
    } else {
        0
    };

    for width in measured_label_widths {
        if *width >= max_label_width {
            max_label_width = *width;
        }
        height_accumulator = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    let width = max_label_width.wrapping_add(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let height = height_accumulator.wrapping_add(SHIP_3D_TARGET_LAYOUT_HEIGHT_PADDING);
    let x = center_x.wrapping_sub(width >> 1);
    let y = SHIP_3D_TARGET_LAYOUT_SCREEN_HEIGHT.wrapping_sub(height) >> 1;

    Ship3dTargetListLayout {
        x,
        y,
        width,
        height,
        max_label_width,
        label_count: measured_label_widths.len(),
        has_extra_entry,
        selector_mode_return: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
    }
}

/// The list widget's row hit-test, `0x84F6..0x850C`:
///
/// ```text
///   0x84E6  add cx,4                       the row origin is box_y + 4
///   0x84F8  mov ax,[0xa2c] / sub ax,dx     dy = mouse_y - that origin
///   0x84FD  js 0x853B                      above the box: miss
///   0x84FF  sub bp,8 / cmp ax,bp / jge     below (height - 8): miss
///   0x8506  mov bl,0xb / div bl            row = dy / 11
///   0x850A  inc al                         ...+ 1, so rows are ONE-BASED
///   0x850C  mov [0x27c7],al                the hovered row
/// ```
///
/// Two details a reimplementation gets wrong by default. The bound is
/// `height - 8`, not the full height — the box's 8px of chrome (the same `add
/// bp,8` [`layout_ship_3d_target_list`] added) is not clickable. And the row is
/// ONE-BASED after the `inc al`, so `0` means "no row" rather than "the first
/// row"; `DS:0x27C7` holds that value and the draw compares against it.
///
/// The 4px inset at `0x84E6` is `SHIP_3D_TARGET_HIT_TEST_TOP_INSET`, and it is the
/// SAME origin the draw uses — hit and draw share it, which is what stops the
/// clickable band drifting from the drawn one.
///
/// `div bl` is an UNSIGNED byte divide, which is why the `js` above it matters:
/// a negative `dy` would otherwise divide as a large positive and land on a row.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn hit_test_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    mouse_x: u16,
    mouse_y: u16,
    activate: bool,
) -> Option<Ship3dTargetHitResult> {
    state.hover_row = 0;
    state.selected_row = 0;
    let mut inside = false;
    let mut activated = false;
    let mut play_select_sound = false;

    if signed_i16(mouse_x) >= signed_i16(layout.x) {
        let right = layout.x.wrapping_add(layout.width);
        if signed_i16(mouse_x) <= signed_i16(right) {
            let row_origin = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
            let row_offset = mouse_y.wrapping_sub(row_origin);
            if signed_i16(row_offset) >= 0 {
                let hit_height = layout
                    .height
                    .wrapping_sub(SHIP_3D_TARGET_HIT_TEST_BOTTOM_INSET);
                if signed_i16(row_offset) < signed_i16(hit_height) {
                    let row =
                        checked_u16_div_u8_to_u8(row_offset, SHIP_3D_TARGET_LAYOUT_ROW_STEP as u8)?
                            .wrapping_add(1);
                    state.hover_row = row;
                    inside = true;

                    if state.presentation_state != SHIP_3D_TARGET_HOVER_PRESENTATION_MODE {
                        state.presentation_state = 0;
                        state.requested_presentation_state = SHIP_3D_TARGET_HOVER_PRESENTATION_MODE;
                    }

                    if activate {
                        state.requested_presentation_state =
                            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE;
                        state.selected_row = row;
                        activated = true;
                        play_select_sound = true;
                    }
                }
            }
        }
    }

    if !inside && state.presentation_state != SHIP_3D_TARGET_IDLE_PRESENTATION_MODE {
        state.presentation_state = 0;
        state.requested_presentation_state = SHIP_3D_TARGET_IDLE_PRESENTATION_MODE;
    }

    let return_value = (state.selected_row as u8).wrapping_sub(1) as i8 as i16 as u16;
    Some(Ship3dTargetHitResult {
        inside,
        activated,
        hover_row: state.hover_row,
        selected_row: state.selected_row,
        return_value,
        play_select_sound,
    })
}

/// The nav-choice DISPATCHER, `nav_choice_dispatch` @`0x86F1`:
///
/// ```text
///   0x86F1  test byte [0x2793],8 / jne 0x8705   the HUD gate: bit 3 SET means
///                                              do nothing and return
///   0x86F8  dec bx / add bx,bx                  choice -> zero-based, then *2
///   0x86FB  test byte [0x2565],1                the phase bit, passed to the
///                                              handler rather than tested here
///   0x8700  call word cs:[bx+0xf29]             the per-row handler table
/// ```
///
/// The gate is INVERTED from the obvious reading: `jne` skips when bit 3 is set,
/// so the dispatcher runs only while that bit is CLEAR. `0x86A4` (the click)
/// ORs `0xC` into the same word — bits 2 and 3 — which is how a click arms the
/// surface and suppresses the dispatcher until it is handled.
///
/// `[0x2565]` is TESTED here but not branched on: the flags land in the handler,
/// which is why `run_ship_3d_nav_choice_handler_*` each begin by examining the
/// phase rather than being called only when it is set.
///
/// The table at `CS:0x0F29` is file `0x8709` (segment `0x071E`, base `0x77E0`) —
/// see `re/labels.csv` `nav_choice_subdispatch_table`, and audit-fixes #109/#129
/// for how it was decoded twice under two names.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn update_ship_3d_nav_choice_dispatch(
    state: &mut Ship3dNavChoiceState,
    gates: Ship3dNavChoiceGates,
    input: Ship3dNavChoiceInput,
) -> Option<Ship3dNavChoiceResult> {
    let mut result = Ship3dNavChoiceResult::default();
    if gates.blocks_nav_choice() {
        result.gated = true;
        return Some(result);
    }

    if state.selected_choice == 0 {
        if input.gate_value > SHIP_3D_NAV_CHOICE_MAX_GATE
            || input.gate_value < SHIP_3D_NAV_CHOICE_MIN_GATE
        {
            return Some(result);
        }

        result.reset_palette_range = true;
        if let Some(choice_index) =
            hit_test_ship_3d_nav_choice(input.dynamic_axis, input.mouse_x, input.mouse_y)?
        {
            let choice = choice_index.wrapping_add(1);
            result.hovered_choice = Some(choice);
            result.highlighted_palette_index =
                Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST.wrapping_add(choice_index));

            if input.activate {
                state.requested_presentation_state = SHIP_3D_NAV_CHOICE_PRESENTATION_MODE;
                state.selected_choice = choice as u16;
                state.hud_flags |= SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS;
                state.hold_ticks = SHIP_3D_NAV_CHOICE_HOLD_TICKS;
                state.handler_phase = SHIP_3D_NAV_CHOICE_HANDLER_PHASE;
                state.target_y = SHIP_3D_NAV_CHOICE_TARGET_Y_BASE.wrapping_add(
                    (choice as u16 - 1).wrapping_mul(SHIP_3D_NAV_CHOICE_TARGET_Y_STEP),
                );
                state.target_layout_preserve_widths = true;
                state.target_layout_center_x = SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X;
                state.target_layout_extra_entry = true;
                state.interpolation_duration_ticks = SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION;
                result.committed_choice = Some(choice);
                result.play_select_sound = Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND);
            }
        }
    }

    if state.selected_choice != 0 && state.hud_flags & SHIP_3D_NAV_CHOICE_DISPATCH_BLOCK_FLAG == 0 {
        let choice = u8::try_from(state.selected_choice).ok()?;
        if choice == 0 || choice > SHIP_3D_NAV_CHOICE_COUNT {
            return None;
        }
        result.dispatched_choice = Some(choice);
    }

    Some(result)
}

/// NAV-CHOICE HANDLER 0 — file `0x8713`, entry 0 of the dispatch table.
///
/// The dispatcher (`0x86F1..0x8704`) takes the committed choice `[0x2A19]`,
/// makes it 0-based and doubles it, then `call word cs:[bx+0xF29]` — a five-entry
/// near-offset table (`0F33 0F4C 0FDD 1068 108C`, CS base file `0x77E0`), so the
/// handlers live at file `0x8713`, `0x872C`, `0x87BD`, `0x8848`, `0x886C`.
///
/// ```text
///   0x8713  test byte [0x2565],1 / je      the handler PHASE bit
///   0x871A  ax = [0x6754]                  the built-in object `Honk`
///   0x871D  [0x676A] = ax                  the deferred record's related field
///   0x8720  [0x6768] = 0xC3                ...and its type
///   0x8726  [0x2565] = 0                   phase cleared
/// ```
pub fn run_ship_3d_nav_choice_handler_0(
    state: &mut Ship3dNavChoiceState,
    named_honk_object: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(named_honk_object),
        cleared_handler_phase: true,
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

/// NAV-CHOICE HANDLER 1 — file `0x872C` (`nav_choice_handler_1`), entry 1 of the
/// dispatch table described on [`run_ship_3d_nav_choice_handler_0`]. Verified
/// against the disassembly step by step:
///
/// ```text
///   0x8735  test byte [0x2565],1 / je 0x876A   the handler PHASE bit
///   0x8741  mov byte [0xADB],0                 reset the interpolation STEP
///   0x8748  walk the word list, `add ax,4` per entry until the 0xFFFF terminator
///   0x8758  mov byte [0x27E6],1                arm the widget's QUERY-ONLY flag
///   0x875E  call 0x8428                        the layout prepass
///   0x8761  mov byte [0x27E6],0                disarm
///   0x8766  inc byte [0x2565]                  advance the phase
///   0x876A  test byte [0x2565],2 / je          the INTERPOLATING phase
/// ```
///
/// `adjust_nav_choice_target_records` is the `add ax,4` loop; the `[0x27E6]`
/// bracket is the query-only flag whose early return was decoded independently at
/// `0x84CD`/`0x85D3` while establishing that the choice box is a tint — the two
/// readings corroborate each other.
pub fn run_ship_3d_nav_choice_handler_1(
    state: &mut Ship3dNavChoiceState,
    target_records: &mut [u16],
    interpolation_complete: bool,
    query_selection: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        state.interpolation_current_tick = 0;
        adjust_nav_choice_target_records(target_records);
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.adjusted_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_type = Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE);
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.load_snd_bank_path = Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET);
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

/// NAV-CHOICE HANDLER 2 — file `0x87BD`, entry 2 of the dispatch table
/// (see [`run_ship_3d_nav_choice_handler_0`]). Verified against the disassembly:
///
/// ```text
///   0x87BE  test byte [0x2565],1 / je 0x87FB   the handler PHASE bit
///   0x87C5  si=0x6D3E  di=0x2B13               special slots -> the MENU WORD LIST
///   0x87CB  lodsw / or ax,ax / je              SKIP zero slots
///   0x87D0  cmp ax,-1 / je 0x87DB              the 0xFFFF sentinel: store, stop
///   0x87D5  add ax,4 / stosw                   otherwise store RECORD+4
///   0x87E4  mov byte [0xADB],0                 reset the interpolation STEP
///   0x87E9  [0x27E6]=1 / call 0x8428 / =0      the query-only layout prepass
///   0x87F7  inc byte [0x2565]                  advance the phase
/// ```
///
/// Where handler 1 adjusts the EXISTING list in place, this one REBUILDS it from
/// WHAT `DS:0x2B13` IS, settled by its consumer rather than by its name: `0x87DF`
/// (immediately after this loop) does `mov si,0x2b13` and `call 0x8428` — the same
/// `list_widget_layout_unified` the OPTION menu enters with `si=0x2567`. That list
/// holds POINTERS TO NUL-TERMINATED STRINGS (`DS:0x2573` `TEXT`, `0x2581`
/// `MUSIC_OFF`, ...), so these entries are pointers too, and `record+4` is the
/// object's INLINE NAME — not a header skip. This doc previously called the
/// destination "target records", which described the wrong thing; `0x71E3` is a
/// second consumer, walking it with `cmp ax,-1` against `gs:[0x6754]`.
/// `VmMachine::ship_contact_menu_words` ports the same loop for the frontend.
///
/// the special-slot array at `DS:0x6D3E` — the 16-word block `0x53FF` clears with
/// `cx=0x10`, matching the port's `SPECIAL_OBJECT_SLOT_COUNT`.
pub fn run_ship_3d_nav_choice_handler_2(
    state: &mut Ship3dNavChoiceState,
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
    interpolation_complete: bool,
    query_selection: u16,
) -> Option<Ship3dNavChoiceHandlerEffect> {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        rebuild_nav_choice_special_target_records(special_slots, target_records)?;
        state.interpolation_current_tick = 0;
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        effect.ran_layout_prepass = true;
        effect.rebuilt_target_records = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return Some(effect);
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if query_selection == SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN {
        return Some(effect);
    }

    let target_index = usize::from(query_selection);
    let target_record = *target_records.get(target_index)?;
    if target_record != SHIP_3D_TARGET_EXIT_SENTINEL {
        effect.deferred_record_related =
            Some(target_record.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        effect.set_input_gate_b = true;
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    Some(effect)
}

/// NAV-CHOICE HANDLER 3 — file `0x8848`, entry 3 of the dispatch table
/// (see [`run_ship_3d_nav_choice_handler_0`]). The shortest of the five, verified
/// whole:
///
/// ```text
///   0x8848  test byte [0x2565],1 / je ret      the handler PHASE bit
///   0x884F  ax = [0x6756]                      the built-in object `menu`
///   0x8852  [0x676A] = ax                      the deferred record's related field
///   0x8855  [0x6768] = 0xC3                    ...and its type
///   0x885B  [0x2565] = 0                       phase cleared
///   0x8860  si=0xD16 ("sn\radio.snd") / ax=1 / lcall 0xB1B:0x855
/// ```
///
/// Structurally handler 0 with two differences: the related object is `menu`
/// (`gs:0x6756`) rather than `Honk` (`gs:0x6754`), and it reloads the radio sound
/// bank. `related_record` is that object, supplied by the caller.
pub fn run_ship_3d_nav_choice_handler_3(
    state: &mut Ship3dNavChoiceState,
    related_record: u16,
) -> Ship3dNavChoiceHandlerEffect {
    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE == 0 {
        return Ship3dNavChoiceHandlerEffect::default();
    }

    state.handler_phase = 0;
    Ship3dNavChoiceHandlerEffect {
        deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
        deferred_record_related: Some(related_record),
        cleared_handler_phase: true,
        load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
        ..Ship3dNavChoiceHandlerEffect::default()
    }
}

/// NAV-CHOICE HANDLER 4 — file `0x886C`, entry 4 of the dispatch table
/// (see [`run_ship_3d_nav_choice_handler_0`]). The largest of the five, and
/// verified end to end:
///
/// ```text
///   0x8874  test byte [0x2565],1 / je 0x889C   the handler PHASE bit
///   0x887B  mov byte [0xADB],0                 reset the interpolation STEP
///   0x8880  [0x27E6]=1 / call 0x8428           the query-only layout prepass
///   0x889C  test byte [0x2565],2 / je 0x88B9   the INTERPOLATING phase
///   0x88AA  lcall 0x8B:0xFAD / jae 0x8961      not complete -> exit
///   0x88B4  mov byte [0x2565],0                phase cleared
///   0x88BA  call 0x8428 / or ax,ax / js        a NEGATIVE selection exits
///   0x88C3  dec al / jns   -> sel 0: [0x259B]=1, [0x259C]=1
///   0x88D4  dec al / jns   -> sel 1: gated on [0xADE]&1, toggles [0xBA3]
///   0x8923  dec al / jns   -> sel 2: [0x2738]=1, [0x2736]=1   (left)
///   0x8933  dec al / jns   -> sel 3: [0x2738]=1, [0x2737]=1   (right)
///   0x8943  dec al / jns   -> sel 4: [0xB13]=2, [0xA3E]=0, [0xA40]=0
///   0x8956  [0x2A19]=0 / and byte [0x2793],0xFB    clear choice + HUD flag
/// ```
///
/// The selection dispatch is a CHAIN OF `dec al / jns`, not a table — each branch
/// tests whether the counter has gone negative yet, so the cases are consumed in
/// order. The port's `match` on the low byte reproduces it exactly, including the
/// fall-through for selections past 4.
pub fn run_ship_3d_nav_choice_handler_4(
    state: &mut Ship3dNavChoiceState,
    handler_state: &mut Ship3dNavChoiceHandler4State,
    layout_rect: [u16; SHIP_3D_INTERPOLATION_WORDS],
    interpolation_complete: bool,
    query_selection: u16,
) -> Ship3dNavChoiceHandlerEffect {
    let mut effect = Ship3dNavChoiceHandlerEffect::default();

    if state.handler_phase & SHIP_3D_NAV_CHOICE_HANDLER_PHASE != 0 {
        state.interpolation_current_tick = 0;
        state.handler_phase = state
            .handler_phase
            .wrapping_add(SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        handler_state.layout_rect_snapshot = layout_rect;
        effect.ran_layout_prepass = true;
        effect.copied_layout_rect_snapshot = true;
        effect.reset_interpolation_tick = true;
    }

    if state.handler_phase & SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING != 0 {
        if !interpolation_complete {
            effect.phase_gate_blocked = true;
            return effect;
        }
        state.handler_phase = 0;
        effect.cleared_handler_phase = true;
    }

    if signed_i16(query_selection) < 0 {
        return effect;
    }

    match query_selection.to_le_bytes()[0] {
        0 => {
            handler_state.menu_gate = true;
            handler_state.secondary_menu_gate = true;
        }
        1 => {
            if handler_state.voc_enabled {
                handler_state.voc_stream_phase = 0;
                if handler_state.tablo2_voc_active {
                    handler_state.tablo2_voc_active = false;
                    handler_state.active_target_list_offset =
                        SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET;
                } else {
                    handler_state.tablo2_voc_reset_gate = false;
                    handler_state.tablo2_voc_active = true;
                    handler_state.active_target_list_offset =
                        SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET;
                    effect.load_voc_path = Some(SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET);
                    effect.start_voc_playback = true;
                }
            }
        }
        2 => {
            handler_state.shared_motion_gate = true;
            handler_state.left_motion_gate = true;
        }
        3 => {
            handler_state.shared_motion_gate = true;
            handler_state.right_motion_gate = true;
        }
        4 => {
            handler_state.sound_gate = SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS;
            handler_state.target_activate_flag = false;
            handler_state.target_activate_secondary_flag = false;
        }
        _ => {}
    }

    state.selected_choice = 0;
    state.hud_flags &= !SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
    effect.cleared_selected_choice = true;
    effect.cleared_hud_target_list_flag = true;
    effect
}

pub fn run_ship_3d_navigation_sequence_update(
    state: &mut Ship3dNavigationSequenceState,
    presentation_active: bool,
    presentation_defer_active: bool,
    interpolation_complete: bool,
    query_selection: u16,
) -> Ship3dNavigationSequenceEffect {
    let mut effect = Ship3dNavigationSequenceEffect::default();

    let run_active_sequence = if state.exit_pending {
        if state.opening {
            true
        } else {
            effect.final_reset_pending = true;
            return effect;
        }
    } else if state.sequence_active {
        true
    } else {
        if !presentation_defer_active {
            state.exit_pending = true;
            state.opening = true;
            effect.armed_opening_exit = true;
        }
        return effect;
    };

    if !run_active_sequence {
        return effect;
    }

    effect.ran_temp_snd_setup = true;
    effect.ran_procedural_update = true;

    if presentation_active {
        effect.blocked_by_presentation_active = true;
        return effect;
    }

    state.framebuffer_dirty = true;
    effect.copied_framebuffer = true;

    if state.interpolation_duration_ticks != SHIP_3D_NAVIGATION_INTERPOLATION_DURATION {
        return effect;
    }

    if !interpolation_complete {
        effect.interpolation_active = true;
        return effect;
    }

    effect.queried_target_list = true;
    if signed_i16(query_selection) >= 0 {
        state.sequence_active = false;
        state.exit_pending = true;
        effect.armed_exit_pending = true;
    }

    effect
}

/// The panorama auto-turn, `0x9733..0x97FC` (audit-fixes #496). PARTIALLY
/// verified — this function had no citation at all, and now names its routine,
/// but only its CONSTANTS have been checked instruction by instruction:
/// `test word [0x2793],8` @`0x9733`, the shortest-distance fold
/// `cmp ax,0xb4 / sub ax,0x168 / neg ax` @`0x9748`, the `[0x2793]` bit-2 branch
/// @`0x975A`, and the cursor ring `shl bp,2 / add cx,0x5a0 / int 0x33` AX=4
/// @`0x9794`.
///
/// BOTH GAPS #496 LEFT OPEN ARE NOW CLOSED (audit-fixes #497). The frame/degree
/// mapping is not an inference: the routine works in DEGREES, wrapping at `0x168`,
/// and stores `shr bx,1` @`0x97E1` into the frame cell `[0x2795]`. So a frame IS
/// half a degree-count, and this function's `angle * 2` is exactly that inverse.
/// The two branch step sizes are named too — [`SHIP_3D_PROCEDURAL_TARGET_LIST_STEP`]
/// (`0x28` @`0x977C`) and [`SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP`] (`0x1E` @`0x97C4`).
///
/// Still NOT a whole-function transcription claim: every constant and the angle
/// mapping are checked, the statement-by-statement equivalence of the branch
/// bookkeeping is not.
pub fn run_ship_3d_procedural_update(
    state: &mut Ship3dProceduralUpdateState,
) -> Ship3dProceduralUpdateEffect {
    let mut effect = Ship3dProceduralUpdateEffect::default();
    let mut angle_double = state.angle.wrapping_mul(2);

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG != 0 {
        let target_angle = state.hold_ticks >> 1;
        if state.angle == target_angle {
            state.hud_flags ^= SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG;
            state.nav_timer = 0;
            effect.cleared_hud_active_flag = true;
        } else {
            let delta = circular_delta(state.angle, target_angle, SHIP_3D_PROCEDURAL_HALF_TURN);
            let compare_angle = wrap_ring_once(
                state.angle as i32 + delta as i32,
                SHIP_3D_PROCEDURAL_HALF_TURN,
            )
            .wrapping_mul(2);

            if state.nav_timer == 0 {
                state.nav_timer = delta;
                effect.initialized_nav_timer = true;
            }

            let mut angle_step = delta >> 1;
            if angle_step == 0 {
                angle_step = 1;
            }
            let mut mouse_step = delta.wrapping_shl(2);
            state.rotation_direction_positive = true;
            if compare_angle != state.hold_ticks {
                state.rotation_direction_positive = false;
                angle_step = angle_step.wrapping_neg();
                mouse_step = mouse_step.wrapping_neg();
            }

            if signed_i16(state.nav_timer) >= signed_i16(SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD) {
                state.mouse_x = state.mouse_x.wrapping_add(mouse_step);
                state.mouse_delta_accumulator =
                    state.mouse_delta_accumulator.wrapping_add(mouse_step);
            }

            state.angle = wrap_ring_once(
                state.angle as i32 + signed_i16(angle_step) as i32,
                SHIP_3D_PROCEDURAL_HALF_TURN,
            );
            state.mouse_button_state = 0;
            angle_double = state.angle.wrapping_mul(2);
            effect.applied_hud_rotation = true;
        }
    }

    state.mouse_x = wrap_ring_once(
        state.mouse_x as i32 - SHIP_3D_PROCEDURAL_MOUSE_RING as i32,
        SHIP_3D_PROCEDURAL_MOUSE_RING,
    );
    effect.mouse_set_position = Some((
        state
            .mouse_x
            .wrapping_add(SHIP_3D_PROCEDURAL_MOUSE_CENTER_X),
        state.mouse_y,
    ));
    state.mouse_sector = state.mouse_x >> 2;

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG == 0 {
        let delta = circular_delta(
            angle_double,
            state.mouse_sector,
            SHIP_3D_PROCEDURAL_FULL_TURN,
        );
        if delta > SHIP_3D_PROCEDURAL_CLOSE_ANGLE_THRESHOLD {
            if state.hud_flags & SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG != 0 {
                if delta >= SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD {
                    let mouse_plus_delta = wrap_ring_once(
                        state.mouse_sector as i32 + delta as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    );
                    let mut target_sector = angle_double;
                    if mouse_plus_delta == angle_double {
                        target_sector = wrap_ring_once(
                            target_sector as i32 - SHIP_3D_PROCEDURAL_TARGET_LIST_STEP as i32,
                            SHIP_3D_PROCEDURAL_FULL_TURN,
                        );
                    } else {
                        target_sector = wrap_ring_once(
                            target_sector as i32 + SHIP_3D_PROCEDURAL_TARGET_LIST_STEP as i32,
                            SHIP_3D_PROCEDURAL_FULL_TURN,
                        );
                    }
                    state.mouse_x = target_sector.wrapping_shl(2);
                    effect.mouse_set_position = Some((
                        state
                            .mouse_x
                            .wrapping_add(SHIP_3D_PROCEDURAL_MOUSE_CENTER_X),
                        state.mouse_y,
                    ));
                    effect.adjusted_target_list_mouse = true;
                }
            } else {
                let mouse_plus_delta = wrap_ring_once(
                    state.mouse_sector as i32 + delta as i32,
                    SHIP_3D_PROCEDURAL_FULL_TURN,
                );
                let next_sector = if mouse_plus_delta != angle_double {
                    state.rotation_direction_positive = true;
                    wrap_ring_once(
                        state.mouse_sector as i32 - SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    )
                } else {
                    state.rotation_direction_positive = false;
                    wrap_ring_once(
                        state.mouse_sector as i32 + SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP as i32,
                        SHIP_3D_PROCEDURAL_FULL_TURN,
                    )
                };
                state.angle = next_sector >> 1;
                effect.auto_rotated_angle = true;
            }
        }
    }

    if state.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG != 0 || effect.auto_rotated_angle {
        state.projection_angle = state.angle;
        state.rotation_offset = state
            .angle
            .wrapping_shl(3)
            .wrapping_sub(SHIP_3D_PROCEDURAL_ROTATION_OFFSET_BIAS);
        state.mouse_x &= SHIP_3D_PROCEDURAL_MOUSE_ALIGN_MASK;
        effect.updated_projection_angle = true;
        effect.carry_set = true;
    }

    state.mouse_x = wrap_ring_once(
        state.mouse_x as i32 - state.rotation_offset as i32,
        SHIP_3D_PROCEDURAL_MOUSE_RING,
    );

    effect
}

/// Build the 3x3 fixed-point projection matrix, `ship_3d_projection_matrix_build`
/// @`0x98B9`:
///
/// ```text
///   0x98CB  mov bp,0x4f45      the ANGLE TABLE -- SHIP_3D_ANGLE_TABLE's address
///   0x98CE  mov si,0x2f7d      the matrix scratch
///   0x98D1  mov di,[0x2f71]    the first angle word
/// ```
///
/// The result lands at `DS:0x2F95`, which `ship_3d_point_cloud_project`
/// (`0x9A10`) then runs the 1000 `DS:0x2FC1` records through. The three angle
/// fields are named for their globals — `angle_2f71`, `projection_angle_2f6d`,
/// `angle_2f6f` — because that is the only thing distinguishing them; the routine
/// reads three words and the port's names keep the correspondence checkable.
///
/// `matrix_pair_for_angle` doubles each table entry to Q15, matching `0x990C`'s
/// `movsx` + `add ebx,ebx` (see `render_nav_pyramid_sprites`).
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn build_ship_3d_projection_matrix(
    angle_table: &[Ship3dAngleTableEntry],
    angles: Ship3dMatrixAngles,
) -> Option<Ship3dProjectionMatrix> {
    let (a_cos, a_sin) = matrix_pair_for_angle(angle_table, angles.angle_2f71)?;
    let (b_cos, b_sin) = matrix_pair_for_angle(angle_table, angles.projection_angle_2f6d)?;
    let (c_cos, c_sin) = matrix_pair_for_angle(angle_table, angles.angle_2f6f)?;

    let b_sin_c_sin = fixed_mul_shift_15(b_sin, c_sin);
    let c_sin_b_cos = fixed_mul_shift_15(c_sin, b_cos);

    Some(Ship3dProjectionMatrix {
        terms: [
            a_cos
                .wrapping_mul(b_cos)
                .wrapping_add(b_sin_c_sin.wrapping_mul(a_sin))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            // NEG BEFORE the >>15 (0x2FB1 `neg eax; sar eax,0xf`): (-P)>>15, not
            // -(P>>15). Arithmetic shift floors toward -inf, so the two differ by 1
            // when P isn't a multiple of 32768.
            c_cos.wrapping_mul(a_sin).wrapping_neg() >> SHIP_3D_MATRIX_FIXED_SHIFT,
            c_sin_b_cos
                .wrapping_mul(a_sin)
                .wrapping_sub(a_cos.wrapping_mul(b_sin))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            b_sin_c_sin
                .wrapping_mul(a_cos)
                .wrapping_sub(a_sin.wrapping_mul(b_cos))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            fixed_mul_shift_15(c_cos, a_cos).wrapping_neg(),
            b_sin
                .wrapping_mul(a_sin)
                .wrapping_add(c_sin_b_cos.wrapping_mul(a_cos))
                >> SHIP_3D_MATRIX_FIXED_SHIFT,
            fixed_mul_shift_15(b_sin, c_cos),
            c_sin,
            fixed_mul_shift_15(c_cos, b_cos),
        ],
    })
}

/// Project one point through the camera, from `ship_3d_point_cloud_project`
/// @`0x9A10` (the body at `0x9A30`):
///
/// ```text
///   0x9A31  mov bp,0x2f95              the matrix built at 0x98B9
///   0x9A34  lodsd / mov [di],eax       copy the point's words
///   0x9A3F  mov ax,[0x2f65] / sub [di],ax      translate by the camera ORIGIN
///   0x9A44  mov ax,[0x2f67] / sub [di+2],ax
///   0x9A4A  mov ax,[0x2f69] / sub [di+4],ax
///   0x9A50  movsx eax,[di] / imul eax,[bp+0x18]   DEPTH first: dot with the
///   0x9A5C  movsx eax,[di+2] / imul eax,[bp+0x1c]  matrix's third row
/// ```
///
/// The matrix is nine 32-bit terms, so `[bp+0x18]` is term 6 — the depth row is
/// `terms[6..=8]`, which is why the port's projection computes depth from those
/// three and not from the first row. The translate is a plain 16-bit `sub` before
/// any widening, so a point far from the origin WRAPS rather than saturating;
/// `movsx` then sign-extends whatever that left.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn project_ship_3d_point(
    point: Ship3dProjectionPoint,
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
) -> Option<Ship3dProjectedPoint> {
    let translated = [
        projection_component(point.x, origin.x),
        projection_component(point.y, origin.y),
        projection_component(point.z, origin.z),
    ];

    let depth = projection_dot(
        translated,
        [matrix.terms[6], matrix.terms[7], matrix.terms[8]],
    ) >> SHIP_3D_MATRIX_FIXED_SHIFT;
    if depth <= 0 {
        return None;
    }

    let screen_x = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[0], matrix.terms[1], matrix.terms[2]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_X,
    );
    let screen_y = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[3], matrix.terms[4], matrix.terms[5]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_Y,
    );

    Some(Ship3dProjectedPoint {
        x: screen_x,
        y: screen_y,
        depth: depth as u16,
    })
}

/// Plot one projected point, `ship_3d_plot_point` @`0x9B04`:
///
/// ```text
///   0x9B0A  cmp ax,[0x5235] / jl   reject left of the clip
///   0x9B10  cmp ax,[0x5237] / jge  reject at or past the right
///   0x9B19  cmp bx,[0x5239] / jl   the same for y against 0x5239/0x523B
///   0x9B30  mov al,es:[di] / or al,al / jne   FIRST WRITE WINS (#149)
/// ```
///
/// The bound tests are `jl`/`jge` — SIGNED — which is why the port compares
/// through `signed_i16` rather than on `u16`. A projected point behind the camera
/// arrives as a large unsigned value, and an unsigned compare would place it on
/// screen instead of rejecting it.
///
/// Note the asymmetry the game uses and the port keeps: `jl` on the low bound and
/// `jge` on the high, so the clip is half-open — `left` is inside, `right` is not.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn plot_ship_3d_projected_point(
    depth_buffer: &mut [u8],
    viewport: Ship3dProjectionViewport,
    projected: Ship3dProjectedPoint,
) -> Option<Ship3dProjectedPixel> {
    if signed_i16(projected.x) < signed_i16(viewport.left)
        || signed_i16(projected.x) >= signed_i16(viewport.right)
        || signed_i16(projected.y) < signed_i16(viewport.top)
        || signed_i16(projected.y) >= signed_i16(viewport.bottom)
    {
        return None;
    }

    let offset = ship_3d_projected_point_offset(projected);
    let pixel = depth_buffer.get_mut(offset)?;
    if *pixel != 0 {
        return None;
    }

    let shade = ship_3d_projected_point_shade(projected.depth);
    *pixel = shade;
    Some(Ship3dProjectedPixel { offset, shade })
}
// The write is GATED on the pixel being empty (`0x9B30 mov al,es:[di] / or al,al /
// jne 0x9B44`): a point only draws where nothing has drawn yet, so the starfield
// keeps the FIRST point at each position rather than the last. That is the whole
// depth model here -- there is no z-buffer, just this ordering rule.

/// Depth shade for a plotted point: `0xEF - (depth >> 12)`, from the plot at
/// `0x9B37`:
///
/// ```text
///   0x9B37  mov ax,[bp+0x28]   the projected DEPTH
///   0x9B3A  shr ax,0xc         >> 12
///   0x9B3D  neg al
///   0x9B3F  add al,0xef        0xEF - that
/// ```
///
/// `neg` then `add` rather than a subtract, so the arithmetic wraps in 8 bits
/// exactly as `wrapping_sub` does here. `SHIP_3D_PROJECTION_SHADE_BASE` is that
/// `0xEF` (239) and `SHADE_SHIFT` that `0xC`.
pub fn ship_3d_projected_point_shade(depth: u16) -> u8 {
    SHIP_3D_PROJECTION_SHADE_BASE.wrapping_sub((depth >> SHIP_3D_PROJECTION_SHADE_SHIFT) as u8)
}

/// Framebuffer offset for a projected point — `y*320 + x`, built by the plot at
/// `0x9B25` without a multiply:
///
/// ```text
///   0x9B25  mov di,bx      di = y
///   0x9B27  xchg bh,bl     bx = y << 8   (y * 256)
///   0x9B29  shl di,6       di = y * 64
///   0x9B2C  add di,bx      y*64 + y*256 = y * 320
///   0x9B2E  add di,ax      + x
/// ```
pub fn ship_3d_projected_point_offset(projected: Ship3dProjectedPoint) -> usize {
    usize::from(projected.y) * SHIP_3D_PROJECTION_SCREEN_WIDTH + usize::from(projected.x)
}

/// Number of 3D point-cloud records the starfield background is built from:
/// `ship_3d_point_cloud_randomize` (`0x9B67`) does `mov cx,0x3E8` (`0x9B6A`) and
/// `mov di,0x2FC1` (`0x9B71`), then fills each record with three
/// `rng(0xFFFF)` words. Checked against the image by
/// `the_point_cloud_length_is_the_randomizers_own_immediate`.
pub const SHIP_3D_POINT_CLOUD_LEN: usize = 1000;
/// File offset of that `mov cx,imm`'s operand — `mov cx,0x3e8` sits at `0x9B6A`
/// (`b9 e8 03`), so the count word is at `0x9B6B`.
pub const SHIP_3D_POINT_CLOUD_COUNT_IMMEDIATE: usize = 0x9B6B;
/// ...and of the `mov di,imm` naming the record base (`DS:0x2FC1`).
pub const SHIP_3D_POINT_CLOUD_BASE_IMMEDIATE: usize = 0x9B72;
/// The record base itself, the operand of `mov di,0x2fc1` @`0x9B71`.
pub const SHIP_3D_POINT_CLOUD_BASE_DS: u16 = 0x2FC1;

/// The engine's pseudo-random generator (`far 0x01CE:0x0B02` in BLOODPRG.EXE).
///
/// Called as `rng(ax = modulus)` and returns `value % modulus` (for
/// `modulus == 0` it returns the raw 16-bit value). The generator threads a
/// carry chain through two state bytes to build a 16-bit word, XORs it with a
/// fixed 16-bit seed word, then advances the two bytes via an incrementing
/// counter. State lives at `cs:0x0AEE` (`seed_word`), `cs:0x0AF0` (`a`),
/// `cs:0x0AF1` (`b`), `cs:0x0AF2` (`counter`); all are zero in the shipped
/// image (the fields below default to that), but the startup code seeds them,
/// so a live run's sequence is not reproducible from the static image alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BloodPrng {
    /// `cs:0x0AEE` — XORed into each result; never mutated by the generator.
    pub seed_word: u16,
    /// `cs:0x0AF0` — low mixing byte, advanced each call.
    pub a: u8,
    /// `cs:0x0AF1` — high mixing byte, advanced each call.
    pub b: u8,
    /// `cs:0x0AF2` — call counter used to advance `a`/`b`.
    pub counter: u8,
}

impl Default for BloodPrng {
    /// The static (unseeded) state from the shipped BLOODPRG.EXE image — VERIFIED
    /// rather than assumed (audit-fixes #276), because #275 found a `Default` whose
    /// zero was a guess about a field the routine it cited never wrote.
    ///
    /// The five state bytes live at `cs:[0xAEE..0xAF2]` with `cs = 0x1CE`
    /// (base file `0x22E0`), so file `0x2DCE`, and the image holds
    /// `00 00 00 00 00` there. Zero here is the game's value, not an absence of
    /// one. `prng_state_is_zero_in_the_shipped_image` reads it back.
    fn default() -> Self {
        Self {
            seed_word: 0,
            a: 0,
            b: 0,
            counter: 0,
        }
    }
}

impl BloodPrng {
    /// Seed as the DOS routine at `0x2DD3` does: it reads the CMOS RTC seconds
    /// byte (`out 0x70 / in 0x71`) and writes it into both halves of the XOR
    /// seed word (`mov ah,al; mov cs:[0xAEE],ax`), leaving the mixing bytes and
    /// counter at zero. Passing the boot second reproduces that run's stream.
    pub fn seeded_from_rtc_seconds(seconds: u8) -> Self {
        Self {
            seed_word: u16::from(seconds) * 0x0101,
            a: 0,
            b: 0,
            counter: 0,
        }
    }

    /// Advance the generator and return the next value in `0..modulus` (or the raw 16-bit word
    /// when `modulus == 0`). Faithful port of the generator at `0x01CE:0x0B02`.
    pub fn next(&mut self, modulus: u16) -> u16 {
        // Thread a carry through the two mixing bytes to build a 16-bit word: each of the eight
        // rounds rotates one bit out of the low byte and one out of the high byte, folding two
        // bits into the word. The chain starts with a cleared carry.
        let mut low = self.a;
        let mut high = self.b;
        let mut word: u16 = 0;
        let mut carry: u16 = 0;
        for _ in 0..8 {
            let next_carry = u16::from(low & 1);
            low = ((carry as u8) << 7) | (low >> 1);
            carry = next_carry;

            let next_carry = word >> 15;
            word = (word << 1) | carry;
            carry = next_carry;

            let next_carry = u16::from(high >> 7);
            high = (high << 1) | (carry as u8);
            carry = next_carry;

            let next_carry = word >> 15;
            word = (word << 1) | carry;
            carry = next_carry;
        }

        word ^= self.seed_word;

        // Advance the two mixing bytes from the incrementing counter.
        self.counter = self.counter.wrapping_add(1);
        let step = self.counter;
        self.b = self.b.wrapping_sub(step);
        self.a ^= step.rotate_left(1);

        // Range-reduce into `0..modulus` by repeated subtraction, as the original does.
        if modulus == 0 {
            return word;
        }
        let mut reduced = word;
        while reduced >= modulus {
            reduced = reduced.wrapping_sub(modulus);
        }
        reduced
    }
}

/// Populate the ship-3D starfield point cloud (`ship_3d_point_cloud_randomize`
/// at `0x9B67`). Each of the [`SHIP_3D_POINT_CLOUD_LEN`] records gets random
/// `x`/`y`/`z` words from [`BloodPrng::next`] with modulus `0xFFFF`; the DOS
/// loop `add di,2` after the three `stosw`s leaves each record's fourth word
/// untouched, which the projection scratch reuses per frame.
pub fn randomize_ship_3d_point_cloud(prng: &mut BloodPrng) -> Vec<Ship3dProjectionPoint> {
    (0..SHIP_3D_POINT_CLOUD_LEN)
        .map(|_| Ship3dProjectionPoint {
            x: prng.next(0xffff),
            y: prng.next(0xffff),
            z: prng.next(0xffff),
        })
        .collect()
}

/// Height of the ship-3D point-cloud depth/color buffer in rows. The DOS pixel
/// helper computes `y * 320 + x` into the active page; 200 native rows cover it.
/// 200 rows, and the binary says so where it RESTORES the clip: `mov word
/// [0x523b],0xc8` @`0xB41D` (#495). Previously this was justified as "200 native
/// rows cover it", which is a reason rather than evidence (audit-fixes #510).
pub const SHIP_3D_PROJECTION_SCREEN_HEIGHT: usize = 200;

/// One plotted starfield pixel and, alongside the returned count, the whole
/// depth-shaded buffer produced by [`render_ship_3d_point_cloud`].
/// Parameter/result shape for [`randomize_ship_3d_point_cloud`].
///
/// The fields ARE that routine's decoded values; this type carries no rule
/// of its own, so the binary citations live on the function rather than being
/// restated here. Recorded because an undocumented struct beside a cited
/// function reads as unexamined when it is simply the function's shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ship3dPointCloudRender {
    /// `320 * 200` depth-shaded buffer; `0` means "no point drawn here".
    pub buffer: Vec<u8>,
    /// Number of points that projected in front of the camera and won their
    /// depth-buffer cell (matches the DOS write-once behavior).
    pub plotted: usize,
}

/// Render the full ship-3D starfield background: the batch loop at `0x9A10`,
/// checked against the disassembly:
///
/// ```text
///   0x9A1D  mov word [0x2F77],0x3E8    the loop count — 1000, the cloud's length
///   0x9A23  mov si,0x2FC1              the point-cloud records
///   0x9A31  mov bp,0x2F95              the projection matrix
///   0x9A2D  mov es,[0x5223]            the render target
///   0x9A34  copy 8 bytes of the record into the work slot at 0x4F01
///   0x9A42  sub word [di],ax           translate by the camera ORIGIN (loaded
///   0x9A47  sub word [di+2],ax         from 0x2F65/0x2F67 at 0x9A3F/0x9A44)
/// ```
///
/// The translation is a SUBTRACTION, which `projection_component`'s
/// `point.wrapping_sub(origin)` reproduces — and `DS:0x2F65` is the same camera
/// origin the nav renderer cites as `(10000, 12000, 0)`. Each translated point
/// then goes through [`project_ship_3d_point`] and
/// [`plot_ship_3d_projected_point`], skipping non-positive depth and cells a
/// nearer point already claimed (the DOS helper only writes empty pixels).
pub fn render_ship_3d_point_cloud(
    points: &[Ship3dProjectionPoint],
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    viewport: Ship3dProjectionViewport,
) -> Ship3dPointCloudRender {
    let mut buffer = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
    let mut plotted = 0usize;
    for &point in points {
        let Some(projected) = project_ship_3d_point(point, origin, matrix) else {
            continue;
        };
        if plot_ship_3d_projected_point(&mut buffer, viewport, projected).is_some() {
            plotted += 1;
        }
    }
    Ship3dPointCloudRender { buffer, plotted }
}

/// The same starfield as [`render_ship_3d_point_cloud`], but returning the plotted
/// points (screen x, y, depth shade) for a GPU point renderer — identical projection,
/// viewport clip, depth-shade, and write-once cell rule.
/// The starfield as a POINT LIST rather than a rendered buffer — the same
/// projection and plot the game runs (`ship_3d_point_cloud_project` @`0x9A10`
/// into the plot at `0x9B04`), reporting the points that survived instead of the
/// pixels they wrote.
///
/// It still allocates and writes the buffer, because it must: the plot's
/// first-write-wins gate (`mov al,es:[di]` / `or al,al` / `jne` @`0x9B30`, see
/// #149) means whether a point is emitted depends on what earlier points already
/// wrote. Testing coordinates alone and skipping the buffer would emit points the
/// game discards, and the difference only shows where the field is dense.
///
/// Used by the GPU path, which draws these points at window resolution instead of
/// into the 320x200 framebuffer.
pub fn ship_3d_point_cloud_points(
    points: &[Ship3dProjectionPoint],
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    viewport: Ship3dProjectionViewport,
) -> Vec<(u16, u16, u8)> {
    let mut buffer = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
    let mut out = Vec::new();
    for &point in points {
        let Some(projected) = project_ship_3d_point(point, origin, matrix) else {
            continue;
        };
        if plot_ship_3d_projected_point(&mut buffer, viewport, projected).is_some() {
            let shade = ship_3d_projected_point_shade(projected.depth);
            out.push((projected.x, projected.y, shade));
        }
    }
    out
}

/// The ship-nav HUD band occupies the bottom rows of the 320x200 frame (below the
/// scene band that ends at row 0xA5=165), where the engine draws the grey
/// pyramid-nav grid + central eye-orb. See re/REVERSE.md.
/// 165 — the same `0xA5` the navigation routine writes as the clip bottom,
/// `mov word [0x523b],0xa5` @`0xB40D` (#495). The scene band ENDS where the HUD
/// band BEGINS, so one row boundary is written into one cell and read under two
/// names; it is not two independently chosen values (audit-fixes #510).
pub const SHIP_3D_HUD_BAND_TOP: usize = 165; // 165

/// The recovered ship-nav HUD pyramid geometry: 32 3D vertices (X,Y,Z, signed
/// fixed-point) copied by `ship_3d_hud_init` (BLOODPRG.EXE @0xB079) from DS:0x5D98
/// (file 0x131B8) into the HUD working area at ship-view entry, then projected by
/// the shared matrix×vector + perspective pipeline. Vertices 16..23 form a linear
/// compass axis; the rest are the pyramid/HUD corners.
///
/// Disassembly-recovered render path (sess 005), the missing transform + draw:
/// - `ship_3d_hud_init` @0xB079: `rep movsd` 0x30=48 dwords (32 verts × 3 words =
///   96 words) from `si=0x5D98` to `di=0x5491` (working copy); then `[0x2795]=0xB3`
///   (the compass *entry angle* — this is the projection angle to use, NOT 0),
///   `[0x279B]=0`, and `[0x2793] |= 8` (HUD gate bit 3).
/// - The compass angle `[0x2795]` animates 0..0xB3 (wraps at 0xB4=180) in
///   `ship_3d_procedural_angle_update` @0x9656.
/// - HUD draw prelude @0xB14A: re-copies 0x10 dwords from `0x5491` into the frame
///   working area `0x5551`, sets the band bounds `[0x5239]=0x23` (35) and
///   `[0x523B]=0xA5` (165) → **the HUD occupies the y=165..200 band (35px)**, then
///   renders via `lcall 0x1CE:0` (the projection/raster segment). So the ship3d HUD
///   is the COMPACT dialogue-mode nav strip; the full-screen star-map nav screen
///   (rows of shaded pyramids) is a SEPARATE view — don't conflate them.
/// - `0x1CE:0` (file 0x22E0) is a `/100` fixed-point perspective helper (called with
///   ax=-50, di=0x5F11 workspace). The pyramid render then dispatches through segment
///   0x299: `lcall 0x299:0x1467` and `lcall 0x299:0x210D` (after `ship_3d_target_
///   record_select` @0xB2BB selects the active target). So the vertex→screen raster
///   lives in seg 0x299; `di=0x6612`/`0x6724` are its record pointers.
/// - `0x299:0x1467` (file 0x43F9) iterates **32-byte records** at `si=0x6212`
///   (indexes by `ax<<5`), emitting to the `di=0x6612` draw list (dword pairs from
///   `[0x5235]`/`[0x5239]` + a 0xFFFF terminator). So the projected pyramid geometry
///   is a 32-byte-record display list; `0x299:0x210D` consumes/rasterises it.
/// - `0x299:0x210D` (file 0x509D) is the **rasteriser**: gated by `gs:[0x5231]`, it
///   walks the display list reading 8-byte segment records (`es:[di]`, `[di+2]`,
///   `[di+4]`, `[di+6]` = endpoints; `di += 8`), computes framebuffer offsets against
///   width 0x140=320, and draws the pyramid edges/spans into `gs:[0x5221]`.
///
/// MORE RECORD FIELDS (sess 005): the 0x6212 32-byte record also has coord fields at
/// `[0x18]/[0x1A]/[0x1C]/[0x1E]`, written by a far-call setter @0x299:0x13FB (file
/// 0x438D) from bx/cx/dx/bp; sibling setters @0x4374 toggle flag bits. So the record
/// holds {flags@0, coords@8/0xC, prev@0x10/0x14, more@0x18-0x1E}. The actual PROJECTION
/// is the routine that computes those coords and far-calls these setters — the next
/// decode target (find callers of 0x299:0x13FB). NOTE: the seg-0x299 far calls are
/// RELOCATED (segment patched at load), so byte-searching for `9A fb 13 99 02` finds
/// nothing — the caller must be found by CODE-FLOW tracing from the HUD update
/// (0xB1D0 calls 0x299:0x1467/0x210D; the projection is invoked in/around there),
/// using dis.py's reloc resolution rather than a raw byte grep.
/// RENDER SIDE TRACED (sess 005): the record renderer @0x299:~0x14E1 (file 0x4473)
/// iterates the 0x6612 records and dispatches each to a draw routine via a flag-indexed
/// jump table at `cs:[((flags>>1)&0xE)+0x1592]` — i.e. the pyramids are drawn by a
/// per-record dispatched draw op (sprite blit / fill / line) selected by the record
/// flags. So the whole RENDER side (build list → flag-dispatch draw → segment raster)
/// is mapped; only the upstream PROJECTION that writes the record coords remains.
/// DRAW MODEL CONFIRMED (sess 005): the dispatch table (cs:0x1592, file 0x4524) points
/// at 8 draw ops; the sprite dispatch (0x83>>1&7 = 1) → entry 1 @0x4BAA is a SPRITE
/// BLIT: it reads the record's projected coords `[di+0x1C]/[di+0x1E]` and a sprite
/// source pointer `[di+4]` and blits. So the pyramids are SPRITE INSTANCES at projected
/// positions (entries 4-6 = 0x210A-C near the segment rasteriser 0x210D). The
/// PROJECTION is therefore precisely the routine that computes bx/cx/dx/bp and far-calls
/// the setter 0x299:0x13FB (writes `[di+0x18..0x1E]` = the coords the blit reads).
///
/// *** PROJECTION LOCATED (sess 005) *** — the per-vertex loop @0x9BBA:
///   for each vertex: copy to working (di), record = 0x6212+((idx+0x15)<<5);
///   translated = vertex − ORIGIN (ORIGIN = DS:0x2F65/0x2F67/0x2F69, the camera pos —
///     NOT the 0x5F11 I earlier guessed);
///   depth = Σ translated_i · matrix_row3(bp+0x18/1C/20) >> 0xF;  (cull if depth<=0)
///   persp = 0x8000000>>7 / depth = 0x100000/depth  (fixed-point 1/z);
///   screen_x = Σ translated_i · matrix_row1(bp+0/4/8) · persp;  (y via row2);
///   then sub sprite half-size ([si+0xC/0xE]>>1) to centre, and far-call the seg-0x299
///   draw ops (0x299:0x133D / 0x127D).
/// This is EXACTLY [`project_ship_3d_point`] (matrix on the bp-stack is the 0x2F95
/// build = [`build_ship_3d_projection_matrix`]). So the star-map is now FULLY decoded:
/// reimplement by projecting the 32 verts with build_ship_3d_projection_matrix(HUD
/// angle 0xB3) + origin=[0x2F65/67/69], then blit the pyramid sprite at each. The one
/// remaining runtime value is the origin [0x2F65/67/69] (camera position).
/// ORIGIN IS DYNAMIC: [0x2F65] is reset to the immediate (0x2710,0x2EE0,0)=(10000,
/// 12000,0) @0x8AFE/0x8CB4 but updated per-frame @0x8A85 (`mov [0x2F65],ax`) from a
/// computed camera position. Projecting the 32 verts with the existing projector +
/// the STATIC base origin gives a scatter, not the symmetric grid (confirmed) — so
/// exact-grid repro needs the RUNTIME camera origin (trace the 0x8A85 source, i.e. the
/// ship-position update, or dump [0x2F65/67/69] from a live savestate). Everything else
/// (algorithm, matrix, perspective, draw) is decoded.
/// CORRECTION: the 0x9BBA loop is STRUCTURALLY like [`project_ship_3d_point`] but NOT
/// bit-exact — its perspective/scale differs: `persp = 0x8000000>>7 / depth =
/// 0x100000/depth`; then the screen coord is `(dot(row) >> 7) idiv <divisor> + 0x64`
/// (center 0x64=100). Projecting the verts with the existing projector does NOT
/// reproduce the grid (confirmed: ≤8/32 with the base origin/angle), so a bit-exact
/// render must TRANSCRIBE the exact 0x9BBA math (shifts 0xF/7, div 0x100000, center
/// 0x64) AND use the runtime animated origin (0x2F65 dec-by-100) + angle (0x2F71).
/// EXACT FORMULA (transcribed from 0x9BBA-0x9CAA; matrix rows are i32 dwords on the
/// bp-stack = the 0x2F95 build; t = vertex − origin):
///   depth  = (t·row_z) >> 15;          // row_z = m[6..8]; cull if 0; if <0, += 0x10000
///   sprite_scale = 0x100000 / depth;    // used to scale the blit sprite
///   screen_x = ((t·row_x) >> 7) / depth + 0xA0;   // row_x = m[0..2]; center 160
///   screen_y = ((t·row_y) >> 7) / depth + 0x64;   // row_y = m[3..5]; center 100
/// Transcribed EXACTLY, sweeping origin_x + angle_2F71 still only lands ≤9/32 verts in
/// a scatter — so the grid needs the FULL runtime camera frame: origin_x (0x2F65,
/// animated) AND all three matrix angles 0x2F71/0x2F6D/0x2F6F (I assumed 6D/6F=0; verify
/// their runtime values). Bit-exact render = this formula + the live camera state
/// (savestate/memory dump of 0x2F65/67/69 + 0x2F6D/6F/71, or trace their per-frame set).
/// ALL PARAMS NOW IDENTIFIED: angle_2f6d = the COMPASS angle DS:0x2795 (=0xB3 at entry,
/// set @0x97EA `[0x2F6D]=[0x2795]`); angle_2f71 = the CAMERA angle (animated); angle_2f6f
/// = 0 (never written); origin = (0x2F65 anim, 0x2F67=12000, 0x2F69=0). Brute-forcing
/// "verts on-screen" finds DEGENERATE diagonal configs (compass-axis verts 16-23 collapse
/// to a line), NOT the grid — so the grid needs the EXACT runtime frame (origin_x +
/// angle_2f71 at a specific animation frame): savestate/memory dump, or image-fit vs the
/// real char_7 grid. Formula + matrix + all param sources are decoded.
/// FINDING (sess 005): rendering the 32 verts with the EXACT formula + the INIT camera
/// (origin=(10000,12000,0), a71=0, a6d init) gives a DIAGONAL scatter (compass-axis verts
/// 16-23 form a line, pyramid verts cluster), NOT the symmetric grid — at any tested
/// angle. So the 32 verts are almost certainly a small TEMPLATE (a few pyramids + the
/// compass axis) that the render INSTANCES/tiles across the nav field (the record loop
/// @0x9BBA iterates `[0x2F77]` records, likely > 32, reusing the template with per-
/// instance offsets). Bit-exact grid therefore needs the INSTANCING (how many records,
/// their per-instance origin/offset) on top of the decoded per-vertex projection.
/// COUNT FOUND: `[0x2F77]` (the 0x9BBA loop count) = 11 (set @0x9BB4 right before the HUD
/// loop) — the nav view projects **11 records** (destination pyramids + compass elements),
/// NOT the 32 template verts. The SAME 0x9BBA loop also projects the 1000-point STARFIELD
/// (`[0x2F77]=1000` @0x9A1D). So the nav grid = 11 projected records from a source array
/// (bx, +6/rec). Bit-exact grid = those 11 source positions (nav destination data) + the
/// decoded projection + runtime camera. Projection done; the 11-record source is the piece.
/// SOURCE FOUND + IT'S RUNTIME DATA: the loop reads from `bx=DS:0x4F09` (dest 0x4F01,
/// matrix bp=0x2F95), 6 bytes/record. The STATIC bytes there are all (10200,12100,900)
/// (a default placeholder) — so the 11 real destination positions are POPULATED AT
/// RUNTIME from the nav state. CONCLUSION: the star-map is fully decoded at the code
/// level (projection formula + matrix + params + 11-record loop + sprite-blit draw); a
/// bit-exact render needs only RUNTIME STATE — the 11 live destination positions
/// (DS:0x4F09) + camera (origin 0x2F65/67/69, angles 0x2F71/6D) — obtainable via a
/// DOSBox-X savestate/memory dump at the nav view, not from the static binary.
///
/// PIPELINE NOW MAPPED END-TO-END (routine level): hud_init (verts→0x5491, angle
/// 0xB3) → prelude (band y165-200) → 0x1CE:0 (/100 perspective) → 0x299:0x1467
/// (32-byte-record display list @0x6212→0x6612) → 0x299:0x210D (8-byte-segment
/// rasteriser). The `0x1CE:0`/`0x22E0` transform reads a rotation matrix from
/// `0x5251` (byte components via `lodsb`/`cwde`), applies `/100` fixed-point scaling
/// with per-axis scale (`[bp+0x10]`) + offset (`[bp+0x12/14/16]`) params, emitting
/// projected coords — i.e. matrix×vector then perspective, same shape as
/// [`project_ship_3d_point`] but with the HUD's own 0x5251 matrix + 0x5F11 origin.
/// 32-BYTE RECORD LAYOUT (partly decoded from the 0x43F9 loop, stride 0x20):
///   [0] = flags byte (bits 0+1 both set → the record draws); [8],[0xC] = current
///   projected coord dwords; [0x10],[0x14] = previous coords (the loop copies 8→0x10,
///   0xC→0x14 each pass, so the rasteriser's 8-byte segment = prev→cur endpoints).
///   The 8-byte rasteriser records (0x509D) are the {cur,prev} endpoint pairs.
/// MATRIX CONFIRMED: `ship_3d_projection_matrix_build` @0x98B9 builds the 3×3 matrix
/// at DS:0x2F95 from the angle table + angle words 0x2F71/0x2F6D/0x2F6F — i.e. this is
/// exactly [`build_ship_3d_projection_matrix`] (same angle fields). 0x5251 is then a
/// working copy (`rep movsd 0xC0` from 0x5B58). So the rotation half of the HUD
/// projection is ALREADY implemented.
///
/// CORRECTION (was wrong before): `0x1CE:0`/`0x22E0` is NOT the perspective transform.
/// Full decode shows it computes squared distances `Σ(a_i-b_i)²` over records and
/// tracks the minimum — a NEAREST-POINT / hit-test search (which pyramid the cursor is
/// closest to), not a projection.
///
/// WITHDRAWN (audit-fixes #448). This paragraph used to continue: "the actual
/// vertex→screen PROJECTION for the pyramids is still unlocated ... TODO: find the
/// routine that projects the 0x5491 verts into the 0x6212 display-list records
/// (that IS the missing projection)". There is no such routine. The `0x6212`
/// records hold SPRITE-FRAME POINTERS, not projected coordinates — see the
/// `entity_object_populate` trace below, which follows the pointer the builder
/// actually writes and lands in a shipped `.SPR` bank.
///
/// The sentence stayed for five audit entries (#443–#447) and sent each of them
/// hunting a routine that does not exist. It is removed rather than annotated,
/// because a doc that states both "find the projection" and "there is no
/// projection" is worse than either. What IS still open: where the pyramid
/// POSITIONS come from, and the compass→matrix-angle map.
///
/// FURTHER (sess 005): the 0x6212-record builder @0x40D0 (seg 0x299) writes
/// `((flags & 4) | 0x83)` into each record — that is the SPRITE bank dispatch (same
/// formula as `sprite::bank_dispatch_index`). So the 0x6212 records carry sprite-draw
/// dispatch: the HUD pyramids are very likely SPRITES drawn at projected positions,
/// not a pure 3D wireframe. This reframes the render as hybrid (3D-projected placement
/// + sprite blit) and is why single-routine estimates kept being wrong. Genuinely
/// multi-session: needs the projection→position math AND the pyramid sprite source.
///
/// NARROWED (audit-fixes #444), without closing it. `0x5491` has exactly TWO
/// immediate loads in the image — `mov di, 0x5491` @`0xB09D` and
/// `mov si, 0x5491` @`0xB166` — and BOTH are `rep movsd` block copies of `0x10`
/// dwords (64 bytes), not vertex reads. So neither is the projection; at those
/// two sites `0x5491` is a 64-byte copy buffer.
///
/// And `0x5491` is `live_palette` (`DS:0x5251`) + `0x240` = palette entry 192 —
/// the palette/vertex alias resolved in commit `bd930b8`, confirmed here from the
/// arithmetic rather than from the earlier byte comparison. The verts and DAC
/// colours 192..255 genuinely are the same storage.
///
/// `0x6212` has 19 immediate loads, all but two clustered in `0x40D0..0x44A2` —
/// the entity-flag accessor family — plus `0x90D9` and `0x9241`.
///
/// THOSE TWO ARE READERS, NOT THE WRITER, and reading them changes the search.
/// Both open identically:
///
/// ```text
///   0x90D9  mov si, 0x6212 / les di, ptr [si + 4] / mov ax, word ptr es:[di]
///   0x9241  mov si, 0x6212 / les di, ptr [si + 4] / mov ax, word ptr es:[di]
/// ```
///
/// `[0x6212 + 4]` IS A FAR POINTER, and the record DATA lives behind it — these
/// routines only follow it and scale what they find (`mul 0xE` / `shr 5` at
/// `0x90E4`; `3 * [0x2789]` at `0x924B`, the same scale cell the location-info
/// panel uses at `0x90FF`).
///
/// So the projection writes THROUGH that far pointer and need never mention
/// `0x6212` at all — which is why enumerating immediate loads of `0x6212` cannot
/// find it. All 19 are now accounted for (17 flag accessors + these 2 readers).
///
/// WHO WRITES THE POINTER (audit-fixes #446): `entity_object_populate` @`0x40D0`,
/// and what it points AT is not projection output:
///
/// ```text
///   0x40D7  mov di, 0x6212 / shl ax,5 / add di,ax   record = 0x6212 + i*32
///   0x40EF  mov ax, [si] / and ax,4 / or al,0x83    the SPRITE bank dispatch
///   0x40F6  mov word ptr gs:[di], ax                 ...into record+0
///   0x40F9  add si, 4                                skip the directory header
///   0x40FF  mov ebp, dword ptr ds:[bp + si]          a PACKED DWORD entry
///   0x4105  and ax, 0xf / add si, ax                 low nibble advances si
///   0x410A  shr ebp, 4                               the rest is the payload
///   0x4114  mov word ptr gs:[di + 6], ax             record+6 = segment (ds)
///   0x4118  mov word ptr gs:[di + 4], si             record+4 = offset
/// ```
///
/// `si` walks a RESOURCE SUBOBJECT DIRECTORY (the label at `0x40F9`), so the
/// record's far pointer aims into LOADED RESOURCE DATA. That undercuts the premise
/// this TODO was written on: the coordinates behind these records may be AUTHORED
/// data the resource supplies, not the output of a projection routine — which
/// would also explain the `or al, 0x83` sprite dispatch sitting right beside it,
/// and why "single-routine estimates kept being wrong".
///
/// THE PACKED DWORD IS A 20-BIT LINEAR OFFSET (audit-fixes #447), decoded from
/// the four instructions after it:
///
/// ```text
///   0x410A  shr ebp, 4        the high 28 bits are a PARAGRAPH count...
///   0x410E  mov ax, ds
///   0x4110  add ax, bp        ...added to ds to form the SEGMENT
///   0x4112  mov ds, ax
///   0x4114  mov word ptr gs:[di + 6], ax    record+6 = that segment
///   0x4118  mov word ptr gs:[di + 4], si    record+4 = si + (packed & 0xF)
///   0x411C  lodsw / mov word ptr gs:[di + 0xc], ax   record+0xC = its first word
/// ```
///
/// So each directory entry is a byte offset into the loaded resource, split the
/// DOS way: `>> 4` is the paragraph added to `ds`, and the low nibble (added to
/// `si` back at `0x4108`) is the byte remainder. `record+4:+6` is the resulting
/// far pointer to that subobject's data, and `record+0xC` is the first word found
/// there.
///
/// AND THE RESOURCE IS A SPRITE BANK (audit-fixes #448). The decoded header shape
/// — `{flags, count}` then `count` packed offsets — is exactly what the shipped
/// `.SPR` files hold:
///
/// ```text
///   CARTE.SPR     1463 bytes  flags=0x0004 count=7
///                 entries -> 0x01C 0x069 0x10B 0x15F 0x233 0x358 0x40D
///                 (ascending, every one inside the file)
///   CROOLIS1.SPR  4873 bytes  flags=0x0004 count=1  -> 0x004
/// ```
///
/// `flags = 4` is why the dispatch is `(4 & 4) | 0x83` = `0x87`. So `record+4:+6`
/// points at a SPRITE FRAME and `record+0xC` is that frame's first word.
///
/// THE TODO ABOVE IS ANSWERED: there is no routine projecting `0x5491` verts into
/// these records, because the records do not hold projected coordinates — they
/// hold sprite-frame pointers into a shipped bank. The doc's own late guess ("very
/// likely SPRITES drawn at projected positions") was right, and five audit entries
/// were spent searching for a projection because the earlier sentence assuming one
/// was never revisited. What is still unlocated is where the POSITIONS come from,
/// which is a different question from the one that was asked.
pub const SHIP_3D_HUD_PYRAMID_VERTICES: [[i16; 3]; 32] = [
    [0, 2304, 3075],
    [776, 1803, 2820],
    [775, 1546, 2306],
    [517, 1288, 1793],
    [262, 1544, 2308],
    [1034, 2573, 3589],
    [1547, 3088, 4615],
    [2062, 2068, 3076],
    [2829, 3093, 5901],
    [3081, 2840, 6415],
    [4362, 2331, 7186],
    [4359, 1562, 5903],
    [4362, 2327, 5388],
    [4368, 4892, 7956],
    [6159, 3101, 8729],
    [6670, 3875, 9244],
    [0, 1024, 1028],
    [2056, 3080, 3084],
    [4369, 5393, 5397],
    [6425, 7449, 7453],
    [8738, 9762, 9766],
    [10794, 11818, 11822],
    [13107, 14131, 14135],
    [15163, 16187, 16191],
    [7697, 4901, 10016],
    [7959, 2596, 8214],
    [5898, 2334, 7700],
    [8982, 6442, 10532],
    [9753, 6956, 11817],
    [11296, 9008, 13103],
    [60, 0, 36],
    [13323, 8194, 6719],
];


/// As [`render_star_map_navview`], but pans the pyramid field horizontally by
/// `compass_angle` (0..179) so the view rotates with the ship's heading (mouse
/// steering) — the interactive nav behaviour. Nearer rows pan more than far rows
/// (parallax); the orb stays centred.
/// Project a 3D nav position with the EXACT star-map projection decoded from the game
/// (loop @0x9BBA): `t = pos - origin`; `depth = (t·row_z) >> 15` (cull if ≤0);
/// `screen_x = ((t·row_x) >> 7) / depth + 160`; `screen_y = ((t·row_y) >> 7) / depth +
/// 100`. `matrix.terms` are the row-major 3×3 (`build_ship_3d_projection_matrix`).
/// Returns the on-screen (x, y) and the 1/depth sprite scale, or None if culled.
pub fn project_star_map_point(
    pos: [i32; 3],
    origin: [i32; 3],
    matrix: &Ship3dProjectionMatrix,
) -> Option<(i32, i32, i32)> {
    let m = &matrix.terms;
    // 16-BIT SUBTRACT, THEN SIGN-EXTEND — the order the routine uses, and not the
    // same function as subtracting in 32 bits (audit-fixes #586):
    //
    //   0x9be8  mov ax,[0x2f65] / sub [di],ax        x -= origin x   (WORD)
    //   0x9bed  mov ax,[0x2f67] / sub [di+2],ax      y -= origin y   (WORD)
    //   0x9bf3  mov ax,[0x2f69] / sub [di+4],ax      z -= origin z   (WORD)
    //   0x9bf9  movsx eax, word ptr [di]             THEN widen
    //
    // The difference is carried in 16 bits and wraps there, so a separation past
    // 32767 comes out the far side as a small negative. Subtracting in `i32` keeps
    // the true distance instead, and the two agree only while `|pos - origin|`
    // stays inside `i16` — which the intro camera does not guarantee, since
    // `origin_x` is advanced with `wrapping_sub` and passes through `0x8000`.
    let delta = |p: i32, o: i32| ((p as u16).wrapping_sub(o as u16)) as i16 as i32;
    let t = [
        delta(pos[0], origin[0]),
        delta(pos[1], origin[1]),
        delta(pos[2], origin[2]),
    ];
    let mut depth = (t[0] * m[6] + t[1] * m[7] + t[2] * m[8]) >> 15;
    if depth == 0 {
        return None;
    }
    if depth < 0 {
        depth += 0x10000;
    }
    let sx = ((t[0] * m[0] + t[1] * m[1] + t[2] * m[2]) >> 7) / depth + 0xa0;
    let sy = ((t[0] * m[3] + t[1] * m[4] + t[2] * m[5]) >> 7) / depth + 0x64;
    // The game divides the SAME depth two ways: `div ecx` @`0x9C3D` (UNSIGNED)
    // for this scale reciprocal, `idiv ecx` @`0x9C6F` (SIGNED) for the screen
    // axes above. Both are safe as one signed `/` here ONLY because the
    // `depth += 0x10000` fixup @`0x9C29` has already made depth positive — which
    // is why that fixup is not an optimisation to drop.
    //
    // `0x100000` is not a literal in the routine either: it is built as
    // `mov eax,0x8000000` @`0x9C30` then `shr eax,7` @`0x9C36`.
    let scale = 0x100000 / depth;
    Some((sx, sy, scale))
}

/// Projection-accurate star-map nav view: a receding grid of star-system pyramids
/// projected with the game's decoded projection ([`project_star_map_point`]) plus the
/// central eye-orb — the systems tile the ground plane and the compass heading pans the
/// field. Uses the real 0x9BBA projection math (verified), so the perspective is the
/// game's, not a hand-drawn approximation. `light`/`dark`/`orb` are palette indices.
pub fn render_star_map_navview_projected(buffer: &mut [u8], light: u8, dark: u8, orb: u8, compass_angle: u16) {
    const W: isize = SHIP_3D_PROJECTION_SCREEN_WIDTH as isize;
    const H: isize = SHIP_3D_PROJECTION_SCREEN_HEIGHT as isize;
    // Camera pitched down over a ground plane of star systems; heading pans in x.
    let m = match build_ship_3d_projection_matrix(
        &SHIP_3D_ANGLE_TABLE,
        Ship3dMatrixAngles { angle_2f71: 0, projection_angle_2f6d: 0, angle_2f6f: 10 },
    ) {
        Some(m) => m,
        None => return,
    };
    let origin = [0i32, -3500, 0];
    let pan = (compass_angle as i32 - 90) * 24; // steer left/right about centre
    // Fewer, larger, wider-spaced pyramids in a corridor — matched against the real
    // game's title-screen decorative HUD (which has ~4 rows of big pyramids, not a dense
    // field). Verified visually against a boot capture of the original.
    for zi in 0..4 {
        for xi in -3..=3 {
            let d = [xi * 3400 + pan, 0, 2200 + zi * 3300];
            let Some((px, py, _)) = project_star_map_point(d, origin, &m) else {
                continue;
            };
            if !(0..W as i32).contains(&px) || !(0..H as i32).contains(&py) {
                continue;
            }
            // filled pyramid (lit left / shadowed right), size by row (near = bigger)
            let hh = 12 - zi as isize * 2;
            for h in 0..hh {
                let half = hh - h;
                for x in -half..=half {
                    let (a, b) = (px as isize + x, py as isize - h);
                    if (0..W).contains(&a) && (0..H).contains(&b) {
                        buffer[(b * W + a) as usize] = if x <= 0 { light } else { dark };
                    }
                }
            }
        }
    }
    // Central eye-orb: pale sphere with a rim, plus a darker iris ring and pupil so it
    // reads as the game's eye-orb rather than a plain disc.
    let (ocx, ocy, r) = (W / 2, 96 + 22, 20isize);
    for y in -r..=r {
        for x in -r..=r {
            let d2 = x * x + y * y;
            if d2 > r * r {
                continue;
            }
            let (a, b) = (ocx + x, ocy + y);
            if !((0..W).contains(&a) && (0..H).contains(&b)) {
                continue;
            }
            let px = if d2 > (r - 3) * (r - 3) {
                light // bright rim
            } else if d2 <= 16 {
                dark // pupil
            } else if d2 <= 49 {
                light // iris ring highlight
            } else {
                orb // sclera
            };
            buffer[(b * W + a) as usize] = px;
        }
    }
}


pub fn render_ship_3d_pyramid_hud(buffer: &mut [u8], grid_color: u8, orb_color: u8) {
    const W: isize = SHIP_3D_PROJECTION_SCREEN_WIDTH as isize; // 320
    const H: isize = SHIP_3D_PROJECTION_SCREEN_HEIGHT as isize; // 200
    let band_top = SHIP_3D_HUD_BAND_TOP as isize; // 165
    let plot = |buf: &mut [u8], x: isize, y: isize, c: u8| {
        if (0..W).contains(&x) && (band_top..H).contains(&y) {
            buf[(y * W + x) as usize] = c;
        }
    };
    // Bresenham line, clipped to the HUD band.
    let line = |buf: &mut [u8], x0: isize, y0: isize, x1: isize, y1: isize, c: u8| {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut err, mut x, mut y) = (dx + dy, x0, y0);
        loop {
            plot(buf, x, y, c);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    };
    // Perspective grid: rows recede toward a vanishing point up-centre; nearer
    // (lower) rows have larger, wider-spaced pyramids.
    let center_x = W / 2;
    for row in 0..3isize {
        let base_y = band_top + 6 + row * 9; // row baseline
        let half = 4 + row * 2; // pyramid half-width grows toward front
        let apex = base_y - (5 + row * 2); // taller toward front
        let spacing = (half * 2 + 6) as isize;
        let cols = (W / spacing) + 1;
        for col in -(cols / 2)..=(cols / 2) {
            let cx = center_x + col * spacing;
            // pyramid = two slanted edges to the apex + a base line
            line(buffer, cx - half, base_y, cx, apex, grid_color);
            line(buffer, cx + half, base_y, cx, apex, grid_color);
            line(buffer, cx - half, base_y, cx + half, base_y, grid_color);
        }
    }
    // Central eye-orb: a small filled disc centred in the band.
    let orb_cy = band_top + 16;
    let r = 6isize;
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y <= r * r {
                plot(buffer, center_x + x, orb_cy + y, orb_color);
            }
        }
    }
}

/// Render a complete ship-3D starfield background from real game data: randomize
/// the point cloud from `prng`, build the camera matrix from `angles` using the
/// recovered [`SHIP_3D_ANGLE_TABLE`], and project/depth-shade into a 320x200
/// buffer. Returns `None` only if `angles` index outside the trig table (they
/// are `% 180` in the engine, so any in-range angle succeeds). This is the whole
/// background layer — the sprite slots and HUD compose over it separately.
///
/// NO SINGLE ROUTINE IN THE BINARY DOES THIS. The game randomizes the cloud ONCE,
/// at boot: `ship_3d_point_cloud_randomize` (`0x9B67`) has exactly one call site,
/// `lcall 0x71e,0x2387` at `0x0FD3` — between `mov [0x27d9],1` and `back_buffer_init`,
/// before the main loop at `0x0FFB` ever runs. The matrix builder (`0x98B9`) is
/// called from elsewhere entirely (`0x8B9D`, `0x9564`).
///
/// So this composition is a PORT CONVENIENCE, and it is only equivalent because the
/// engine seeds `BloodPrng::seeded_from_rtc_seconds(self.starfield_seed)` from a
/// FIXED `starfield_seed` (17, never reassigned) on every frame: re-deriving the
/// same cloud each time is indistinguishable from keeping the one built at boot.
/// Seed this from the real clock, as the constructor's name invites, and the stars
/// would reshuffle every frame. `the_starfield_is_stable_only_because_the_seed_is`
/// pins that (audit-fixes #584).
pub fn render_ship_3d_starfield(
    prng: &mut BloodPrng,
    angles: Ship3dMatrixAngles,
    origin: Ship3dProjectionOrigin,
    viewport: Ship3dProjectionViewport,
) -> Option<Ship3dPointCloudRender> {
    let points = randomize_ship_3d_point_cloud(prng);
    let matrix = build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles)?;
    Some(render_ship_3d_point_cloud(
        &points, origin, matrix, viewport,
    ))
}

/// Project one nav-destination object to a sprite slot, from
/// `ship_3d_object_sprite_project` @`0x9B98`:
///
/// ```text
///   0x9BBA  dec word [0x2f77]              the object counter, walked DOWN
///   0x9BBE  js 0x9CFB                      negative -> the loop is done
///   0x9BC2  mov eax,[bx] / mov [di],eax    copy the object's coordinates
///   0x9BD1  mov ax,[0x2f77]
///   0x9BD4  add ax,0x15                    + SHIP_3D_NAV_ENTITY_BASE
///   0x9BD7  shl ax,5                       * 32, the entity stride
///   0x9BDA  add ax,0x6212                  + SHIP_3D_ENTITY_TABLE
/// ```
///
/// So the nav destinations do not occupy entity slots `0..n` — they start at
/// entity `0x15`, and the counter is decremented BEFORE use, so it indexes
/// `n-1..0`. Both matter for a port: sharing the entity table means an
/// off-by-`0x15` writes over whatever occupies the low slots, and walking up
/// instead of down reverses which destination lands in which slot when two
/// project to the same place (see the first-write-wins plot in #149).
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn project_ship_3d_object_sprite(
    anchor: Ship3dProjectionPoint,
    origin: Ship3dProjectionOrigin,
    matrix: Ship3dProjectionMatrix,
    descriptor: &mut Ship3dObjectSpriteDescriptor,
) -> Option<Ship3dObjectSpriteProjection> {
    if descriptor.flags & SHIP_3D_OBJECT_VISIBLE_FLAG == 0 {
        return None;
    }

    let translated = [
        projection_component(anchor.x, origin.x),
        projection_component(anchor.y, origin.y),
        projection_component(anchor.z, origin.z),
    ];
    let raw_depth = projection_dot(
        translated,
        [matrix.terms[6], matrix.terms[7], matrix.terms[8]],
    ) >> SHIP_3D_MATRIX_FIXED_SHIFT;
    if raw_depth == 0 {
        return None;
    }

    let depth = if raw_depth < 0 {
        raw_depth.wrapping_add(SHIP_3D_OBJECT_DEPTH_WRAP_BIAS)
    } else {
        raw_depth
    };
    if depth == 0 {
        return None;
    }

    let depth_scale = (SHIP_3D_OBJECT_SCALE_NUMERATOR / depth as u32) as u16;
    let screen_x = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[0], matrix.terms[1], matrix.terms[2]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_X,
    );
    let screen_y = project_ship_3d_axis(
        projection_dot(
            translated,
            [matrix.terms[3], matrix.terms[4], matrix.terms[5]],
        ) >> SHIP_3D_PROJECTION_AXIS_SHIFT,
        depth,
        SHIP_3D_PROJECTION_SCREEN_CENTER_Y,
    );
    let scaled_width = scale_ship_3d_object_dimension(descriptor.source_width, depth_scale);
    let scaled_height = scale_ship_3d_object_dimension(descriptor.source_height, depth_scale);
    update_ship_3d_sprite_slot_extent(descriptor, scaled_width, scaled_height);

    let draw_x = screen_x.wrapping_sub(descriptor.extent_width >> 1);
    let draw_y = screen_y.wrapping_sub(descriptor.extent_height >> 1);
    update_ship_3d_sprite_slot_position(descriptor, draw_x, draw_y);

    Some(Ship3dObjectSpriteProjection {
        projected: Ship3dProjectedPoint {
            x: screen_x,
            y: screen_y,
            depth: depth as u16,
        },
        depth_scale,
        scaled_width,
        scaled_height,
        draw_x,
        draw_y,
    })
}

/// Update a sprite slot's screen position, `sprite_slot_position_update`
/// @`0x420D`:
///
/// ```text
///   0x4210  shl ax,5 / mov bx,0x6212 / add bx,ax   slot = 0x6212 + id*32
///   0x421D  test al,0x81 / je                      ACTIVE mask 0x81
///   0x4221  cmp dx,gs:[bx+8]  / je / or al,2 / mov gs:[bx+8],dx    x  at +0x08
///   0x422D  cmp cx,gs:[bx+0xa]/ je / or al,2 / mov gs:[bx+0xa],cx  y  at +0x0A
/// ```
///
/// Each coordinate is compared BEFORE being written, and the dirty bit (`or al,2`)
/// is set only when the value actually changes — so moving a slot to where it
/// already is marks nothing dirty. The port's per-field `if` mirrors that; writing
/// both unconditionally would dirty every slot every frame and defeat the dirty
/// list the renderer walks (`DS:0x6612`).
///
/// `SHIP_3D_SPRITE_SLOT_ACTIVE_MASK` is that `0x81`, `..._DIRTY_FLAG` the `2`, and
/// `0x6212` is `SHIP_3D_ENTITY_TABLE` with its 32-byte stride.
pub fn update_ship_3d_sprite_slot_position(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
    x: u16,
    y: u16,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_MASK == 0 {
        return effect;
    }

    effect.ran = true;
    if descriptor.draw_x != x {
        descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
        descriptor.draw_x = x;
        effect.marked_dirty = true;
        effect.updated_position = true;
    }
    if descriptor.draw_y != y {
        descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
        descriptor.draw_y = y;
        effect.marked_dirty = true;
        effect.updated_position = true;
    }
    effect
}

/// Update a sprite slot's extent, `sprite_slot_extent_update` @`0x42CD`. The
/// sibling of [`update_ship_3d_sprite_slot_position`], and a good deal subtler:
///
/// ```text
///   0x42D2  shl ax,5 / mov bx,0x6212 / add bx,ax   slot = 0x6212 + id*32
///   0x42DD  test al,0x81 / je                      the same ACTIVE mask
///   0x42E1  lds si,[bp+4]                          the SOURCE dimensions
///   0x42E4  cmp cx,[si]   / jne 0x42F7             width  vs source
///   0x42E8  cmp dx,[si+2] / jne 0x42F7             height vs source
///   0x42ED  btr ax,4                               matches: CLEAR extent-changed
///   0x42F1  jae 0x430D                             ...and if it was ALREADY
///   0x42F3  or al,2                                   clear, do nothing
///   0x42F7  cmp cx,gs:[bx+0xc] / cmp dx,gs:[bx+0xe]  differs: vs the slot extent
///   0x4303  or al,0x12                             set extent-changed AND dirty
///   0x4305  mov gs:[bx+0xc],cx / gs:[bx+0xe],dx
/// ```
///
/// `btr` is bit-test-and-RESET: it clears bit 4 and leaves the OLD bit in CF, so
/// the `jae` right after means "the flag was already clear, nothing to report".
/// Clearing the flag is itself a change worth marking dirty, which is why the
/// matching branch can still set bit 1. And `0x12` is the two flags at once —
/// `EXTENT_CHANGED | DIRTY` — not a third flag.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn update_ship_3d_sprite_slot_extent(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
    width: u16,
    height: u16,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_MASK == 0 {
        return effect;
    }

    effect.ran = true;
    if width == descriptor.source_width && height == descriptor.source_height {
        if descriptor.flags & SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG != 0 {
            descriptor.flags &= !SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG;
            descriptor.flags |= SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
            effect.marked_dirty = true;
            effect.cleared_extent_changed_flag = true;
        }
        return effect;
    }

    if descriptor.extent_width != width || descriptor.extent_height != height {
        descriptor.flags |=
            SHIP_3D_SPRITE_SLOT_DIRTY_FLAG | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG;
        descriptor.extent_width = width;
        descriptor.extent_height = height;
        effect.marked_dirty = true;
        effect.updated_extent = true;
    }
    effect
}

/// Commit one slot's current geometry into its previous-geometry fields — the
/// BODY of `sprite_slot_commit_dirty_range` @`0x43F7`.
///
/// SCOPE, stated because the addresses do not match one-to-one: the game's
/// routine takes a slot RANGE packed into `ebp` (`shl ebp,0x10 / mov bp,bx`),
/// walks it from `0x6212 + first*32`, and has a second path entirely —
///
/// ```text
///   0x4412  test word [0x5249],1 / je 0x4435   the clip-SNAPSHOT flag
///   0x441D  mov eax,[0x5235] / stosd           left+right as one dword
///   0x4423  mov eax,[0x5239] / stosd           top+bottom
///   0x4429  mov word [di],0xffff               terminate the list
///   0x442D  mov word [0x5249],0                and clear the flag
/// ```
///
/// — which pushes the WHOLE clip window into the dirty-rect list at `DS:0x6612`
/// as a single entry instead of per-slot rects. Neither the range walk nor the
/// snapshot is ported, because this engine redraws every frame and keeps no dirty
/// list; what it needs is the per-slot commit, which is what this function is.
///
/// That is also why `update_ship_3d_sprite_slot_position`'s compare-before-write
/// (#150) still matters here: the dirty BIT is read by this commit even though the
/// dirty LIST is not built.
pub fn commit_ship_3d_sprite_slot_dirty_geometry(
    descriptor: &mut Ship3dObjectSpriteDescriptor,
) -> Ship3dSpriteSlotUpdateEffect {
    let mut effect = Ship3dSpriteSlotUpdateEffect::default();
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG == 0 {
        return effect;
    }

    effect.ran = true;
    if descriptor.flags & SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG == 0 {
        return effect;
    }

    descriptor.committed_draw_x = descriptor.draw_x;
    descriptor.committed_draw_y = descriptor.draw_y;
    descriptor.committed_extent_width = descriptor.extent_width;
    descriptor.committed_extent_height = descriptor.extent_height;
    effect.committed_geometry = true;
    effect
}

/// The clip-SNAPSHOT branch of `sprite_slot_commit_dirty_range` @`0x4412`:
///
/// ```text
///   0x4412  test word [0x5249],1 / je 0x4435   the snapshot flag
///   0x441D  mov eax,[0x5235] / stosd           left+right as ONE dword
///   0x4423  mov eax,[0x5239] / stosd           top+bottom as one more
///   0x4429  mov word [di],0xffff               terminate
///   0x442D  mov word [0x5249],0                and CLEAR the flag
/// ```
///
/// The four clip bounds are copied as TWO dwords, not four words, because
/// `DS:0x5235..0x523B` are contiguous — left/right and top/bottom each pair into
/// one 32-bit move. That is why the port reads them as pairs.
///
/// The flag is one-shot: set it, and the next commit replaces the whole
/// per-slot dirty list with a single full-window rect and clears the flag. Losing
/// the clear would make every subsequent frame a full-screen redraw, which looks
/// correct and is the bug the dirty list exists to avoid.
///
/// Cited here because it was settled ASM with no doc (#141's queue); #152 records
/// why the surrounding range walk is not ported.
pub fn commit_ship_3d_global_clip_snapshot(
    dirty_rects: &mut Ship3dDirtyRectList,
    snapshot_armed: &mut bool,
    clip: Ship3dProjectionViewport,
) -> Ship3dDirtyRectSnapshotEffect {
    if !*snapshot_armed {
        return Ship3dDirtyRectSnapshotEffect::default();
    }

    dirty_rects.rects.clear();
    dirty_rects.rects.push(clip);
    dirty_rects.sentinel = SHIP_3D_DIRTY_RECT_SENTINEL;
    *snapshot_armed = false;

    Ship3dDirtyRectSnapshotEffect {
        ran: true,
        wrote_clip_rect: true,
        wrote_sentinel: true,
        cleared_snapshot_flag: true,
    }
}

/// Walk the dirty-rect list against the active sprite slots —
/// `sprite_slot_dirty_range_render` @`0x4471`:
///
/// ```text
///   0x448A  mov di,0x6612          the dirty-rect list
///   0x448D  mov ax,[di]
///   0x448F  or ax,ax / js 0x4516   TERMINATED BY SIGN, not by 0xFFFF
///   0x4495  mov bx,bp / shr ebp,0x10   unpack the slot range from ebp
///   0x44A2  mov di,0x6212 / shl bx,5 / add di,bx / sub di,0x20
///                                  start at the LAST slot and step back
/// ```
///
/// The terminator test is `js`, so ANY negative word ends the list — `0xFFFF` is
/// simply the value the writer uses (`0x1001`, `0x4429`). A port that compares
/// against `0xFFFF` exactly agrees on every list the game builds and disagrees on
/// any other negative sentinel.
///
/// The slot walk runs BACKWARD from the range's end (`sub di,0x20` per step),
/// which matters for the same reason #158's backward object loop did: with
/// first-write-wins plotting, order decides what survives.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn collect_ship_3d_dirty_sprite_slot_render_commands(
    slots: &mut [Ship3dObjectSpriteDescriptor],
    dirty_rects: &Ship3dDirtyRectList,
    start_index: usize,
    end_index: usize,
) -> Vec<Ship3dSpriteSlotRenderCommand> {
    if dirty_rects.rects.is_empty() || start_index > end_index {
        return Vec::new();
    }

    let mut commands = Vec::new();
    for slot_index in (start_index..=end_index).rev() {
        let Some(slot) = slots.get_mut(slot_index) else {
            continue;
        };
        let flags = slot.flags;

        if flags & SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG != 0 {
            let slot_rect = Ship3dProjectionViewport {
                left: slot.draw_x,
                right: slot.draw_x.wrapping_add(slot.extent_width),
                top: slot.draw_y,
                bottom: slot.draw_y.wrapping_add(slot.extent_height),
            };
            for dirty_rect in &dirty_rects.rects {
                if ship_3d_rects_intersect(slot_rect, *dirty_rect) {
                    commands.push(Ship3dSpriteSlotRenderCommand {
                        slot_index,
                        dispatch_index: ((flags >> 1) & 0x07) as u8,
                        destination_remap_mode: ((flags >> 8) & 0x03) as u8,
                        flip_x: flags & 0x0020 != 0,
                        flip_y: flags & 0x0040 != 0,
                        slot_rect,
                        dirty_rect: *dirty_rect,
                    });
                }
            }
        }

        slot.flags &= !SHIP_3D_SPRITE_SLOT_DIRTY_FLAG;
    }

    commands
}

/// The temporary `sn\3D.snd` presentation path — `ship_3d_temp_snd_setup`
/// @`0xB591` (`0x0A9A:0x05F1`), verified instruction by instruction (audit-fixes
/// #484). The whole body is gated on `test byte [0xAE4],1 / je` @0xB592, so a
/// clear trigger returns having done nothing.
///
/// Taken in order, the routine: clears `[0xAE4]` and `[0xAE3]`; PUSHES the mouse
/// position `[0xA2A]`/`[0xA2C]` @0xB5A5 and pops it back @0xB64F, which is the
/// `preserved_mouse_position` effect; cycles the overlay index `[0xAE5]` 0..2
/// @0xB5B5 and indexes the 3-pointer table `DS:0xACC` (amer / croolis / scrut);
/// loads `sn\3D.snd` (`si = DS:0xD23`) @0xB5DC; zeroes the callback-bank gate
/// `[0xBA0]` around `lcall [0xA96]` @0xB5FE; restores `sn\tb.snd`
/// (`si = DS:0xCFC`) @0xB610; writes the viewport descriptor through the far
/// pointer at `DS:0x522D` @0xB629 (see `SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR`);
/// then clears the hold counter `[0xB3B]` and sets the refresh flag `[0x5B55]`.
///
/// THE INDEX USES THE OLD PHASE, and the encoding is the reason. `0x98` @0xB5C2
/// is a bare one-byte **CBW** — capstone prints it `cwde` in 16-bit mode (see
/// re/tools/check_opsize_mnemonics.py, audit-fixes #100). CBW sign-extends AL,
/// OVERWRITING the AH that `inc ah` had just advanced, so `add ax,ax / add si,ax`
/// indexes with the PRE-advance value. Read as `cwde` the index would be
/// `(next<<8)|old` doubled — far outside a 3-pointer table. Hence the ordering
/// here: read the callback offset first, advance the phase after.
///
/// The tail branches on `test byte [0x252A],1` @0xB657 — `sequence_active`:
/// - set (@0xB65E): `[0x252E]` is cleared, `lcall 0x8B:0x967` runs, `[0x252E]` is
///   set again, then `[0x1FA3] = 0xFFFF`. The clear-then-set below is NOT
///   redundant — it brackets that call, and the sentinel is the scene selector.
/// - clear (@0xB675): `lcall 0x8B:0x929`, then the two byte clears `[0x5B53]` and
///   `[0x5B57]` — the pair modelled as `setup_flag_a` / `setup_flag_b`.
pub fn run_ship_3d_temp_snd_setup(state: &mut Ship3dTempSndState) -> Option<Ship3dTempSndEffect> {
    if !state.trigger {
        return Some(Ship3dTempSndEffect::default());
    }

    let selected_callback_offset =
        SHIP_3D_TEMP_SND_CALLBACK_OFFSETS.get(usize::from(state.phase))?;
    let mut effect = Ship3dTempSndEffect {
        ran: true,
        selected_callback_offset: Some(*selected_callback_offset),
        load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
        restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
        preserved_mouse_position: true,
        reset_callback_bank_gate: true,
        called_presentation_callback: true,
        reset_hold_ticks: true,
        wrote_viewport_descriptor: true,
        ..Ship3dTempSndEffect::default()
    };

    state.trigger = false;
    state.auxiliary_trigger = false;
    state.phase = next_ship_3d_temp_snd_phase(state.phase);
    effect.next_phase = Some(state.phase);
    state.hold_ticks = 0;
    state.fullscreen_refresh = true;
    state.viewport_descriptor = SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR;

    if state.sequence_active {
        // @0xB65E: `[0x252E]` is cleared, `lcall 0x8B:0x967` runs with it clear,
        // and it is set again @0xB668. The clear/set pair BRACKETS that call —
        // not dead motion, which is why both edges are reported as effects.
        state.plane_copy_enabled = false;
        effect.temporarily_disabled_plane_copy = true;
        state.plane_copy_enabled = true;
        state.scene_selector = SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL;
        effect.enabled_plane_copy = true;
        effect.reset_scene_selector = true;
        effect.sequence_branch = true;
    } else {
        state.setup_flag_a = false;
        state.setup_flag_b = false;
        effect.reset_setup_flags = true;
        effect.non_sequence_branch = true;
    }

    Some(effect)
}

pub fn run_ship_3d_navigation_final_reset(
    state: &mut Ship3dNavigationFinalResetState,
) -> Ship3dNavigationFinalResetEffect {
    if !state.exit_pending {
        return Ship3dNavigationFinalResetEffect::default();
    }

    if state.opening {
        return Ship3dNavigationFinalResetEffect {
            reentered_active_sequence: true,
            ..Ship3dNavigationFinalResetEffect::default()
        };
    }

    state.hud_flags = SHIP_3D_FINAL_RESET_HUD_FLAGS;
    state.nav_choice_hold_ticks = 0;
    state.nav_choice_timer = SHIP_3D_FINAL_RESET_NAV_TIMER;
    state.post_reset_gate = true;
    state.navigation_gate = true;

    // From `0xB521` the reset stops being nav-specific: `xor ax,ax` there is the
    // entry labelled `dlg_clear_b`, the DIALOGUE CLEAR, inlined rather than
    // called (`dlg_clear_a` at `0x1A5E` clears the same `0x1FAB`/`0x6788` pair).
    // `ax` is zeroed once and then reused by every store, which is why the two
    // sentinels below are separate `mov word` instructions carrying their
    // immediate while the neighbours are one-byte `mov [addr],al` (audit-fixes
    // #287). The order here follows the routine's store order.
    state.dialogue_state = 0;
    state.scene_band_top = 0;
    state.scene_selector = SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL;
    state.active_record = SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL;
    state.presentation_gate = false;
    state.exit_pending = false;
    state.pending_state_byte = false;
    state.subtitle_gate = false;
    state.presentation_defer_active = false;
    state.secondary_presentation_defer_active = false;
    state.plane_copy_enabled = false;
    state.sequence_active = false;
    state.status_flags &= SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK;
    state.secondary_status_flag = false;

    state.dirty_marker = SHIP_3D_FINAL_RESET_DIRTY_MARKER;
    state.scroll_value = 0;
    state.scroll_mode = SHIP_3D_FINAL_RESET_SCROLL_MODE;

    Ship3dNavigationFinalResetEffect {
        ran: true,
        cleared_dialogue_state: true,
        reset_hud_state: true,
        reset_presentation_gates: true,
        reset_sequence_flags: true,
        reset_status_flags: true,
        copied_backbuffer_restore_block: true,
        cleared_overlay_scratch: true,
        reset_scroll_state: true,
        called_render_clear: true,
        called_input_reset: true,
        called_target_cleanup: true,
        ..Ship3dNavigationFinalResetEffect::default()
    }
}

/// The ship-3D camera-approach animation state, driven by phase counter `DS:0x27DF`
/// (BLOODPRG.EXE `0x8A6A..0x8B5A`). The nav camera moves because this scripted FSM
/// walks the camera origin `[0x2F65/67/69]` and angle `[0x2F71]` through fixed phases
/// each frame — the source of the "the ship travels" motion, decoded and portable.
///
/// Phased camera-intro parameters (see [`Ship3dCameraApproach::step`]):
/// phase 1 pulls X toward [`SHIP_INTRO_X_END`] in [`SHIP_INTRO_X_STEP`] increments while spinning
/// the yaw (wrapping at [`SHIP_INTRO_YAW_WRAP`]); phase 2 accelerates Z up to
/// [`SHIP_INTRO_Z_CRUISE`], gaining [`SHIP_INTRO_Z_ACCEL_STEP`] each frame; phase 3 resets to the
/// cruise pose ([`SHIP_INTRO_X_RESET`]); phase 4 settles Z at [`SHIP_INTRO_Z_FINAL`].
const SHIP_INTRO_X_END: u16 = 9000;
/// `sub ax,0x64` @`0x8A82` — X is pulled DOWN toward `SHIP_INTRO_X_END` by 100 a
/// frame while `cmp ax,0x2328 / jl` @`0x8A7D` holds the phase (audit-fixes #507).
const SHIP_INTRO_X_STEP: u16 = 100;
/// `mov ax,0xb4` @`0x8A8E` — the yaw cell `[0x2f71]` is DECREMENTED and reloaded
/// with 180 when `dec ax / jns` @`0x8A8B` goes negative, so the spin runs
/// 179..0 downward. (The alternate path @`0x8A9E` increments and wraps to 0 at
/// the same 180.) — audit-fixes #507
const SHIP_INTRO_YAW_WRAP: u16 = 180;
/// `cmp ax,0x4e20 / ja` @`0x8ABA` — the phase-2 ceiling on the camera Z cell
/// `[0x2f69]`, tested UNSIGNED (audit-fixes #507).
const SHIP_INTRO_Z_CRUISE: u16 = 20000;
/// `add word ptr [0x2f6b],0x64` @`0x8AC3` — 100 is added to the VELOCITY cell,
/// not to Z. Z gains that velocity (`add ax,[0x2f6b]` @`0x8ABF`), so the approach
/// ACCELERATES; adding 100 to Z directly would give a constant glide
/// (audit-fixes #507).
const SHIP_INTRO_Z_ACCEL_STEP: u16 = 100;
/// `mov word ptr [0x2f65],0x2710` @`0x8AFE` — phase 3 snaps X back to 10000
/// alongside `[0x2f69] = 0x4e20` @`0x8AF2` and a zeroed yaw @`0x8AF8`
/// (audit-fixes #507).
const SHIP_INTRO_X_RESET: u16 = 10000;
const SHIP_INTRO_Z_FINAL: u16 = 30000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ship3dCameraApproach {
    /// Phase counter (`DS:0x27DF`): 1 = pull-in X, 2 = accelerate Z, 3 = reset,
    /// 4 = hold, then done.
    pub phase: u8,
    /// Camera origin words `DS:0x2F65/0x2F67/0x2F69`.
    pub origin_x: u16,
    pub origin_y: u16,
    pub origin_z: u16,
    /// Z acceleration accumulator `DS:0x2F6B` (added to Z each frame in phase 2,
    /// itself growing by 0x64/frame).
    pub z_accel: u16,
    /// Camera yaw `DS:0x2F71` (0..0xB4 = 0..180°), rotated during phase 1.
    pub angle_2f71: u16,
    /// True once the approach animation has completed all phases.
    pub done: bool,
}

impl Default for Ship3dCameraApproach {
    fn default() -> Self {
        // Phase-3 reset immediates (`0x8AF2..0x8AFE`): the approach's start state.
        //
        // `origin_y` WAS 0 HERE AND THAT WAS WRONG (audit-fixes #275). The reset
        // writes only `[0x2F69]=0x4E20` @`0x8AF2`, `[0x2F71]=0` @`0x8AF8` and
        // `[0x2F65]=0x2710` @`0x8AFE` — it never touches `[0x2F67]`, so Y keeps
        // the value already in force. That value is 12000: the full origin reset
        // at `0x8CB4..0x8CC0` sets `(0x2710, 0x2EE0, 0)`, and the shipped image
        // carries the same three words at `DS:0x2F65`.
        //
        // A default of 0 shifted the approach camera's Y origin by 12000 units
        // for every frame it fed to the projector.
        Self {
            phase: 1,
            origin_x: 0x2710, // 10000, `mov word [0x2f65],0x2710` @0x8AFE
            origin_y: 0x2EE0, // 12000, NOT reset here -- `mov word [0x2f67],0x2ee0` @0x8CBA
            origin_z: 0x4E20, // 20000, `mov word [0x2f69],0x4e20` @0x8AF2
            z_accel: 0,
            angle_2f71: 0,
            done: false,
        }
    }
}

impl Ship3dCameraApproach {
    /// Advance one frame through the decoded phase machine (`0x8A6A..0x8B5A`):
    /// - **P1** (`0x8A76`): if `X >= 0x2328` (9000), `X -= 0x64`; the yaw
    ///   `[0x2F71]` decrements toward 0 (wrapping to 0xB4). When `X < 0x2328`, P1
    ///   ends (`inc phase`).
    /// - **P2** (`0x8AB3`): while Z is below the cruise altitude, accelerate Z upward. Above it,
    ///   P2 ends.
    /// - **P3** (`0x8AE0`): reset to the cruise pose. P3 ends.
    /// - **P4** (`0x8B2B`): settle Z at its final altitude. P4 ends → animation done.
    ///
    /// TWO SIGNEDNESS DIFFERENCES, verified reachable-safe rather than ignored
    /// (audit-fixes #278):
    ///
    ///   * P1's end test is `cmp ax,0x2328 / jl` @`0x8A7D` — the `jl` at `0x8A80` is SIGNED. The port
    ///     compares `u16 >= 9000`, unsigned. They agree for every X the animation
    ///     reaches, since X starts at 10000 and falls; they would differ only
    ///     above `0x7FFF`.
    ///   * P1's yaw wrap is `dec ax / jns / mov ax,0xb4` @`0x8A8B` — it wraps when
    ///     the DECREMENT goes negative, so `0 -> 180`. The port writes
    ///     `if angle == 0 { 180 } else { angle - 1 }`, identical for `0..=180`
    ///     and different only for a yaw above `0x8000`, which the wrap itself
    ///     prevents.
    ///
    /// P2's `cmp ax,0x4e20 / ja` @`0x8ABA` (the `ja` at `0x8ABD`) IS unsigned, matching the port's `<=`
    /// directly, and the accumulate order is the routine's: `z += accel` first
    /// (`0x8ABF`), then `accel += 0x64` (`0x8AC3`).
    pub fn step(&mut self) {
        match self.phase {
            1 => {
                if self.origin_x >= SHIP_INTRO_X_END {
                    self.origin_x = self.origin_x.wrapping_sub(SHIP_INTRO_X_STEP);
                    // Spin the yaw down each frame, wrapping past zero back to the top of the turn.
                    self.angle_2f71 = if self.angle_2f71 == 0 {
                        SHIP_INTRO_YAW_WRAP
                    } else {
                        self.angle_2f71 - 1
                    };
                } else {
                    self.phase += 1;
                }
            }
            2 => {
                if self.origin_z <= SHIP_INTRO_Z_CRUISE {
                    self.origin_z = self.origin_z.wrapping_add(self.z_accel);
                    self.z_accel = self.z_accel.wrapping_add(SHIP_INTRO_Z_ACCEL_STEP);
                } else {
                    self.phase += 1;
                }
            }
            3 => {
                self.origin_z = SHIP_INTRO_Z_CRUISE;
                self.angle_2f71 = 0;
                self.origin_x = SHIP_INTRO_X_RESET;
                self.phase += 1;
            }
            4 => {
                self.origin_z = SHIP_INTRO_Z_FINAL;
                self.phase += 1;
            }
            _ => self.done = true,
        }
    }

    /// The camera origin as a projection origin (for `project_star_map_point` etc.).
    ///
    /// The three cells the projector subtracts: `[0x2F65]`, `[0x2F67]`, `[0x2F69]`,
    /// read at `0x9BE8`/`0x9BED`/`0x9BF3` and seeded by `mov word [0x2f65],0x2710`
    /// @`0x8AFE` and `mov word [0x2f69],0x4e20` @`0x8AF2`.
    ///
    /// The `as i16` here is presentation only — [`project_star_map_point`] does the
    /// subtraction in 16 bits itself (`sub [di],ax` then `movsx`), so widening the
    /// origin first cannot change the result. It is signed rather than zero-extended
    /// so the value READS as the coordinate it is when printed or asserted on
    /// (audit-fixes #586).
    pub fn origin(&self) -> [i32; 3] {
        [
            self.origin_x as i16 as i32,
            self.origin_y as i16 as i32,
            self.origin_z as i16 as i32,
        ]
    }
}

pub fn build_ship_3d_navigation_source_records(
    source_entries: &[Ship3dNavigationSourceEntry],
    records: &[Ship3dNavigationRuntimeRecord],
    root_target: u16,
) -> Option<Vec<u16>> {
    let mut source_records = Vec::new();
    append_ship_3d_navigation_source_children(
        source_entries,
        records,
        root_target,
        &mut source_records,
    )?;
    source_records.push(SHIP_3D_TARGET_EXIT_SENTINEL);
    Some(source_records)
}

pub fn build_ship_3d_navigation_candidate_records(
    source_records: &[u16],
    records: &[Ship3dNavigationRuntimeRecord],
    honk_object: u16,
) -> Option<Vec<u16>> {
    let mut candidates = Vec::new();
    for record_offset in source_records {
        if *record_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            return Some(candidates);
        }
        if *record_offset == honk_object {
            continue;
        }

        let record = find_ship_3d_navigation_record(records, *record_offset)?;
        if record.kind_flags == SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE
            && record.state_flags & SHIP_3D_NAVIGATION_RECORD_ACTIVE_FLAG != 0
        {
            candidates.push(*record_offset);
        }
    }
    None
}

/// The position-field resolver, `ship_3d_position_distance`'s front half
/// (`0x60DD`): a ladder on the record's KIND that picks which selector resolves
/// its position (audit-fixes #283 — the function and its five constants had no
/// origin between them).
///
/// ```text
///   0x60E3  mov ax,[si]      the kind
///   0x60E5  cmp ax,0x100     KIND100 -> the comparing branch
///   0x60EC  mov ax,0xe  / call 0x6023   relation word, from the DI record
///   0x60F6  mov bx,0x100
///   0x60F9  mov ax,0xc  / call 0x6023   match word, kind 0x100, from SI
///   0x6101  mov ax,9
///   0x6104  cmp dx,[bx+si] / je         equal -> selector 9
///   0x6108  inc ax                      otherwise -> selector 10
///   0x6114  cmp ax,0x40                 the direct kinds continue below
/// ```
///
/// THIS FUNCTION MERGES TWO GAME ROUTINES, which #283 did not say and which
/// changes how the code below should be read (audit-fixes #289). `0x60DD` tests
/// only `0x100` and `0x40`; everything else it delegates (`call 0x61A6`
/// @`0x6126`). The remaining three direct kinds and the parent walk live in
/// `ship_3d_position_field_resolve` (`0x61A6`):
///
/// ```text
///   0x61AB  mov ax,[si]                  the kind
///   0x61AD  cmp ax,0x100 / je            KIND100
///   0x61B2  cmp ax,8     / je 0x61DF     direct
///   0x61B7  cmp ax,0x10  / je 0x61DF     direct
///   0x61BC  cmp ax,0x200 / je 0x61DF     direct   <- note: NO 0x40 here
///   0x61C3  mov ax,0x11 / call 0x6023    the parent selector
///   0x61C9  add si,ax                    added UNCONDITIONALLY
///   0x61CB  mov si,[si]                  follow the link
///   0x61CD  cmp si,-1 / jne              0xFFFF -> arche fallback gs:[0x6752]
/// ```
///
/// The union of the two ladders is `{8, 0x10, 0x40, 0x200}` and all four resolve
/// selector 11, so the `match` below is behaviourally right — but `0x40` comes
/// from a different routine than the other three, and no single routine tests
/// all four.
///
/// SUSPECTED DIVERGENCE, recorded rather than silently "fixed": the arm below
/// does `if parent_field == 0 { return None }`, and NO INSTRUCTION DOES THAT.
/// `0x61C9` adds the offset unconditionally and dereferences, so a kind whose
/// selector-0x11 column is 0 makes the game read the KIND WORD ITSELF as the
/// next record pointer. Column 5 (kind `0x20`) is such a column. Whether any
/// shipped record has kind `0x20` and reaches this walk is a DATA question that
/// decides whether this is a live bug or an unreachable one; until that is
/// answered, changing it would trade a decoded-but-unreached path for an
/// undecoded one. Tracked in docs/port-validation.md.
pub fn resolve_ship_3d_position_field(
    records: &[Ship3dPositionRecord],
    record_offset: u16,
    arche_object: u16,
    kind100_compare_word: u16,
) -> Option<u16> {
    let mut current_offset = record_offset;
    for _ in 0..records.len().saturating_add(1) {
        let record = find_ship_3d_position_record(records, current_offset)?;
        match record.kind_flags {
            SHIP_3D_OBJECT_KIND_POSITION_KIND100 => {
                let selector = if record.kind100_match_word? == kind100_compare_word {
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MATCH
                } else {
                    SHIP_3D_FIELD_SELECTOR_KIND100_POSITION_MISMATCH
                };
                return ship_3d_record_field(record.offset, record.kind_flags, selector);
            }
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40
            | SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200 => {
                return ship_3d_record_field(
                    record.offset,
                    record.kind_flags,
                    SHIP_3D_FIELD_SELECTOR_POSITION,
                );
            }
            kind_flags => {
                let parent_field =
                    vm::vm_field_offset(SHIP_3D_FIELD_SELECTOR_PARENT_LINK, kind_flags)?;
                if parent_field == 0 {
                    return None;
                }
                current_offset = record.parent_link.unwrap_or(arche_object);
            }
        }
    }
    None
}

pub fn ship_3d_position_distance(
    records: &[Ship3dPositionRecord],
    fields: &[Ship3dPositionField],
    first_record_offset: u16,
    second_record_offset: u16,
    arche_object: u16,
    inherited_kind100_compare_word: u16,
) -> Option<u16> {
    let first_record = find_ship_3d_position_record(records, first_record_offset)?;
    let second_record = find_ship_3d_position_record(records, second_record_offset)?;
    let first_field_offset = resolve_ship_3d_distance_position_field(
        records,
        first_record,
        second_record,
        arche_object,
        inherited_kind100_compare_word,
    )?;
    let second_field_offset = resolve_ship_3d_distance_position_field(
        records,
        second_record,
        first_record,
        arche_object,
        inherited_kind100_compare_word,
    )?;
    let first_field = find_ship_3d_position_field(fields, first_field_offset)?;
    let second_field = find_ship_3d_position_field(fields, second_field_offset)?;
    ship_3d_position_field_distance(first_field, second_field)
}

/// Euclidean distance between two object POSITION FIELDS —
/// `ship_3d_position_distance` @`0x60DD`.
///
/// The routine's interesting half is how it FINDS the coordinates, which this
/// function takes as already-resolved inputs:
///
/// ```text
///   0x60E5  cmp ax,0x100 / jne 0x6114   the first object must be kind 0x100
///   0x60EA  mov bx,[di]                 the second object's kind
///   0x60EC  mov ax,0xe / call 0x6023    vm_field_offset(selector 0xE, that kind)
///   0x60F4  mov dx,[bx+di]              read the field it resolved to
///   0x60F6  mov bx,0x100 / mov ax,0xc / call 0x6023   selector 0xC for kind 0x100
/// ```
///
/// So the two coordinates come from DIFFERENT selectors — `0xC` for the kind-`0x100`
/// object, `0xE` for the other — resolved per kind through `vm_field_offset`
/// (`0x6023`, the `BSF`-column resolver). Kind `0x100` is
/// `vm::LOCATION_KIND_BLACK_HOLE`, the same bit the status header tests.
///
/// The arithmetic here is `sqrt(dx^2 + dy^2)` over ABSOLUTE differences, with the
/// squares accumulated in 32 bits before [`ship_3d_binary_sqrt`] — which is why
/// that function takes a `u32` and not a pair of words.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn ship_3d_position_field_distance(
    first: Ship3dPositionField,
    second: Ship3dPositionField,
) -> Option<u16> {
    let dx = binary_abs_word_diff(first.x, second.x) as i16 as i32 as u32;
    let dy = binary_abs_word_diff(first.y, second.y) as i16 as i32 as u32;
    let squared = dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy));
    ship_3d_binary_sqrt(squared)
}

/// Integer square root of a 32-bit value, `binary_u32_sqrt` @`0x2E33` — the
/// helper `ship_3d_position_distance` (`0x60DD`) uses for object distances.
///
/// The estimate seeding is a ladder on the input's magnitude:
///
/// ```text
///   0x2E3B  or dx,dx / je 0x2E4F      high == 0 ?
///   0x2E3F  mov bx,0xfff              high != 0: estimate 0x0FFF
///   0x2E42  or dh,dh / je 0x2E5C      ...unless the TOP byte is set
///   0x2E46  mov bh,0xff               then 0xFFFF -- built by overwriting
///                                     0x0FFF's high byte, not reloaded
///   0x2E48  cmp dx,-2 / jae 0x2E6E    high >= 0xFFFE: return the input
///   0x2E4F  or ax,ax / je 0x2E6E      zero: the root is the input
///   0x2E53  mov bx,0xf                low only: estimate 0x000F
///   0x2E56  or ah,ah / je 0x2E5C      ...or 0x00FF if the high byte is set
/// ```
///
/// `cmp dx,-2 / jae` is an UNSIGNED compare against `0xFFFE`, so it means "the
/// high word is at or above 0xFFFE" rather than anything about negatives — the
/// case where the root would overflow a `u16`, answered by returning the input.
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn ship_3d_binary_sqrt(value: u32) -> Option<u16> {
    let low = value as u16;
    let high = (value >> 16) as u16;

    // Seed the estimate from the input's magnitude (leading-bit brackets, as the original does),
    // returning early for zero/tiny inputs whose root is the input itself. The brackets are bit
    // patterns, so hexadecimal is the natural form here.
    let mut estimate: u16 = if high & 0xff00 != 0 {
        if high >= 0xfffe {
            return Some(low);
        }
        0xffff
    } else if high != 0 {
        0x0fff
    } else if low == 0 {
        return Some(low);
    } else if low & 0xff00 != 0 {
        0x00ff
    } else {
        0x000f
    };

    // Newton's method: estimate <- (estimate + value / estimate) / 2, iterated until it stops
    // decreasing. The (sum >> 1) with carry into the top bit is the exact averaging the original
    // uses so results are bit-identical.
    loop {
        let quotient = value / estimate as u32;
        if quotient > u16::MAX as u32 {
            return None;
        }
        let (sum, carry) = (quotient as u16).overflowing_add(estimate);
        let candidate = (sum >> 1) | if carry { 0x8000 } else { 0 };
        if candidate >= estimate {
            return Some(candidate);
        }
        estimate = candidate;
    }
}

/// `ship_3d_object_table_bit_test_full` @`0x6210` — is this object's bit set in the
/// selector-5/kind-2 bitset?
///
/// ```text
/// 0x6216  les di,gs:[0x672c]              the 20-byte directory
/// 0x621d  cmp ax,es:[di+0x10] / je 0x6229 match on the object offset...
/// 0x6223  add di,0x14 / inc cx / jmp      ...counting the DIRECTORY INDEX in cx
/// 0x6229  mov ax,5 / mov bx,2             selector 5, kind 2 -- BOTH FIXED
/// 0x622f  call 0x6023 / add si,ax         the bitset field
/// 0x6234  mov ax,cx                       then index>>3 selects the byte
/// ```
///
/// THE SELECTOR AND KIND ARE LITERALS (`mov ax,5` / `mov bx,2`), not derived from the
/// object being tested — so an object of any kind is looked up in the kind-2 column.
/// That is why the port passes constants here rather than the record's own kind.
///
/// THE SCAN HAS NO TERMINATION CHECK. `0x621D`..`0x6227` loops until it finds a
/// match; an object not in the directory walks off the end of the table. The port's
/// `.position()?` returns `None` instead — a guard, not a decode (audit-fixes #605).
pub fn ship_3d_object_table_bit_is_set(
    object_table_records: &[u16],
    bitset_base: &[u8],
    object_record_offset: u16,
) -> Option<bool> {
    let object_index = object_table_records
        .iter()
        .position(|record| *record == object_record_offset)?;
    let field_offset =
        vm::vm_field_offset(SHIP_3D_SOURCE_BITSET_SELECTOR, SHIP_3D_SOURCE_BITSET_KIND)? as usize;
    let byte_offset = field_offset.checked_add(object_index >> 3)?;
    let value = *bitset_base.get(byte_offset)?;
    // HIGH-BIT-FIRST, and the equivalence is worth spelling out because the
    // routine expresses it as a shift rather than a mask: `cl = (index & 7) + 1`
    // then `shl al,cl` @`0x6236` leaves bit `7 - (index & 7)` in CF, which is
    // what `0x80 >> (index & 7)` selects. Index 0 is bit 7, index 7 is bit 0 —
    // the opposite of the `1 << i` a reader would write unprompted.
    let mask = vm::bit_flag_mask((object_index & 7) as u8);
    Some(value & mask != 0)
}

/// The C1 kind-0x10 source-list scan, `0x6C1C`. Reached from the 0xC1 handler
/// (`0x6B4C`) only when the resolved record's kind is `0x10` (`cmp ax,0x10 /
/// jne` @`0x6C07`); the handler first rebuilds the list by calling
/// `ship_3d_nav_source_list_build_full` (`0x624B`) with `bp = 0x6886` @`0x6C0D`.
///
/// The loop is `lodsw` @`0x6C1C`, exit on the `-1` sentinel (`cmp ax,-1 / je`
/// @`0x6C1D`), then a two-way branch on the record's kind word: 2 selects the
/// object-table bitset test, 1 selects the operand-flag test, and anything else
/// falls back to the next entry (`jne 0x6C1C` @`0x6C39`) — which is why the port's
/// `match` needs its `_ => {}` arm.
///
/// `source_list_bytes` starts at the binary's `DS:0x6886` scratch list. Kind-2
/// tests use the post-`lodsw` cursor for the current source record as the bitset
/// base before applying helper `0x6210`'s selector-5 offset.
///
/// THAT OFFSET IS `0x1E` (audit-fixes #293), read from the matrix and pinned by
/// `field_matrix_entries_match_the_constants`, and the number has a consequence
/// worth stating. `0x6210` ends in `mov al, byte ptr [si]` @`0x6240` — DS-relative,
/// and DS is GS at this call site (`mov ax,gs / mov ds,ax` @`0x6C15`) — so the
/// bitset byte is read from the `0x6886` SCRATCH BUFFER, thirty bytes past the
/// cursor plus `index >> 3`. For a short source list that lands beyond the
/// `0xFFFF` terminator, in whatever the scratch happens to hold.
///
/// So a faithful kind-2 arm needs the real BUFFER, not a list of offsets. This
/// function takes `source_list_bytes` and indexes it exactly as the game does,
/// which is why the `ExecutionContext` path can model it; `VmMachine`'s
/// `build_nav_source_list` returns `Vec<u16>` of entries and therefore cannot,
/// which is the concrete blocker recorded in #292 — now with its reason
/// quantified rather than described.
pub fn select_ship_3d_c1_source_record(
    source_records: &[u16],
    records: &[Ship3dNavigationRuntimeRecord],
    object_table_records: &[u16],
    source_list_bytes: &[u8],
    operand_record_offset: u16,
    operand_state_flags: u8,
) -> Option<Option<u16>> {
    for (source_index, record_offset) in source_records.iter().enumerate() {
        if *record_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            return Some(None);
        }

        let record = find_ship_3d_navigation_record(records, *record_offset)?;
        match record.kind_flags {
            SHIP_3D_C1_SOURCE_KIND_BITSET => {
                let bitset_cursor = source_index.checked_add(1)?.checked_mul(2)?;
                let bitset_base = source_list_bytes.get(bitset_cursor..)?;
                if ship_3d_object_table_bit_is_set(
                    object_table_records,
                    bitset_base,
                    operand_record_offset,
                )? {
                    return Some(Some(record.offset));
                }
            }
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG => {
                if operand_state_flags & SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG != 0 {
                    return Some(Some(record.offset));
                }
            }
            _ => {}
        }
    }

    None
}

/// Where a `0xC1` kind-`0x10` write lands: `0x6C48`..`0x6C53`.
///
/// ```text
/// 0x6c48  mov ax,0x13 / mov bx,0x10   selector 0x13, for kind 0x10 SPECIFICALLY
/// 0x6c4e  call 0x6023                 vm_field_offset
/// 0x6c51  add ax,di / mov bp,ax       destination = target record + that field
/// ```
///
/// The kind is a LITERAL `0x10` in the field-matrix lookup, not the target's own
/// kind flags — `mov bx,0x10` — which is why the guard above rejects anything else
/// rather than resolving the selector against whatever kind arrived (audit-fixes
/// #598).
pub fn resolve_ship_3d_c1_kind10_destination_record(
    target_record_offset: u16,
    target_kind_flags: u16,
) -> Option<u16> {
    if target_kind_flags != SHIP_3D_C1_KIND10_RECORD_KIND {
        return None;
    }
    vm::vm_field_offset(
        SHIP_3D_C1_DESTINATION_SELECTOR,
        SHIP_3D_C1_KIND10_RECORD_KIND,
    )
    .map(|field| target_record_offset.wrapping_add(field))
}

/// The write itself, `0x6C55`..`0x6C6C` — and it only happens into an EMPTY slot:
///
/// ```text
/// 0x6c55  mov cx, es:[bp]             the destination's first word
/// 0x6c59  or cx,cx / jne 0x6c73       ALREADY OCCUPIED -> fail, write nothing
/// 0x6c5d  mov word es:[bp],0xc1       opcode
/// 0x6c63  mov ax,gs:[0x6736] / mov es:[bp+2],ax    the STORED operand
/// 0x6c6c  mov word es:[bp+4],2        aux word
/// ```
///
/// TWO THINGS SEPARATE THIS FROM THE C4..C8 WRITERS, per `vm_c1_write_record`: the
/// related word comes from the stored operand at `gs:0x6736` rather than from `bx`,
/// and the third word is `2`, not `0`.
///
/// `Some(None)` is the occupied case — a refusal to overwrite, which the caller must
/// not confuse with the `None` that means "not a kind-0x10 target at all"
/// (audit-fixes #598).
pub fn write_ship_3d_c1_kind10_destination_slot(
    target_record_offset: u16,
    target_kind_flags: u16,
    destination_slot: &mut Ship3dRecordStateSlot,
    operand_record_offset: u16,
) -> Option<Option<Ship3dC1DestinationWrite>> {
    let destination_record_offset =
        resolve_ship_3d_c1_kind10_destination_record(target_record_offset, target_kind_flags)?;
    if destination_slot.opcode != 0 {
        return Some(None);
    }

    *destination_slot = Ship3dRecordStateSlot {
        opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
        operand: operand_record_offset,
        aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
    };
    Some(Some(Ship3dC1DestinationWrite {
        destination_record_offset,
        slot: *destination_slot,
    }))
}

pub fn run_ship_3d_navigation_trigger_prelude(
    state: &mut Ship3dNavigationTriggerState,
    records: &[Ship3dNavigationRuntimeRecord],
    source_records: &[u16],
    honk_object: u16,
    ark_object: u16,
    pending_presentation_state: u16,
    layout_rect: [u16; SHIP_3D_INTERPOLATION_WORDS],
) -> Option<Ship3dNavigationTriggerEffect> {
    let mut effect = Ship3dNavigationTriggerEffect::default();
    if !state.trigger_active {
        return Some(effect);
    }

    state.requested_presentation_state = pending_presentation_state;
    effect.copied_pending_presentation_state = true;

    let current_record = find_ship_3d_navigation_record(records, state.current_target)?;
    effect.incremented_counter_record = Some(
        if current_record.kind_flags & SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG != 0 {
            current_record.counter_link
        } else {
            current_record.offset
        },
    );

    let candidate_records =
        build_ship_3d_navigation_candidate_records(source_records, records, honk_object)?;
    effect.candidate_records = candidate_records;

    let mut opened_target_list = true;
    for candidate_record_offset in &effect.candidate_records {
        let candidate_record = find_ship_3d_navigation_record(records, *candidate_record_offset)?;
        if current_record.state_flags & SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG == 0
            && candidate_record.related_target != state.current_target
        {
            continue;
        }

        if ark_object != state.current_target && candidate_record.related_target == ark_object {
            break;
        }

        effect.deferred_record_type = Some(SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE);
        effect.deferred_record_related = Some(*candidate_record_offset);
        effect.candidate_handler_record =
            Some(candidate_record_offset.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
        opened_target_list = false;
        break;
    }

    if opened_target_list {
        state.hud_flags |= SHIP_3D_NAVIGATION_TARGET_LIST_FLAG;
        state.interpolation_current_tick = 0;
        state.interpolation_duration_ticks = SHIP_3D_NAVIGATION_INTERPOLATION_DURATION;
        state.target_query_mode = false;
        state.layout_rect_snapshot[0] = layout_rect[0];
        state.layout_rect_snapshot[2] = layout_rect[2];
        effect.opened_target_list = true;
        effect.reset_interpolation_tick = true;
        effect.ran_layout_prepass = true;
        effect.copied_layout_x_and_width = true;
    }

    state.trigger_active = false;
    state.sequence_active = true;
    state.scene_band_top = SHIP_3D_NAVIGATION_SCENE_BAND_TOP;
    state.render_clip_top = 0;
    state.render_clip_bottom = SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM;
    state.active_dialogue_record = SHIP_3D_TARGET_EXIT_SENTINEL;
    state.closing = true;
    state.depth_step = SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP;
    effect.cleared_trigger = true;
    effect.started_sequence = true;
    effect.set_scene_band = true;
    effect.restored_render_clip = true;
    effect.cleared_active_dialogue_record = true;
    effect.requested_closing = true;

    Some(effect)
}

pub fn draw_ship_3d_target_list(
    state: &mut Ship3dTargetHitState,
    layout: Ship3dTargetListLayout,
    label_offsets: &[u16],
    width_table: &[u16],
    activate: bool,
    alias_source_offset: Option<u16>,
) -> Option<Ship3dTargetDrawResult> {
    let inner_width = layout
        .width
        .wrapping_sub(SHIP_3D_TARGET_LAYOUT_WIDTH_PADDING);
    let x_origin = layout.x.wrapping_add(SHIP_3D_TARGET_DRAW_X_INSET);
    let mut y = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
    let mut commands = Vec::new();

    for (row_index, label_offset) in label_offsets.iter().copied().enumerate() {
        if label_offset == 0 || label_offset == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::TargetList,
            string_offset: if Some(label_offset) == alias_source_offset {
                SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
            } else {
                label_offset
            },
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: false,
        });
        y = y.wrapping_add(SHIP_3D_TARGET_LAYOUT_ROW_STEP);
    }

    if layout.has_extra_entry {
        let row_index = commands.len();
        let measured_width = *width_table.get(row_index)?;
        commands.push(Ship3dTargetDrawCommand {
            row_index,
            string_segment: Ship3dTargetTextSegment::GameData,
            string_offset: SHIP_3D_TARGET_EXTRA_LABEL_OFFSET,
            x: target_list_draw_x(x_origin, inner_width, measured_width),
            y,
            color: next_target_list_draw_color(state, activate),
            measured_width,
            extra_entry: true,
        });
    }

    Some(Ship3dTargetDrawResult {
        commands,
        final_hover_counter: state.hover_row,
    })
}

pub fn select_ship_3d_target_record(
    state: &mut Ship3dTargetSelectorState,
    primary_targets: &[u16],
    fallback_targets: &[u16],
    query_index: u16,
    phase_gate_complete: bool,
) -> Option<Ship3dTargetSelection> {
    state.target_fallback = false;
    let mut targets = primary_targets;
    if primary_targets.first().copied() == Some(SHIP_3D_TARGET_EXIT_SENTINEL) {
        targets = fallback_targets;
        state.target_fallback = true;
    }
    let used_fallback_table = state.target_fallback;

    let mut ran_layout_prepass = false;
    if state.target_select_phase & 1 != 0 {
        ran_layout_prepass = true;
        state.target_animation_tick = 0;
        state.target_select_phase = state.target_select_phase.wrapping_add(1);
    }

    if state.target_select_phase & 2 != 0 {
        if !phase_gate_complete {
            return Some(Ship3dTargetSelection {
                selected_target: 0,
                used_fallback_table,
                ran_layout_prepass,
                phase_gate_blocked: true,
            });
        }
        state.target_select_phase = 0;
    }

    if query_index == SHIP_3D_TARGET_EXIT_SENTINEL {
        return Some(Ship3dTargetSelection {
            selected_target: 0,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let selected = targets.get(query_index as usize).copied()?;
    if selected == SHIP_3D_TARGET_EXIT_SENTINEL {
        state.opening = true;
        state.depth_step = SHIP_3D_TARGET_OPEN_STEP;
        return Some(Ship3dTargetSelection {
            selected_target: SHIP_3D_TARGET_EXIT_SENTINEL,
            used_fallback_table,
            ran_layout_prepass,
            phase_gate_blocked: false,
        });
    }

    let selected_target = if state.target_fallback {
        state.current_target
    } else {
        selected.wrapping_sub(SHIP_3D_TARGET_RECORD_HEADER_BYTES)
    };
    Some(Ship3dTargetSelection {
        selected_target,
        used_fallback_table,
        ran_layout_prepass,
        phase_gate_blocked: false,
    })
}

/// Bytes the plane band copy moves: `(depth + 35) * 80`, from `0xB71C` —
/// `mov ax,bx / add ax,0x23 / mov dl,0x50 / mul dl`. The add happens in 8 bits
/// (`mul dl` takes AL), which is why the port wraps the row count as a `u8`
/// before multiplying rather than widening first.
pub fn ship_3d_plane_band_byte_count(depth_offset: u16) -> usize {
    let rows = (depth_offset as u8).wrapping_add(SHIP_3D_PLANE_BASE_ROWS as u8) as usize;
    rows * SHIP_3D_PLANE_ROW_BYTES
}

/// The scroll value written to `DS:0x524F`, from `0xB6F7..0xB708`:
///
/// ```text
///   0x6F7  mov ax,bx / add ax,ax      2 * depth
///   0x6FB  cmp ax,0x64 / jle          clamp to 100
///   0x700  mov ax,0x64
///   0x703  sub ax,0x64 / neg ax       100 - that
/// ```
///
/// `sub` then `neg` rather than a reversed subtract, so the value is
/// `100 - min(2*depth, 100)` and reaches 0 exactly when the depth passes 50.
/// Scroll mode `0xA` skips this entirely (`cmp word [0x524d],0xa / je` @`0xB6F0`).
pub fn ship_3d_scroll_value(depth_offset: u16) -> u16 {
    let doubled = depth_offset.wrapping_mul(2);
    let capped = if (doubled as i16) > 0x64 {
        0x64
    } else {
        doubled
    };
    0x64u16.wrapping_sub(capped)
}

/// The armed-and-timer-exhausted branch of the transition updater, `0xB6B8`:
/// `mov byte [0x2531],8` (close step) / `mov byte [0x2530],1` (closing) /
/// `mov byte [0x2533],0` (disarmed) — three writes that always go together, which
/// is why they are one function here rather than three assignments at the call
/// site.
fn start_closing_transition(state: &mut Ship3dTransitionState) {
    state.depth_step = SHIP_3D_TRANSITION_CLOSE_STEP;
    state.closing = true;
    state.transition_armed = false;
}

/// Add to a word's LOW BYTE ONLY, leaving the high byte untouched — the effect of
/// the 8-bit `add`/`sub` the transition and interpolation code use on word-sized
/// state (e.g. `0x2531`'s step against `0x2527`'s depth). A 16-bit add would carry
/// into the high byte on wrap; the original cannot, so neither does this.
fn add_to_low_byte(value: u16, addend: u8) -> u16 {
    (value & 0xff00) | value.to_le_bytes()[0].wrapping_add(addend) as u16
}

fn circular_delta(first: u16, second: u16, modulus: u16) -> u16 {
    let (max, min) = if signed_i16(first) > signed_i16(second) {
        (first, second)
    } else {
        (second, first)
    };
    let delta = max.wrapping_sub(min);
    if signed_i16(delta) < signed_i16(modulus >> 1) {
        delta
    } else {
        modulus.wrapping_sub(delta)
    }
}

/// ONE conditional correction, not a modulo — the shape the angle code uses
/// everywhere, three times in `0x97AF`..`0x97DB` alone:
///
/// ```text
/// 0x97af  add dx,ax / cmp dx,0x168 / jl 0x97bb / sub dx,0x168   overflow: subtract once
/// 0x97c4  sub bx,0x1e / jns 0x97e1 / add bx,0x168               underflow: add once
/// 0x97d4  add bx,0x1e / cmp bx,0x168 / jl 0x97e1                overflow again
/// ```
///
/// `jl` and `jns` are the SIGNED forms, which is why this takes an `i32` and tests
/// `< 0` rather than relying on `u16` wraparound.
///
/// A SINGLE correction only works while the step is smaller than the modulus, and
/// the game relies on that: the steps here are `0x1E` and `0x28` against a `0x168`
/// ring. Feed it a larger delta and it lands outside the ring — `rem_euclid` would
/// not, which is exactly why this is not `rem_euclid` (audit-fixes #605).
fn wrap_ring_once(value: i32, modulus: u16) -> u16 {
    if value < 0 {
        value.wrapping_add(modulus as i32) as u16
    } else if value >= modulus as i32 {
        value.wrapping_sub(modulus as i32) as u16
    } else {
        value as u16
    }
}

/// Fetch a `(cosine, sine)` pair from the angle table and widen it to Q15.
///
/// The doubling is the game's: `0x990C` reads the angle word, `shl di,2` scales it
/// to the table's 4-byte stride, `movsx` widens each entry to 32 bits, and
/// `add ebx,ebx` / `add ecx,ecx` doubles them — so a Q14 table entry becomes a Q15
/// term for the matrix build. The port's `* 2` is that add, and the table itself
/// is `SHIP_3D_ANGLE_TABLE`, byte-exact against `DS:0x4F45`.
///
/// Returning `None` for an out-of-range angle is the port's own bound: the table
/// has 180 entries and the game indexes it after a modulus, so an unmasked angle
/// would read past it rather than wrapping.
fn matrix_pair_for_angle(angle_table: &[Ship3dAngleTableEntry], angle: u16) -> Option<(i32, i32)> {
    let entry = *angle_table.get(usize::from(angle))?;
    Some((
        i32::from(entry.cosine).wrapping_mul(2),
        i32::from(entry.sine).wrapping_mul(2),
    ))
}

/// The Q15 fixed-point multiply the matrix code uses: `imul` then
/// `sar eax,0xf` — e.g. `0x9957 imul eax,[si+0x14]` / `0x995F sar eax,0xf` in
/// `matrix3d_mul_fixed` (`0x994D`).
///
/// `sar`, not `shr`: an ARITHMETIC shift, so a negative product keeps its sign
/// rather than becoming a large positive. Rust's `>>` on `i32` is arithmetic, so
/// this matches — but only because the argument is typed `i32`; the same
/// expression on `u32` would be the wrong instruction.
fn fixed_mul_shift_15(lhs: i32, rhs: i32) -> i32 {
    lhs.wrapping_mul(rhs) >> SHIP_3D_MATRIX_FIXED_SHIFT
}

/// One component of the camera translation: `sub word [di],ax` at `0x9A42` (and
/// `[di+2],ax` at `0x9A47`), with the origin loaded from `[0x2F65]`/`[0x2F67]`
/// just before — a wrapping 16-bit SUBTRACT, sign-extended for the dot product.
fn projection_component(point_component: u16, origin_component: u16) -> i32 {
    i32::from(signed_i16(point_component.wrapping_sub(origin_component)))
}

/// The three-term dot product of the projection, accumulated in 32 bits WITHOUT
/// an intermediate shift — `0x9A50..0x9A66`: `imul eax,[bp+0x18]` / `mov ecx,eax`
/// / `imul eax,[bp+0x1c]` / `add ecx,eax`. The Q15 shift happens once on the
/// result (see [`fixed_mul_shift_15`]), not per term; shifting per term would
/// lose the low bits of each product before they are summed.
fn projection_dot(components: [i32; 3], terms: [i32; 3]) -> i32 {
    components[0]
        .wrapping_mul(terms[0])
        .wrapping_add(components[1].wrapping_mul(terms[1]))
        .wrapping_add(components[2].wrapping_mul(terms[2]))
}

/// The perspective divide, `0x9AD9..0x9AE2`:
///
/// ```text
///   0x9AD9  sar eax,7      pre-scale the dotted numerator
///   0x9ADD  cdq            sign-extend into edx:eax
///   0x9ADF  idiv ecx       divide by the DEPTH
///   0x9AE2  add ax,0x64    + the screen centre (100)
/// ```
///
/// `idiv` is signed and `cdq` is what makes it so — without the sign extension a
/// negative numerator would divide as a huge positive. The `sar eax,7` before it
/// is a pre-scale that keeps precision through the divide, and it is arithmetic
/// for the same reason.
///
/// The `+ 0x64` is the screen CENTRE, added after the divide, so the projection
/// produces an offset from centre rather than an absolute coordinate.
fn project_ship_3d_axis(numerator: i32, depth: i32, center: u16) -> u16 {
    let quotient = numerator / depth;
    (quotient as u16).wrapping_add(center)
}

/// Scale an object's sprite dimension by its depth factor, as the object-sprite
/// projector does at `0x9B98`: a 32-bit multiply
/// followed by `>> SHIP_3D_OBJECT_SCALE_SHIFT`. Widening to 32 bits BEFORE the
/// multiply is the point — the product of two words overflows 16 bits routinely,
/// and the original keeps it in `eax` for exactly that reason.
fn scale_ship_3d_object_dimension(dimension: u16, depth_scale: u16) -> u16 {
    (u32::from(dimension).wrapping_mul(u32::from(depth_scale)) >> SHIP_3D_OBJECT_SCALE_SHIFT) as u16
}

/// The slot-vs-dirty-rect overlap test inside `sprite_slot_dirty_range_render`,
/// `0x44F2`..`0x4504` — four REJECTIONS, so the accept is their conjunction:
///
/// ```text
/// 0x044d8  mov ax, [di+8]        slot left
/// 0x044db  mov bx, [di+0xa]      slot top
/// 0x044de  mov dx, ax
/// 0x044e0  add dx, [di+0xc]      slot right  = left + extent_width
/// 0x044e3  mov bp, bx
/// 0x044e5  add bp, [di+0xe]      slot bottom = top + extent_height
/// 0x044f2  cmp ax, [di+0x1a] / jge 0x450b    left   >= dirty right  -> skip
/// 0x044f7  cmp bx, [di+0x1e] / jge 0x450b    top    >= dirty bottom -> skip
/// 0x044fc  cmp dx, [di+0x18] / jle 0x450b    right  <= dirty left   -> skip
/// 0x04501  cmp bp, [di+0x1c] / jle 0x450b    bottom <= dirty top    -> skip
/// 0x04506  call word ptr cs:[0x15a2]         draw
/// ```
///
/// `jge`/`jle` are the SIGNED forms, which is the whole reason this reads its
/// coordinates through [`signed_i16`]: a slot projected off the left edge has a
/// negative `draw_x`, and an unsigned compare would place it at ~65000 and call it
/// far to the RIGHT of every dirty rect — never drawn instead of always clipped.
///
/// Note the dirty rect's field order in the record: `+0x18` left, `+0x1A` right,
/// `+0x1C` top, `+0x1E` bottom. Left/RIGHT/top/bottom, not left/top/right/bottom —
/// the pairs are per-axis, and reading it the conventional way swaps two of the
/// four bounds while still type-checking (audit-fixes #585).
fn ship_3d_rects_intersect(
    slot_rect: Ship3dProjectionViewport,
    dirty_rect: Ship3dProjectionViewport,
) -> bool {
    signed_i16(slot_rect.left) < signed_i16(dirty_rect.right)
        && signed_i16(slot_rect.top) < signed_i16(dirty_rect.bottom)
        && signed_i16(slot_rect.right) > signed_i16(dirty_rect.left)
        && signed_i16(slot_rect.bottom) > signed_i16(dirty_rect.top)
}

/// `idiv bl` modelled: AX divided by an 8-bit divisor, quotient in AL. The
/// `Option` covers the two cases the CPU TRAPS on — a zero divisor, and a
/// quotient too large for 8 bits — rather than wrapping them, so the port stops
/// where the game would fault instead of continuing with a different number.
/// Used by the interpolation gate at `0x1E74`.
fn checked_i16_div_i8_to_i8(dividend: i16, divisor: i8) -> Option<i8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as i16;
    i8::try_from(quotient).ok()
}

fn checked_u16_div_u8_to_u8(dividend: u16, divisor: u8) -> Option<u8> {
    if divisor == 0 {
        return None;
    }
    let quotient = dividend / divisor as u16;
    u8::try_from(quotient).ok()
}

/// Reinterpret a stored word as SIGNED. The game's coordinate and clip tests are
/// `jl`/`jge` (e.g. the plot's bounds at `0x9B0A`), so port comparisons must go
/// through this rather than comparing `u16`s — a value behind the camera is a
/// large unsigned number and a small negative one, and only the second is right.
fn signed_i16(value: u16) -> i16 {
    value as i16
}

fn target_list_draw_x(x_origin: u16, inner_width: u16, measured_width: u16) -> u16 {
    x_origin.wrapping_add(inner_width.wrapping_sub(measured_width) >> 1)
}

impl Ship3dNavChoiceGates {
    fn blocks_nav_choice(self) -> bool {
        self.c2_presentation_gate
            || self.left_motion_gate
            || self.right_motion_gate
            || self.menu_gate
            || self.sound_gate
            || self.presentation_active
    }
}

fn hit_test_ship_3d_nav_choice(
    dynamic_axis: u16,
    mouse_x: u16,
    mouse_y: u16,
) -> Option<Option<u8>> {
    let relative_axis = dynamic_axis.wrapping_sub(SHIP_3D_NAV_CHOICE_AXIS_BIAS);
    let right = SHIP_3D_NAV_CHOICE_RIGHT_BASE.wrapping_sub(relative_axis.wrapping_shl(3));
    if signed_i16(mouse_x) > signed_i16(right) {
        return Some(None);
    }

    let left = right.wrapping_sub(SHIP_3D_NAV_CHOICE_X_WIDTH);
    if signed_i16(left) < 0 || signed_i16(mouse_x) < signed_i16(left) {
        return Some(None);
    }

    let abs_axis = if signed_i16(relative_axis) < 0 {
        0u16.wrapping_sub(relative_axis)
    } else {
        relative_axis
    };
    let quarter_axis = abs_axis >> 2;
    let y_origin = SHIP_3D_NAV_CHOICE_Y_BASE
        .wrapping_add(abs_axis)
        .wrapping_add(quarter_axis);
    let row_height = SHIP_3D_NAV_CHOICE_ROW_HEIGHT_BASE.wrapping_sub((quarter_axis as u8) >> 1);
    let row_offset = mouse_y.wrapping_sub(y_origin);
    if signed_i16(row_offset) < 0 {
        return Some(None);
    }

    let choice = checked_u16_div_u8_to_u8(row_offset, row_height)?;
    if choice >= SHIP_3D_NAV_CHOICE_COUNT {
        return Some(None);
    }
    Some(Some(choice))
}

/// The `add ax,4` loop of nav-choice handler 1 (`0x8748..0x8754`): walk the word
/// list, add the 4-byte record header to each entry, stop at the `0xFFFF`
/// terminator (`cmp ax,-1 / je` @`0x8749`).
fn adjust_nav_choice_target_records(target_records: &mut [u16]) {
    for target_record in target_records {
        if *target_record == SHIP_3D_TARGET_EXIT_SENTINEL {
            break;
        }
        *target_record = target_record.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES);
    }
}

/// The rebuild loop of nav-choice handler 2 (`0x87CB..0x87DB`): walk the special
/// slots at `DS:0x6D3E`, skip zeros (`or ax,ax / je`), stop at the `0xFFFF`
/// sentinel after storing it, and otherwise store `slot + 4`. Returns `None` when
/// the slots run out with no sentinel — where the original would read past the
/// array, the port refuses rather than inventing a list.
fn rebuild_nav_choice_special_target_records(
    special_slots: &[u16],
    target_records: &mut Vec<u16>,
) -> Option<()> {
    target_records.clear();
    for special_slot in special_slots {
        if *special_slot == 0 {
            continue;
        }
        if *special_slot == SHIP_3D_TARGET_EXIT_SENTINEL {
            target_records.push(SHIP_3D_TARGET_EXIT_SENTINEL);
            return Some(());
        }
        target_records.push(special_slot.wrapping_add(SHIP_3D_TARGET_RECORD_HEADER_BYTES));
    }
    None
}

fn next_ship_3d_temp_snd_phase(phase: u8) -> u8 {
    let next = phase.wrapping_add(1);
    if next == SHIP_3D_TEMP_SND_PHASE_COUNT {
        0
    } else {
        next
    }
}

fn append_ship_3d_navigation_source_children(
    source_entries: &[Ship3dNavigationSourceEntry],
    records: &[Ship3dNavigationRuntimeRecord],
    parent_target: u16,
    source_records: &mut Vec<u16>,
) -> Option<()> {
    if source_entries.is_empty() {
        return None;
    }

    let mut index = 0;
    loop {
        let entry = source_entries.get(index)?;
        let record = find_ship_3d_navigation_record(records, entry.record_offset)?;
        if record.source_parent == Some(parent_target) {
            source_records.push(record.offset);
            append_ship_3d_navigation_source_children(
                source_entries,
                records,
                record.offset,
                source_records,
            )?;
        }

        index += 1;
        if source_entries.get(index).map(|entry| entry.entry_kind) != Some(1) {
            break;
        }
    }

    Some(())
}

fn find_ship_3d_navigation_record(
    records: &[Ship3dNavigationRuntimeRecord],
    offset: u16,
) -> Option<Ship3dNavigationRuntimeRecord> {
    records
        .iter()
        .copied()
        .find(|record| record.offset == offset)
}

fn find_ship_3d_position_record(
    records: &[Ship3dPositionRecord],
    offset: u16,
) -> Option<Ship3dPositionRecord> {
    records
        .iter()
        .copied()
        .find(|record| record.offset == offset)
}

fn find_ship_3d_position_field(
    fields: &[Ship3dPositionField],
    offset: u16,
) -> Option<Ship3dPositionField> {
    fields.iter().copied().find(|field| field.offset == offset)
}

fn resolve_ship_3d_distance_position_field(
    records: &[Ship3dPositionRecord],
    record: Ship3dPositionRecord,
    other_record: Ship3dPositionRecord,
    arche_object: u16,
    inherited_kind100_compare_word: u16,
) -> Option<u16> {
    if record.kind_flags == SHIP_3D_OBJECT_KIND_POSITION_KIND100 {
        return resolve_ship_3d_position_field(
            records,
            record.offset,
            arche_object,
            kind100_relation_word(other_record)?,
        );
    }

    resolve_ship_3d_position_field(
        records,
        record.offset,
        arche_object,
        inherited_kind100_compare_word,
    )
}

fn kind100_relation_word(record: Ship3dPositionRecord) -> Option<u16> {
    match vm::vm_field_offset(
        SHIP_3D_FIELD_SELECTOR_KIND100_RELATION_WORD,
        record.kind_flags,
    )? {
        0 => Some(record.kind_flags),
        _ => record.kind100_relation_word,
    }
}

fn ship_3d_record_field(record_offset: u16, kind_flags: u16, selector: u8) -> Option<u16> {
    vm::vm_field_offset(selector, kind_flags).map(|field| record_offset.wrapping_add(field))
}

/// Absolute difference of two words, as the distance helper computes it: a
/// wrapping 16-bit subtract, then negate if the SIGN BIT is set. That is a test of
/// bit 15 rather than a signed comparison, so a difference of exactly `0x8000`
/// negates to itself and stays `0x8000` — the one input where "absolute value"
/// has no representable answer, and the original does not special-case it either.
fn binary_abs_word_diff(first: u16, second: u16) -> u16 {
    let diff = first.wrapping_sub(second);
    if diff & 0x8000 != 0 {
        diff.wrapping_neg()
    } else {
        diff
    }
}

fn next_target_list_draw_color(state: &mut Ship3dTargetHitState, activate: bool) -> u8 {
    state.hover_row = state.hover_row.wrapping_sub(1);
    if state.hover_row == 0 {
        if activate {
            SHIP_3D_TARGET_ACTIVE_TEXT_COLOR
        } else {
            SHIP_3D_TARGET_HOVER_TEXT_COLOR
        }
    } else {
        SHIP_3D_TARGET_DEFAULT_TEXT_COLOR
    }
}

#[cfg(test)]
mod tests {

    /// `BloodPrng::default()` claims the shipped image's unseeded state. Check it,
    /// because #275 showed a defaulted zero can be a guess about an unwritten
    /// field rather than a value read from anywhere.
    #[test]
    fn prng_state_is_zero_in_the_shipped_image() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        // cs = 0x1CE for the PRNG (`0x1CE:0xB02` is `0x2DE2`), so base = 0x22E0.
        let base = 0x600 + 0x1CE * 16;
        assert_eq!(base + 0xB02, 0x2DE2, "the segment base does not place the PRNG");
        assert_eq!(
            &exe[base + 0xAEE..base + 0xAF3],
            &[0, 0, 0, 0, 0],
            "the shipped PRNG state is not zero, so the default is wrong"
        );

        let d = BloodPrng::default();
        assert_eq!((d.seed_word, d.a, d.b, d.counter), (0, 0, 0, 0));

        // And the seeder writes the RTC byte into BOTH halves (`mov ah,al`),
        // leaving the rest zero -- so a seeded PRNG differs from the default in
        // exactly one field.
        let seeded = BloodPrng::seeded_from_rtc_seconds(0x2A);
        assert_eq!(seeded.seed_word, 0x2A2A);
        assert_eq!((seeded.a, seeded.b, seeded.counter), (0, 0, 0));
    }

    /// THE APPROACH CAMERA'S ORIGIN, against the image and the two routines that
    /// set it. `origin_y` was 0 until #275; the phase-3 reset does not write that
    /// cell, so it keeps the 12000 the full reset established.
    #[test]
    fn the_approach_origin_matches_the_game() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let ds = 0xD420usize;
        let word = |off: usize| u16::from_le_bytes([exe[ds + off], exe[ds + off + 1]]);

        // The shipped values at DS:0x2F65/67/69.
        assert_eq!((word(0x2F65), word(0x2F67), word(0x2F69)), (10000, 12000, 0));

        // The full reset writes exactly those three (`c7 06 <disp> <imm>`).
        let mov_word = |at: usize| -> (u16, u16) {
            assert_eq!(&exe[at..at + 2], &[0xC7, 0x06], "{at:#x} is not mov word [mem],imm");
            (
                u16::from_le_bytes([exe[at + 2], exe[at + 3]]),
                u16::from_le_bytes([exe[at + 4], exe[at + 5]]),
            )
        };
        assert_eq!(mov_word(0x8CB4), (0x2F65, 10000));
        assert_eq!(mov_word(0x8CBA), (0x2F67, 12000));
        assert_eq!(mov_word(0x8CC0), (0x2F69, 0));

        // The PHASE-3 reset writes z and x but NOT y -- which is why the default
        // must carry 12000 rather than zero.
        assert_eq!(mov_word(0x8AF2), (0x2F69, 0x4E20));
        assert_eq!(mov_word(0x8AFE), (0x2F65, 0x2710));
        let phase3 = &exe[0x8AF2..0x8B04];
        assert!(
            !phase3.windows(2).any(|w| w == [0x67, 0x2F]),
            "the phase-3 reset does touch 0x2F67 after all"
        );

        let start = Ship3dCameraApproach::default();
        assert_eq!(start.origin_x, 0x2710);
        assert_eq!(start.origin_y, 0x2EE0, "Y must survive the phase-3 reset");
        assert_eq!(start.origin_z, 0x4E20);
    }

    /// THE TEMP-SND CALLBACK TABLE IS DATA, and it is in the image.
    ///
    /// `SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET` is `DS:0x0ACC`, so the three
    /// offsets the port carries as a literal array can be read straight out of
    /// `BLOODPRG.EXE` rather than trusted. They are, and the word after them is
    /// zero — which independently confirms the phase COUNT is 3 rather than the
    /// array simply being as long as someone transcribed.
    #[test]
    fn the_temp_snd_callback_offsets_are_the_images_own_words() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        const DS_BASE: usize = 0xD420;
        let at = DS_BASE + SHIP_3D_TEMP_SND_CALLBACK_TABLE_OFFSET as usize;

        for (index, expected) in SHIP_3D_TEMP_SND_CALLBACK_OFFSETS.iter().enumerate() {
            let word = u16::from_le_bytes([exe[at + index * 2], exe[at + index * 2 + 1]]);
            assert_eq!(
                word, *expected,
                "callback {index} is {word:#06x} in the image, {expected:#06x} in the port"
            );
        }

        // The entry PAST the table is zero: the count is the data's, not a
        // transcription choice.
        let n = SHIP_3D_TEMP_SND_CALLBACK_OFFSETS.len();
        let past = u16::from_le_bytes([exe[at + n * 2], exe[at + n * 2 + 1]]);
        assert_eq!(past, 0, "the table does not end where the port says it does");
        assert_eq!(
            n as u8, SHIP_3D_TEMP_SND_PHASE_COUNT,
            "the phase count and the table length disagree"
        );

        // The offsets are strictly increasing, as consecutive entry points must
        // be -- a transposed pair would still pass a set comparison.
        assert!(
            SHIP_3D_TEMP_SND_CALLBACK_OFFSETS.windows(2).all(|w| w[0] < w[1]),
            "callback offsets are not in ascending order"
        );
    }

    /// TWO ROUTINES MUST AGREE ABOUT THE ENTITY TABLE.
    ///
    /// `ship_3d_nav_entity_for_slot` builds an entity's record address as
    /// `0x6212 + (id << 5)` — table base, stride and count all decoded from the
    /// ship-3D projector. Somewhere else entirely, `engine.rs` decodes the nav
    /// hover panel reading ENTITY `0x1F`'s record directly at `si = 0x65F2`
    /// (`0x830A`), with no reference to the table base at all.
    ///
    /// Those are independent decodes of the same structure, so they have to line
    /// up: `0x6212 + 31*32 == 0x65F2`. If the base, the stride or the count were
    /// wrong, the last entity would not land where the other routine reads it.
    /// That makes this a real cross-check rather than the port agreeing with
    /// itself — four constants, two routines, one arithmetic identity.
    #[test]
    fn the_entity_table_base_stride_and_count_agree_with_the_hover_panel() {
        /// `si = 0x65F2` @`0x830A` — decoded in `engine.rs`, restated here only as
        /// the value this identity must reproduce.
        const HOVER_PANEL_LAST_ENTITY: u16 = 0x65F2;

        assert_eq!(
            SHIP_3D_ENTITY_TABLE + (SHIP_3D_ENTITY_COUNT - 1) * SHIP_3D_ENTITY_STRIDE,
            HOVER_PANEL_LAST_ENTITY,
            "the last of the {} entities does not land where 0x830A reads it",
            SHIP_3D_ENTITY_COUNT
        );

        // The nav slots occupy the tail of that table, starting at 0x15.
        let first = ship_3d_nav_entity_for_slot(0).expect("slot 0 exists");
        assert_eq!(first.0, SHIP_3D_NAV_ENTITY_BASE);
        assert_eq!(
            first.1,
            SHIP_3D_ENTITY_TABLE + SHIP_3D_NAV_ENTITY_BASE * SHIP_3D_ENTITY_STRIDE
        );

        // Every slot is distinct, in range, and stops at the table's end.
        let mut seen = std::collections::BTreeSet::new();
        let mut last_slot = 0usize;
        for slot in 0..64usize {
            match ship_3d_nav_entity_for_slot(slot) {
                Some((id, address)) => {
                    assert!(id < SHIP_3D_ENTITY_COUNT, "entity {id} past the table");
                    assert!(
                        address <= HOVER_PANEL_LAST_ENTITY,
                        "slot {slot} addresses {address:#x}, past the last entity"
                    );
                    assert!(seen.insert(address), "slot {slot} reuses an address");
                    last_slot = slot;
                }
                None => {
                    assert!(
                        SHIP_3D_NAV_ENTITY_BASE as usize + slot >= SHIP_3D_ENTITY_COUNT as usize,
                        "slot {slot} was rejected while still inside the table"
                    );
                }
            }
        }
        assert_eq!(
            last_slot + 1,
            (SHIP_3D_ENTITY_COUNT - SHIP_3D_NAV_ENTITY_BASE) as usize,
            "the nav slots do not fill the table's tail exactly"
        );
    }

    /// THE DIRTY-RECT COLLECTOR'S CONTRACT, `0x9B98`'s consumer
    /// (`collect_ship_3d_dirty_sprite_slot_render_commands`).
    ///
    /// Four decoded rules, each of which fails silently if broken:
    ///
    ///   * a command is emitted ONLY where the slot rect really intersects the
    ///     dirty rect — the filter's whole purpose;
    ///   * `dispatch_index` is `(flags >> 1) & 7` and `destination_remap_mode` is
    ///     `(flags >> 8) & 3`, so they are 3- and 2-bit fields. A wrong mask
    ///     produces an out-of-range dispatch index, which is the sprite version of
    ///     #224's handler-table overrun;
    ///   * an INACTIVE slot emits nothing;
    ///   * every slot in range has its DIRTY flag cleared, active or not — the
    ///     clear sits outside the active branch, which is easy to "tidy" inward.
    ///
    /// NOT GROUNDS FOR SETTLING, and deliberately so: this drives synthetic slots,
    /// not game data, so it verifies the port against the decoded rules rather
    /// than against the original. Per #219 the ledger rows stay as they are.
    #[test]
    fn the_dirty_rect_collector_honours_its_filter_and_field_widths() {
        let dirty = Ship3dDirtyRectList {
            rects: vec![
                Ship3dProjectionViewport { left: 50, top: 50, right: 100, bottom: 100 },
                Ship3dProjectionViewport { left: 200, top: 20, right: 260, bottom: 60 },
            ],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };

        let mut emitted = 0usize;
        for seed in 0..400u16 {
            let flags = seed.wrapping_mul(2477) | (seed & 1);
            let mut slots = vec![Ship3dObjectSpriteDescriptor {
                flags,
                draw_x: (seed.wrapping_mul(13)) % 300,
                draw_y: (seed.wrapping_mul(7)) % 190,
                extent_width: 8 + (seed % 40),
                extent_height: 8 + (seed % 30),
                ..Default::default()
            }];
            let commands =
                collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty, 0, 0);

            if flags & SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG == 0 {
                assert!(commands.is_empty(), "an inactive slot emitted {} command(s)", commands.len());
            }
            for command in &commands {
                assert!(
                    ship_3d_rects_intersect(command.slot_rect, command.dirty_rect),
                    "emitted a command for rects that do not intersect"
                );
                assert!(command.dispatch_index <= 7, "dispatch index is a 3-bit field");
                assert!(
                    command.destination_remap_mode <= 3,
                    "remap mode is a 2-bit field"
                );
                emitted += 1;
            }
            // The dirty flag is cleared for EVERY slot walked, active or not.
            assert_eq!(
                slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
                0,
                "the dirty flag survived the pass (seed {seed})"
            );
        }
        assert!(emitted > 20, "only {emitted} commands; the sweep proves little");

        // An empty rect list and an inverted range both produce nothing.
        let mut slots = vec![Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG,
            ..Default::default()
        }];
        let empty = Ship3dDirtyRectList { rects: Vec::new(), sentinel: SHIP_3D_DIRTY_RECT_SENTINEL };
        assert!(collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &empty, 0, 0).is_empty());
        assert!(collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty, 5, 0).is_empty());
    }

    /// THE DISPATCH FSM CANNOT SELECT A HANDLER THAT DOES NOT EXIST.
    ///
    /// `update_ship_3d_nav_choice_dispatch` (`0x0F29`'s caller) hit-tests a mouse
    /// position into a choice index that becomes an index into the FIVE-entry
    /// handler table at `CS:0x0F29` (`SHIP_3D_NAV_CHOICE_COUNT`). An index past
    /// the end reads a garbage far pointer and jumps into it — the FSM equivalent
    /// of the out-of-viewport write pinned in #223, and not something a plausible
    /// return value would reveal.
    ///
    /// So the sweep covers the input space rather than a few chosen points, and
    /// asserts three decoded rules:
    ///
    ///   * a blocking gate returns `gated` and NEVER a choice, whatever the mouse
    ///     is doing (`gates.blocks_nav_choice()` @`0x2565`);
    ///   * a `gate_value` outside `40..=60` never selects
    ///     (`SHIP_3D_NAV_CHOICE_MIN_GATE`/`MAX_GATE`);
    ///   * any selected choice is in `1..=5`, so the table index is in range.
    #[test]
    fn the_nav_choice_dispatch_never_selects_a_nonexistent_handler() {
        let mut selected = 0usize;
        let mut gated_seen = 0usize;
        // The box is anchored at AXIS_BIAS (45), so sweep AROUND it -- a sweep
        // from 0 with a coarse step never enters the hit box at all, which is how
        // the first run of this test scored zero selections.
        for axis in 30..70u16 {
            for mouse_x in (150..300u16).step_by(9) {
                for mouse_y in (60..200u16).step_by(3) {
                    for gate_value in [0u16, 39, 40, 50, 60, 61, 4000] {
                        let input = Ship3dNavChoiceInput {
                            gate_value,
                            dynamic_axis: axis,
                            mouse_x,
                            mouse_y,
                            activate: true,
                        };

                        // A blocking gate wins over everything.
                        let mut blocked_state = Ship3dNavChoiceState::default();
                        // Any one of the six gates blocks; pick the menu gate.
                        let blocking = Ship3dNavChoiceGates {
                            menu_gate: true,
                            ..Default::default()
                        };
                        if let Some(r) =
                            update_ship_3d_nav_choice_dispatch(&mut blocked_state, blocking, input)
                        {
                            assert!(r.gated, "a blocking gate did not report gated");
                            assert_eq!(
                                blocked_state.selected_choice, 0,
                                "a blocked dispatch still selected a choice"
                            );
                            gated_seen += 1;
                        }

                        // Ungated: any selection must be a real handler index.
                        let mut state = Ship3dNavChoiceState::default();
                        let gates = Ship3dNavChoiceGates::default();
                        let Some(_r) =
                            update_ship_3d_nav_choice_dispatch(&mut state, gates, input)
                        else {
                            continue;
                        };
                        // Both the HOVER and the DISPATCH index must be real
                        // handler indices; the hit-test bounds the first
                        // (`choice >= COUNT` @`0x8508`'s neighbour) and the
                        // dispatcher re-checks the second.
                        if let Some(hovered) = _r.hovered_choice {
                            assert!(
                                (1..=SHIP_3D_NAV_CHOICE_COUNT).contains(&hovered),
                                "hovered choice {hovered} is outside 1..={}",
                                SHIP_3D_NAV_CHOICE_COUNT
                            );
                            let palette = _r.highlighted_palette_index.expect("hover sets a colour");
                            assert!(
                                (SHIP_3D_NAV_CHOICE_PALETTE_FIRST
                                    ..SHIP_3D_NAV_CHOICE_PALETTE_FIRST
                                        + SHIP_3D_NAV_CHOICE_COUNT)
                                    .contains(&palette),
                                "palette index {palette:#x} is outside the choice bank"
                            );
                        }
                        if let Some(dispatched) = _r.dispatched_choice {
                            assert!(
                                (1..=SHIP_3D_NAV_CHOICE_COUNT).contains(&dispatched),
                                "dispatched choice {dispatched} would index past the \
                                 five-entry table at CS:0x0F29"
                            );
                        }
                        let choice = state.selected_choice;
                        if choice != 0 {
                            assert!(
                                (1..=u16::from(SHIP_3D_NAV_CHOICE_COUNT)).contains(&choice),
                                "choice {choice} is outside the {}-entry handler table \
                                 (axis {axis}, mouse {mouse_x},{mouse_y})",
                                SHIP_3D_NAV_CHOICE_COUNT
                            );
                            assert!(
                                (SHIP_3D_NAV_CHOICE_MIN_GATE..=SHIP_3D_NAV_CHOICE_MAX_GATE)
                                    .contains(&gate_value),
                                "gate_value {gate_value} is outside 40..=60 yet selected \
                                 choice {choice}"
                            );
                            selected += 1;
                        }
                    }
                }
            }
        }
        assert!(gated_seen > 0, "the blocking-gate path never ran");
        assert!(selected > 50, "only {selected} selections; the sweep proves little");
    }

    /// THE WHOLE PROJECTION CHAIN, ending at the pixel: the game's angle table
    /// builds a matrix, the matrix projects points, and `plot_ship_3d_projected_
    /// point` (`0x9B04`) clips and writes them. Two decoded rules are asserted at
    /// the end of it:
    ///
    ///   * a point outside the viewport writes NOTHING — not a wrapped pixel
    ///     somewhere else, which is what a missing sign check would produce;
    ///   * FIRST WRITE WINS (`mov al,es:[di] / or al,al / jne` @`0x9B30`), so a
    ///     second point at the same offset is rejected and the first shade stands.
    ///
    /// Provenance is transitive, as in #222: no file is opened here, but
    /// `SHIP_3D_ANGLE_TABLE` is verified byte-for-byte against `BLOODPRG.EXE` by
    /// `angle_table_matches_binary`, so the numbers driving this ARE the game's.
    #[test]
    fn the_projection_chain_never_writes_outside_the_viewport() {
        let viewport = Ship3dProjectionViewport {
            left: 0,
            top: 0,
            right: SHIP_3D_PROJECTION_SCREEN_WIDTH as u16,
            bottom: SHIP_3D_PROJECTION_SCREEN_HEIGHT as u16,
        };
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let size = SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT;

        let mut plotted = 0usize;
        for step in (0..180u16).step_by(2) {
            let Some(matrix) = build_ship_3d_projection_matrix(
                &SHIP_3D_ANGLE_TABLE,
                Ship3dMatrixAngles {
                    angle_2f71: step,
                    projection_angle_2f6d: (step * 5) % 180,
                    angle_2f6f: (step * 7) % 180,
                },
            ) else {
                continue;
            };
            let mut buffer = vec![0u8; size];
            for k in 0..64u16 {
                let point = Ship3dProjectionPoint {
                    x: k.wrapping_mul(97),
                    y: k.wrapping_mul(53),
                    z: k.wrapping_mul(31).wrapping_add(100),
                };
                let Some(projected) = project_ship_3d_point(point, origin, matrix) else {
                    continue;
                };
                let before = buffer.clone();
                match plot_ship_3d_projected_point(&mut buffer, viewport, projected) {
                    Some(pixel) => {
                        assert!(pixel.offset < size, "wrote past the buffer");
                        assert!(
                            (projected.x as usize) < SHIP_3D_PROJECTION_SCREEN_WIDTH
                                && (projected.y as usize) < SHIP_3D_PROJECTION_SCREEN_HEIGHT,
                            "accepted a point outside the viewport at ({}, {})",
                            projected.x,
                            projected.y
                        );
                        assert_eq!(
                            pixel.offset,
                            ship_3d_projected_point_offset(projected),
                            "the written offset is not the point's own"
                        );
                        // FIRST WRITE WINS: replaying the same point changes nothing.
                        let shade = buffer[pixel.offset];
                        assert_eq!(
                            plot_ship_3d_projected_point(&mut buffer, viewport, projected),
                            None,
                            "a second point at the same offset was accepted"
                        );
                        assert_eq!(buffer[pixel.offset], shade, "the first shade was overwritten");
                        plotted += 1;
                    }
                    None => assert!(
                        buffer == before,
                        "a rejected point still modified the buffer -- a coordinate wrapped"
                    ),
                }
            }
        }
        // A floor on COVERAGE, not a measurement: the assertions above are
        // vacuous if almost nothing reaches the plot stage. 96 points got through
        // at step 3, so the sweep was widened rather than the bar lowered.
        assert!(plotted > 100, "only {plotted} points plotted; the sweep proves little");
    }

    /// PROJECTED DEPTH CANNOT EXCEED THE DISTANCE, because the matrix row it is
    /// dotted with has unit length (#221). That is Cauchy-Schwarz, and it holds
    /// for the ORIGINAL too, so it checks the transcription rather than the port
    /// against itself.
    ///
    /// `project_ship_3d_point` (`0x2F65`) translates by the origin, dots with the
    /// matrix's third row for depth, culls `depth <= 0`, then divides the other
    /// two dots by it. A lost shift or a swapped row shows up here as a depth
    /// larger than the point can possibly be.
    #[test]
    fn projected_depth_never_exceeds_the_points_distance() {
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let point = Ship3dProjectionPoint { x: 300, y: 200, z: 500 };
        let distance =
            ((300f64).powi(2) + (200f64).powi(2) + (500f64).powi(2)).sqrt();

        let mut hits = 0usize;
        let mut best = 0f64;
        for step in 0..180u16 {
            let angles = Ship3dMatrixAngles {
                angle_2f71: step,
                projection_angle_2f6d: (step * 2) % 180,
                angle_2f6f: 0,
            };
            let Some(matrix) = build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles)
            else {
                continue;
            };
            let Some(projected): Option<Ship3dProjectedPoint> =
                project_ship_3d_point(point, origin, matrix)
            else {
                continue; // behind the viewer: culled by `depth <= 0`
            };
            let depth = projected.depth as f64;
            assert!(
                depth <= distance * 1.02,
                "angle {step}: depth {depth:.1} exceeds the distance {distance:.1} \
                 -- the view row is not unit length or a shift was lost"
            );
            best = best.max(depth);
            hits += 1;
        }

        assert!(hits > 20, "only {hits} angles projected; the sweep is too narrow");
        // NON-VACUOUS: some angle must look nearly straight at the point, or the
        // bound above would be satisfied by any small depth (including a broken
        // one that always returned 1).
        assert!(
            best > distance * 0.5,
            "best depth {best:.1} never approached the distance {distance:.1}"
        );
    }

    /// The origin itself has zero depth and is culled (`depth <= 0` @`0x2F65`).
    #[test]
    fn a_point_at_the_origin_is_culled() {
        let origin = Ship3dProjectionOrigin { x: 400, y: 400, z: 400 };
        let point = Ship3dProjectionPoint { x: 400, y: 400, z: 400 };
        let matrix = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles { angle_2f71: 0, projection_angle_2f6d: 0, angle_2f6f: 0 },
        )
        .expect("the identity-ish matrix builds");
        assert_eq!(
            project_ship_3d_point(point, origin, matrix),
            None,
            "translating a point onto the origin gives depth 0, which is culled"
        );
    }

    /// THE COMPOSED MATRIX MUST BE A ROTATION, checked with the game's own angle
    /// table (`SHIP_3D_ANGLE_TABLE`, itself verified against the binary by
    /// `angle_table_matches_binary`).
    ///
    /// `build_ship_3d_projection_matrix` (`0x2F95`) folds three angle pairs into
    /// nine fixed-point terms through a long chain of `imul`/`sar 15` with one
    /// deliberate `neg`-before-shift. Nothing in that chain announces an error: a
    /// swapped term or a wrong shift still produces plausible numbers.
    ///
    /// A rotation matrix has a property those numbers must satisfy anyway — each
    /// ROW has unit length. In this fixed point that is `sum(t^2) ~= (1<<15)^2`
    /// per row, and a transposed pair or a lost shift breaks it immediately. The
    /// tolerance is generous (1.5%) because every term is truncated by `sar`,
    /// which loses up to one unit per multiply.
    #[test]
    fn the_projection_matrix_is_a_rotation_at_every_table_angle() {
        const ONE: f64 = 32768.0;
        let mut checked = 0usize;
        // Sweep the table rather than one favourable angle: an error in a term
        // that only bites when a sine is negative would survive a single sample.
        for step in (0..180).step_by(7) {
            let angles = Ship3dMatrixAngles {
                angle_2f71: step,
                projection_angle_2f6d: (step * 2) % 180,
                angle_2f6f: (step * 3) % 180,
            };
            let Some(matrix): Option<Ship3dProjectionMatrix> =
                build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles)
            else {
                continue;
            };
            for row in 0..3 {
                let norm: f64 = (0..3)
                    .map(|c| {
                        let t = matrix.terms[row * 3 + c] as f64 / ONE;
                        t * t
                    })
                    .sum();
                assert!(
                    (norm - 1.0).abs() < 0.015,
                    "angle {step}: row {row} has length^2 {norm:.4}, not 1 -- the \
                     composed matrix is not a rotation"
                );
                checked += 1;
            }
        }
        assert!(checked >= 60, "the sweep covered {checked} rows");
    }

    /// LAYOUT AND HIT-TEST DRIVEN BY REAL GAME TEXT: the labels come out of
    /// `BLOODPRG.EXE`'s own string table, are measured with the port's font, laid
    /// out by `layout_ship_3d_target_list` (`0x84A1`) and then hit-tested row by
    /// row by `hit_test_ship_3d_target_list` (`0x84E6`).
    ///
    /// This is the round trip the shapes needed (audit-fixes #219 documented them
    /// but settled none, because documentation is not verification): every row's
    /// own centre must hit-test back to that row, and a point outside the box must
    /// hit nothing.
    #[test]
    fn real_game_labels_lay_out_and_hit_test_back_to_their_own_rows() {
        let binary = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"]
            .iter()
            .find_map(|p| crate::bloodprg::BloodPrg::parse_file(p).ok());
        let Some(binary) = binary else { return };
        let labels = binary.option_menu_labels();
        assert!(labels.len() >= 2, "the game's OPTION menu has rows");

        let widths: Vec<u16> = labels
            .iter()
            .map(|l| crate::font::square_caps_text_width(l) as u16)
            .collect();
        let layout: Ship3dTargetListLayout = layout_ship_3d_target_list(&widths, 0xE1, false);
        assert!(layout.width > 0 && layout.height > 0);

        // Each row's vertical centre, at the box's horizontal middle.
        let x = layout.x.wrapping_add(layout.width / 2);
        let top = layout.y.wrapping_add(SHIP_3D_TARGET_HIT_TEST_TOP_INSET);
        for row in 0..labels.len() {
            let y = top + (row as u16) * SHIP_3D_TARGET_LAYOUT_ROW_STEP + 1;
            let mut state = Ship3dTargetHitState::default();
            let hit: Ship3dTargetHitResult =
                hit_test_ship_3d_target_list(&mut state, layout, x, y, false)
                    .unwrap_or_else(|| panic!("row {row} at y={y} produced no result"));
            assert!(hit.inside, "row {row} reports outside the box");
            assert_eq!(
                hit.hover_row as usize,
                row + 1,
                "row {row} hit-tested to hover_row {} (rows are 1-based @0x84F6)",
                hit.hover_row
            );
            assert_eq!(hit.hover_row, state.hover_row, "result and state agree");
            assert!(!hit.activated, "no activation without the click flag");
        }

        // Left of the box hits nothing -- the `>= layout.x` gate.
        let mut outside = Ship3dTargetHitState::default();
        hit_test_ship_3d_target_list(&mut outside, layout, layout.x.wrapping_sub(4), top, false);
        assert_eq!(outside.hover_row, 0, "outside the box selects no row");

        // Above the first row hits nothing -- the row_offset >= 0 gate.
        let mut above = Ship3dTargetHitState::default();
        hit_test_ship_3d_target_list(&mut above, layout, x, layout.y, false);
        assert_eq!(above.hover_row, 0, "above the first row selects no row");
    }
    use super::*;

    #[test]
    fn projection_matches_the_decoded_native_routines() {
        // The native perspective projection (0x9aa4/0x9ad9, decoded sess 007):
        //   screen = (dot >> 7) idiv depth + centre;  centre_x = 0xA0, centre_y = 0x64.
        // Assert the ported constants match the disassembly exactly.
        assert_eq!(SHIP_3D_PROJECTION_AXIS_SHIFT, 7, "native sar eax,7");
        assert_eq!(SHIP_3D_PROJECTION_SCREEN_CENTER_X, 160, "native add ax,0xA0 (=160)");
        assert_eq!(SHIP_3D_PROJECTION_SCREEN_CENTER_Y, 100, "native add ax,0x64 (=100)");
        // project_ship_3d_axis reproduces `numerator idiv depth + centre` exactly.
        // e.g. numerator=(dot>>7)=1000, depth=8 -> 125 + 160 = 285.
        assert_eq!(project_ship_3d_axis(1000, 8, SHIP_3D_PROJECTION_SCREEN_CENTER_X), 285);
        // A point on the view axis (numerator 0) projects to the screen centre.
        assert_eq!(project_ship_3d_axis(0, 100, SHIP_3D_PROJECTION_SCREEN_CENTER_X), 160);
        assert_eq!(project_ship_3d_axis(0, 100, SHIP_3D_PROJECTION_SCREEN_CENTER_Y), 100);
    }

    #[test]
    fn camera_approach_walks_the_decoded_phase_machine() {
        let mut cam = Ship3dCameraApproach::default();
        assert_eq!((cam.phase, cam.origin_x, cam.origin_z), (1, 0x2710, 0x4E20));
        // Phase 1: X pulls in by 0x64/frame until below 0x2328, yaw rotates down.
        let mut steps = 0;
        while cam.phase == 1 && steps < 1000 {
            let prev_x = cam.origin_x;
            cam.step();
            if cam.phase == 1 {
                assert_eq!(cam.origin_x, prev_x - 0x64, "X decreases 0x64/frame");
            }
            steps += 1;
        }
        assert!(cam.origin_x < 0x2328, "phase 1 pulls X in below 0x2328");
        // 0x2710->0x2328 is 10 decrements to reach 0x2328 (still >=, so one more to
        // 0x2264), then the frame that sees X<0x2328 trips the phase: 12 steps.
        assert_eq!(steps, 12);
        // Phase 2: Z accelerates.
        let z_before = cam.origin_z;
        cam.step();
        assert!(cam.origin_z >= z_before && cam.z_accel == 0x64, "Z accelerates");
        while cam.phase == 2 {
            cam.step();
        }
        // Phase 3 reset then phase 4 sets Z=0x7530.
        assert_eq!(cam.phase, 3);
        cam.step();
        assert_eq!((cam.origin_x, cam.origin_z, cam.angle_2f71), (0x2710, 0x4E20, 0));
        cam.step(); // phase 4
        assert_eq!(cam.origin_z, 0x7530);
        cam.step();
        assert!(cam.done, "animation completes");
    }

    #[test]
    fn recovered_hud_pyramid_vertices_project_via_shared_projection() {
        // The recovered HUD geometry runs through the same projection as the ship
        // view / overlays. With the HUD entry angle (0xB3) at least some vertices
        // project to valid on-screen depths (>0), confirming the data + pipeline.
        assert_eq!(SHIP_3D_HUD_PYRAMID_VERTICES.len(), 32);
        let matrix = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 0,
            },
        )
        .expect("matrix");
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let projected = SHIP_3D_HUD_PYRAMID_VERTICES
            .iter()
            .filter_map(|v| {
                project_ship_3d_point(
                    Ship3dProjectionPoint {
                        x: v[0] as u16,
                        y: v[1] as u16,
                        z: v[2] as u16,
                    },
                    origin,
                    matrix,
                )
            })
            .count();
        assert!(projected > 0, "some HUD vertices must project on-screen");
    }


    #[test]
    fn star_map_projection_matches_decoded_formula() {
        // Identity-ish matrix: row_x=[1<<7,0,0], row_y=[0,1<<7,0], row_z=[0,0,1<<15].
        let m = Ship3dProjectionMatrix { terms: [128, 0, 0, 0, 128, 0, 0, 0, 32768] };
        let origin = [0, 0, 0];
        // pos (10, 20, 4): depth = (4*32768)>>15 = 4; sx = ((10*128)>>7)/4 + 160 = 10/4+160 = 162;
        // sy = ((20*128)>>7)/4 + 100 = 20/4+100 = 105; scale = 0x100000/4 = 0x40000.
        let (x, y, sc) = project_star_map_point([10, 20, 4], origin, &m).unwrap();
        assert_eq!((x, y, sc), (162, 105, 0x40000));
        // depth 0 -> culled
        assert!(project_star_map_point([1, 1, 0], origin, &m).is_none());
    }

    /// `0x9BE8`..`0x9BF9`: the camera subtract happens in 16 BITS and the result is
    /// sign-extended after. A separation past 32767 therefore comes out negative,
    /// and subtracting in `i32` — which the port used to do — gives a different
    /// point entirely (audit-fixes #586).
    #[test]
    fn the_camera_subtract_wraps_in_sixteen_bits() {
        let m = Ship3dProjectionMatrix { terms: [128, 0, 0, 0, 128, 0, 0, 0, 32768] };
        // z separation 40004 - 4 = 40000, which does NOT fit in i16: as a word it is
        // 0x9C40, sign-extended to -25536. Depth is therefore negative, and the
        // `depth += 0x10000` fixup @0x9C29 is what rescues it.
        let (_, _, scale) = project_star_map_point([10, 20, 40004], [0, 0, 4], &m)
            .expect("negative depth is fixed up, not culled");
        // 16-bit reading: depth = (-25536*32768)>>15 = -25536, +0x10000 -> 40000.
        assert_eq!(scale, 0x100000 / 40000, "the wrapped, sign-extended depth");
        // A 32-bit subtract would have given depth 40000 with NO fixup, and the
        // fixup only fires on a negative — so the two readings agree here by
        // arithmetic accident. The x axis is where they part company:
        let (x16, _, _) = project_star_map_point([40010, 20, 40004], [10, 0, 4], &m)
            .expect("projects");
        // 16-bit: 40000 -> -25536, so sx = ((-25536*128)>>7)/40000 + 160 = 160.
        assert_eq!(x16, 160 + (-25536i32) / 40000, "x from the WRAPPED delta");
        // 32-bit would have been 160 + 40000/40000 = 161. Different pixel.
        assert_ne!(x16, 161, "a 32-bit subtract would put this star a pixel right");
    }

    #[test]
    fn projected_navview_draws_perspective_grid_and_orb() {
        let mut fb = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
        render_star_map_navview_projected(&mut fb, 200, 90, 240, 90);
        assert!(fb.iter().any(|&p| p == 200), "lit pyramid faces");
        assert!(fb.iter().any(|&p| p == 90), "shadowed faces");
        assert!(fb.iter().any(|&p| p == 240), "orb");
        // heading pans the grid -> a different heading differs
        let mut fb2 = vec![0u8; fb.len()];
        render_star_map_navview_projected(&mut fb2, 200, 90, 240, 40);
        assert!(fb != fb2, "grid pans with heading");
    }


    #[test]
    fn pyramid_hud_draws_only_in_the_bottom_band() {
        let mut fb = vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
        render_ship_3d_pyramid_hud(&mut fb, 0x80, 0xFD);
        // Draws grid + orb pixels...
        assert!(fb.iter().any(|&p| p == 0x80), "pyramid grid drawn");
        assert!(fb.iter().any(|&p| p == 0xFD), "eye-orb drawn");
        // ...and NOTHING above the HUD band (the scene band stays untouched).
        let band_start = SHIP_3D_HUD_BAND_TOP * SHIP_3D_PROJECTION_SCREEN_WIDTH;
        assert!(
            fb[..band_start].iter().all(|&p| p == 0),
            "HUD render must not touch the scene band above row 165"
        );
    }

    fn source_segment() -> Vec<u8> {
        (0..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES)
            .map(|idx| (idx & 0xff) as u8)
            .collect()
    }

    #[test]
    fn transition_state_starts_opening_after_hold_threshold() {
        let mut state = Ship3dTransitionState {
            hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut state, true);
        assert_eq!(
            state,
            Ship3dTransitionState {
                hold_ticks: SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD,
                ..Ship3dTransitionState::default()
            }
        );

        state.hold_ticks = SHIP_3D_TRANSITION_OPEN_TIMER_THRESHOLD + 1;
        update_ship_3d_transition_state(&mut state, false);
        assert_eq!(state.depth_step, SHIP_3D_TRANSITION_OPEN_STEP);
        assert!(state.opening);
        assert!(state.transition_armed);
        assert!(!state.closing);
    }

    #[test]
    fn transition_state_starts_closing_when_armed_timer_expires_or_random_gate_hits() {
        let mut expired = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 0,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut expired, false);
        assert_eq!(expired.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(expired.closing);
        assert!(!expired.transition_armed);

        let mut gated = Ship3dTransitionState {
            transition_armed: true,
            hold_ticks: 1,
            ..Ship3dTransitionState::default()
        };
        update_ship_3d_transition_state(&mut gated, false);
        assert_eq!(
            gated,
            Ship3dTransitionState {
                transition_armed: true,
                hold_ticks: 1,
                ..Ship3dTransitionState::default()
            }
        );
        update_ship_3d_transition_state(&mut gated, true);
        assert_eq!(gated.depth_step, SHIP_3D_TRANSITION_CLOSE_STEP);
        assert!(gated.closing);
        assert!(!gated.transition_armed);
    }

    #[test]
    fn depth_scroll_opens_to_max_then_clears_opening_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 0x3c,
            opening: true,
            depth_step: SHIP_3D_TRANSITION_OPEN_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0x40);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(state.opening);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, SHIP_3D_MAX_DEPTH_OFFSET);
        assert!(!state.opening);
    }

    #[test]
    fn depth_scroll_closes_to_zero_then_clears_closing_flag() {
        let mut state = Ship3dDepthState {
            depth_offset: 5,
            closing: true,
            depth_step: SHIP_3D_TRANSITION_CLOSE_STEP,
            ..Ship3dDepthState::default()
        };

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(state.closing);

        step_ship_3d_depth_scroll(&mut state);
        assert_eq!(state.depth_offset, 0);
        assert!(!state.closing);
    }

    #[test]
    fn depth_scroll_uses_8086_low_byte_arithmetic() {
        let mut closing = Ship3dDepthState {
            depth_offset: 0x0101,
            closing: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut closing);
        assert_eq!(closing.depth_offset, 0x0100);

        let mut opening = Ship3dDepthState {
            depth_offset: 0xff00,
            opening: true,
            depth_step: 1,
            ..Ship3dDepthState::default()
        };
        step_ship_3d_depth_scroll(&mut opening);
        assert_eq!(opening.depth_offset, 0xff01);

        assert_eq!(
            ship_3d_plane_band_byte_count(0x0100),
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
    }

    #[test]
    fn plane_band_copy_uses_depth_plus_35_planar_rows() {
        let source = source_segment();
        let mut dest = vec![0xee; SHIP_3D_PLANE_DEST_BYTES];
        let copied =
            copy_ship_3d_plane_bands(&mut dest, &source, 0, true, 0).expect("ship 3D plane copy");

        assert_eq!(copied.row_count, SHIP_3D_PLANE_BASE_ROWS);
        assert_eq!(
            copied.byte_count,
            SHIP_3D_PLANE_BASE_ROWS * SHIP_3D_PLANE_ROW_BYTES
        );
        assert_eq!(
            copied.first_source_start,
            SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES - copied.byte_count
        );
        assert_eq!(
            &dest[0..copied.byte_count],
            &source[copied.first_source_start..copied.first_source_start + copied.byte_count]
        );
        assert_eq!(
            &dest[copied.second_dest_start..copied.second_dest_start + copied.byte_count],
            &source[copied.second_source_start..copied.second_source_start + copied.byte_count]
        );
        assert!(
            dest[copied.byte_count..copied.second_dest_start]
                .iter()
                .all(|value| *value == 0xee)
        );
        assert_eq!(copied.new_scroll_value, Some(0x64));
    }

    #[test]
    fn plane_band_copy_at_max_depth_copies_two_full_planar_pages() {
        let source = source_segment();
        let mut dest = vec![0; SHIP_3D_PLANE_DEST_BYTES];
        let copied = copy_ship_3d_plane_bands(
            &mut dest,
            &source,
            SHIP_3D_MAX_DEPTH_OFFSET,
            true,
            SHIP_3D_SCROLL_MODE_HOLD,
        )
        .expect("ship 3D plane copy");

        assert_eq!(copied.row_count, 100);
        assert_eq!(copied.byte_count, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(copied.first_source_start, SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET);
        assert_eq!(copied.second_dest_start, SHIP_3D_PLANE_PAGE_BYTES);
        assert_eq!(
            &dest[0..SHIP_3D_PLANE_PAGE_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE0_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(
            &dest[SHIP_3D_PLANE_PAGE_BYTES..SHIP_3D_PLANE_DEST_BYTES],
            &source[SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET
                ..SHIP_3D_PLANE_SOURCE_PAGE1_OFFSET + SHIP_3D_PLANE_PAGE_BYTES]
        );
        assert_eq!(copied.new_scroll_value, None);
    }

    #[test]
    fn plane_band_copy_reports_scroll_value_like_binary_math() {
        assert_eq!(ship_3d_scroll_value(0), 0x64);
        assert_eq!(ship_3d_scroll_value(30), 40);
        assert_eq!(ship_3d_scroll_value(50), 0);
        assert_eq!(ship_3d_scroll_value(SHIP_3D_MAX_DEPTH_OFFSET), 0);
        assert_eq!(copy_ship_3d_plane_bands(&mut [], &[], 0, false, 0), None);
    }

    #[test]
    fn target_selector_runs_phase_prepass_and_blocks_while_gate_is_active() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 1,
            target_animation_tick: 7,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(&mut state, &[0x1200], &[], 0, false).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                selected_target: 0,
                used_fallback_table: false,
                ran_layout_prepass: true,
                phase_gate_blocked: true,
            }
        );
        assert_eq!(state.target_select_phase, 2);
        assert_eq!(state.target_animation_tick, 0);
    }

    #[test]
    fn target_selector_returns_primary_target_after_phase_gate_completes() {
        let mut state = Ship3dTargetSelectorState {
            target_select_phase: 2,
            ..Ship3dTargetSelectorState::default()
        };

        let selected =
            select_ship_3d_target_record(&mut state, &[0x1200, 0x2345], &[], 1, true).unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                selected_target: 0x2341,
                used_fallback_table: false,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert_eq!(state.target_select_phase, 0);
        assert!(!state.opening);
    }

    #[test]
    fn target_selector_fallback_table_returns_current_target() {
        let mut state = Ship3dTargetSelectorState {
            current_target: 0x4567,
            ..Ship3dTargetSelectorState::default()
        };

        let selected = select_ship_3d_target_record(
            &mut state,
            &[SHIP_3D_TARGET_EXIT_SENTINEL],
            &[0x2222],
            0,
            true,
        )
        .unwrap();

        assert_eq!(
            selected,
            Ship3dTargetSelection {
                selected_target: 0x4567,
                used_fallback_table: true,
                ran_layout_prepass: false,
                phase_gate_blocked: false,
            }
        );
        assert!(state.target_fallback);
    }

    #[test]
    fn target_selector_exit_sentinel_arms_opening_transition() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[],
            1,
            true,
        )
        .unwrap();

        assert_eq!(selected.selected_target, SHIP_3D_TARGET_EXIT_SENTINEL);
        assert!(state.opening);
        assert_eq!(state.depth_step, SHIP_3D_TARGET_OPEN_STEP);
    }

    #[test]
    fn target_selector_no_query_selection_returns_zero() {
        let mut state = Ship3dTargetSelectorState::default();

        let selected = select_ship_3d_target_record(
            &mut state,
            &[0x1200],
            &[],
            SHIP_3D_TARGET_EXIT_SENTINEL,
            true,
        )
        .unwrap();

        assert_eq!(selected.selected_target, 0);
        assert!(!state.opening);
    }

    #[test]
    fn interpolation_gate_reports_complete_without_advancing_at_duration() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 6,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [10, 20, 30, 40], [0, 0, 0, 0]);

        assert_eq!(step, Some(Ship3dInterpolationStep::Complete));
        assert_eq!(gate.current_tick, 6);
    }

    #[test]
    fn interpolation_gate_increments_tick_and_interpolates_four_words() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 1,
        };

        let step = step_ship_3d_interpolation_gate(&mut gate, [60, 66, 72, 78], [0, 6, 12, 18]);

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([20, 26, 32, 38]))
        );
        assert_eq!(gate.current_tick, 2);
    }

    #[test]
    fn interpolation_gate_uses_signed_truncating_division() {
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: 6,
            current_tick: 2,
        };

        let step = step_ship_3d_interpolation_gate(
            &mut gate,
            [0xfff0, 0x0000, 0x0031, 0x0000],
            [0, 31, 0, 0],
        );

        assert_eq!(
            step,
            Some(Ship3dInterpolationStep::Active([
                0xfffa, // (-16 / 6) * 3 = -6, added to 0.
                0x0010, // (-31 / 6) * 3 = -15, added to 31.
                0x0018, // (49 / 6) * 3 = 24.
                0,
            ]))
        );
        assert_eq!(gate.current_tick, 3);
    }

    #[test]
    fn interpolation_gate_rejects_binary_idiv_error_shapes() {
        let mut zero_duration = Ship3dInterpolationGate {
            duration_ticks: 0,
            current_tick: 1,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(&mut zero_duration, [1, 0, 0, 0], [0, 0, 0, 0]),
            None
        );

        let mut quotient_overflow = Ship3dInterpolationGate {
            duration_ticks: 1,
            current_tick: 0,
        };
        assert_eq!(
            step_ship_3d_interpolation_gate(
                &mut quotient_overflow,
                [0x0100, 0, 0, 0],
                [0, 0, 0, 0]
            ),
            None
        );
    }

    #[test]
    fn target_list_layout_uses_binary_default_width_floor_and_centering() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 20,
                y: 85,
                width: 120,
                height: 30,
                max_label_width: SHIP_3D_TARGET_LAYOUT_DEFAULT_MAX_WIDTH,
                label_count: 2,
                has_extra_entry: false,
                selector_mode_return: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_grows_to_widest_label() {
        let layout = layout_ship_3d_target_list(&[120, 50], 0x50, false);

        assert_eq!(layout.max_label_width, 120);
        assert_eq!(layout.width, 140);
        assert_eq!(layout.x, 10);
        assert_eq!(layout.height, 30);
        assert_eq!(layout.y, 85);
    }

    #[test]
    fn target_list_layout_extra_entry_uses_shorter_width_and_height_seed() {
        let layout = layout_ship_3d_target_list(&[], 0x50, true);

        assert_eq!(
            layout,
            Ship3dTargetListLayout {
                x: 43,
                y: 91,
                width: 75,
                height: 18,
                max_label_width: SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH,
                label_count: 0,
                has_extra_entry: true,
                selector_mode_return: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            }
        );
    }

    #[test]
    fn target_list_layout_preserves_binary_wrapping_for_tall_lists() {
        let widths = vec![1; 20];
        let layout = layout_ship_3d_target_list(&widths, 0x50, false);

        assert_eq!(layout.height, 228);
        assert_eq!(layout.y, 0x7ff2);
    }

    #[test]
    fn target_hit_test_commits_selection_only_when_active() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let hover = hit_test_ship_3d_target_list(&mut state, layout, 30, 90, false).unwrap();
        assert_eq!(
            hover,
            Ship3dTargetHitResult {
                inside: true,
                activated: false,
                hover_row: 1,
                selected_row: 0,
                return_value: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 1);
        assert_eq!(state.selected_row, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_HOVER_PRESENTATION_MODE
        );

        let active = hit_test_ship_3d_target_list(&mut state, layout, 30, 101, true).unwrap();
        assert_eq!(
            active,
            Ship3dTargetHitResult {
                inside: true,
                activated: true,
                hover_row: 2,
                selected_row: 2,
                return_value: 1,
                play_select_sound: true,
            }
        );
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_ACTIVE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_uses_inclusive_x_and_exclusive_bottom_y() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET - 1,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x + layout.width + 1,
                layout.y + SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
        assert!(
            !hit_test_ship_3d_target_list(
                &mut state,
                layout,
                layout.x,
                layout.y + layout.height - SHIP_3D_TARGET_HIT_TEST_TOP_INSET,
                false,
            )
            .unwrap()
            .inside
        );
    }

    #[test]
    fn target_hit_test_clears_selection_then_requests_idle_when_outside() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            selected_row: 2,
            presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            requested_presentation_state: SHIP_3D_TARGET_HOVER_PRESENTATION_MODE,
            ..Ship3dTargetHitState::default()
        };

        let result = hit_test_ship_3d_target_list(&mut state, layout, 0, 0, false).unwrap();

        assert_eq!(
            result,
            Ship3dTargetHitResult {
                inside: false,
                activated: false,
                hover_row: 0,
                selected_row: 0,
                return_value: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
                play_select_sound: false,
            }
        );
        assert_eq!(state.hover_row, 0);
        assert_eq!(state.selected_row, 0);
        assert_eq!(state.presentation_state, 0);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_TARGET_IDLE_PRESENTATION_MODE
        );
    }

    #[test]
    fn target_hit_test_rejects_binary_div_overflow_shape() {
        let layout = Ship3dTargetListLayout {
            x: 0,
            y: 0,
            width: 1,
            height: 0x0b09,
            max_label_width: 1,
            label_count: 256,
            has_extra_entry: false,
            selector_mode_return: SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        };
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            hit_test_ship_3d_target_list(&mut state, layout, 0, 0x0b04, false),
            None
        );
    }

    #[test]
    fn target_draw_centers_rows_from_width_table_and_highlights_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL],
            &[20, 80],
            false,
            None,
        )
        .unwrap();

        assert_eq!(
            drawn.commands,
            vec![
                Ship3dTargetDrawCommand {
                    row_index: 0,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x1000,
                    x: 70,
                    y: 89,
                    color: SHIP_3D_TARGET_DEFAULT_TEXT_COLOR,
                    measured_width: 20,
                    extra_entry: false,
                },
                Ship3dTargetDrawCommand {
                    row_index: 1,
                    string_segment: Ship3dTargetTextSegment::TargetList,
                    string_offset: 0x2000,
                    x: 40,
                    y: 100,
                    color: SHIP_3D_TARGET_HOVER_TEXT_COLOR,
                    measured_width: 80,
                    extra_entry: false,
                },
            ]
        );
        assert_eq!(drawn.final_hover_counter, 0);
        assert_eq!(state.hover_row, 0);
    }

    #[test]
    fn target_draw_uses_active_color_and_keeps_decrementing_after_hover() {
        let layout = layout_ship_3d_target_list(&[20, 80, 40], 0x50, false);
        let mut state = Ship3dTargetHitState {
            hover_row: 1,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, 0x2000, 0x3000],
            &[20, 80, 40],
            true,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands[0].color, SHIP_3D_TARGET_ACTIVE_TEXT_COLOR);
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.commands[2].color, SHIP_3D_TARGET_DEFAULT_TEXT_COLOR);
        assert_eq!(drawn.final_hover_counter, 0xfe);
    }

    #[test]
    fn target_draw_stops_at_sentinel_then_draws_cancel_extra_entry() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, true);
        let mut state = Ship3dTargetHitState {
            hover_row: 2,
            ..Ship3dTargetHitState::default()
        };

        let drawn = draw_ship_3d_target_list(
            &mut state,
            layout,
            &[0x1000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000],
            &[20, SHIP_3D_TARGET_LAYOUT_EXTRA_WIDTH],
            false,
            None,
        )
        .unwrap();

        assert_eq!(drawn.commands.len(), 2);
        assert_eq!(drawn.commands[0].string_offset, 0x1000);
        assert_eq!(drawn.commands[1].row_index, 1);
        assert_eq!(
            drawn.commands[1].string_segment,
            Ship3dTargetTextSegment::GameData
        );
        assert_eq!(
            drawn.commands[1].string_offset,
            SHIP_3D_TARGET_EXTRA_LABEL_OFFSET
        );
        assert_eq!(drawn.commands[1].color, SHIP_3D_TARGET_HOVER_TEXT_COLOR);
        assert!(drawn.commands[1].extra_entry);
    }

    #[test]
    fn target_draw_applies_alias_blank_label_offset() {
        let layout = layout_ship_3d_target_list(&[20], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        let drawn =
            draw_ship_3d_target_list(&mut state, layout, &[0x4444], &[20], false, Some(0x4444))
                .unwrap();

        assert_eq!(
            drawn.commands[0].string_offset,
            SHIP_3D_TARGET_ALIAS_LABEL_OFFSET
        );
    }

    #[test]
    fn target_draw_requires_matching_width_table_entries() {
        let layout = layout_ship_3d_target_list(&[20, 80], 0x50, false);
        let mut state = Ship3dTargetHitState::default();

        assert_eq!(
            draw_ship_3d_target_list(&mut state, layout, &[0x1000, 0x2000], &[20], false, None),
            None
        );
    }

    #[test]
    fn nav_choice_hover_maps_mouse_to_palette_highlight() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 2,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: false,
                reset_palette_range: true,
                hovered_choice: Some(3),
                highlighted_palette_index: Some(SHIP_3D_NAV_CHOICE_PALETTE_FIRST + 2),
                committed_choice: None,
                dispatched_choice: None,
                play_select_sound: None,
            }
        );
        assert_eq!(state, Ship3dNavChoiceState::default());
    }

    #[test]
    fn nav_choice_activation_sets_binary_state_without_dispatching_yet() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MAX_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result.hovered_choice, Some(4));
        assert_eq!(result.committed_choice, Some(4));
        assert_eq!(result.dispatched_choice, None);
        assert_eq!(
            result.play_select_sound,
            Some(SHIP_3D_NAV_CHOICE_SELECT_SOUND)
        );
        assert_eq!(state.selected_choice, 4);
        assert_eq!(
            state.requested_presentation_state,
            SHIP_3D_NAV_CHOICE_PRESENTATION_MODE
        );
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_HUD_SELECT_FLAGS);
        assert_eq!(state.hold_ticks, SHIP_3D_NAV_CHOICE_HOLD_TICKS);
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_HANDLER_PHASE);
        assert_eq!(
            state.target_y,
            SHIP_3D_NAV_CHOICE_TARGET_Y_BASE + SHIP_3D_NAV_CHOICE_TARGET_Y_STEP * 3
        );
        assert!(state.target_layout_preserve_widths);
        assert_eq!(
            state.target_layout_center_x,
            SHIP_3D_NAV_CHOICE_LAYOUT_CENTER_X
        );
        assert!(state.target_layout_extra_entry);
        assert_eq!(
            state.interpolation_duration_ticks,
            SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION
        );
    }

    #[test]
    fn nav_choice_existing_selection_dispatches_after_hud_bit_clears() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: 0,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: 0,
                dynamic_axis: 0,
                mouse_x: 0,
                mouse_y: 0,
                activate: false,
            },
        )
        .unwrap();

        assert_eq!(result.reset_palette_range, false);
        assert_eq!(result.hovered_choice, None);
        assert_eq!(result.dispatched_choice, Some(2));
    }

    #[test]
    fn nav_choice_gates_block_hit_test_and_dispatch() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            ..Ship3dNavChoiceState::default()
        };

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates {
                presentation_active: true,
                ..Ship3dNavChoiceGates::default()
            },
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(
            result,
            Ship3dNavChoiceResult {
                gated: true,
                ..Ship3dNavChoiceResult::default()
            }
        );
        assert_eq!(state.selected_choice, 2);
    }

    #[test]
    fn nav_choice_rejects_out_of_range_gate_before_palette_reset() {
        let mut state = Ship3dNavChoiceState::default();

        let result = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE - 1,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS,
                mouse_x: 0x00c0,
                mouse_y: SHIP_3D_NAV_CHOICE_Y_BASE,
                activate: true,
            },
        )
        .unwrap();

        assert_eq!(result, Ship3dNavChoiceResult::default());
        assert_eq!(state.selected_choice, 0);
    }

    #[test]
    fn nav_choice_uses_dynamic_axis_for_slanted_bounds() {
        let mut state = Ship3dNavChoiceState::default();

        let outside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0090,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(outside.reset_palette_range, true);
        assert_eq!(outside.hovered_choice, None);

        let inside = update_ship_3d_nav_choice_dispatch(
            &mut state,
            Ship3dNavChoiceGates::default(),
            Ship3dNavChoiceInput {
                gate_value: SHIP_3D_NAV_CHOICE_MIN_GATE,
                dynamic_axis: SHIP_3D_NAV_CHOICE_AXIS_BIAS + 4,
                mouse_x: 0x0091,
                mouse_y: 0x004d,
                activate: false,
            },
        )
        .unwrap();
        assert_eq!(inside.hovered_choice, Some(1));
    }

    #[test]
    fn nav_choice_handler_0_defers_honk_record_link_and_clears_phase() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_duration_ticks: SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6754),
                cleared_handler_phase: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
    }

    #[test]
    fn nav_choice_handler_0_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: 0x02,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_0(&mut state, 0x6754);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, 0x02);
    }

    #[test]
    fn nav_choice_handler_1_adjusts_records_and_waits_for_interpolation() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 3,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1000, 0x2000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL, 0x3000]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                adjusted_target_records: true,
                phase_gate_blocked: true,
                reset_interpolation_tick: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_1_selects_target_after_interpolation() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, 0x2004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x2000),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_exit_sentinel_clears_choice_without_deferred_record() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_1(&mut state, &mut target_records, true, 1).unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_1_no_selection_leaves_state_armed() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 2,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = [0x1004, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect = run_ship_3d_nav_choice_handler_1(
            &mut state,
            &mut target_records,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.selected_choice, 2);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
    }

    #[test]
    fn nav_choice_handler_2_rebuilds_targets_from_special_slots_and_waits() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 7,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa, 0xbbbb];

        let effect = run_ship_3d_nav_choice_handler_2(
            &mut state,
            &[0, 0x1200, 0, 0x3400, SHIP_3D_TARGET_EXIT_SENTINEL, 0x5600],
            &mut target_records,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        )
        .unwrap();

        assert_eq!(
            target_records,
            vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL]
        );
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                rebuilt_target_records: true,
                reset_interpolation_tick: true,
                phase_gate_blocked: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_2_selects_special_slot_target_and_sets_input_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, 0x3404, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_related: Some(0x3400),
                cleared_handler_phase: true,
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                set_input_gate_b: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_exit_sentinel_clears_choice_without_gate() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 3,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0x1204, SHIP_3D_TARGET_EXIT_SENTINEL];

        let effect =
            run_ship_3d_nav_choice_handler_2(&mut state, &[], &mut target_records, true, 1)
                .unwrap();

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_2_requires_special_slot_sentinel_when_rebuilding() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            ..Ship3dNavChoiceState::default()
        };
        let mut target_records = vec![0xaaaa];

        assert_eq!(
            run_ship_3d_nav_choice_handler_2(
                &mut state,
                &[0, 0x1200],
                &mut target_records,
                false,
                SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
            ),
            None
        );
        assert_eq!(target_records, vec![0x1204]);
    }

    #[test]
    fn nav_choice_handler_3_defers_static_record_link_and_reloads_radio_bank() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 4,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 8,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                deferred_record_type: Some(SHIP_3D_NAV_CHOICE_RECORD_LINK_TYPE),
                deferred_record_related: Some(0x6756),
                cleared_handler_phase: true,
                load_snd_bank_path: Some(SHIP_3D_NAV_CHOICE_RADIO_SND_PATH_OFFSET),
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 4);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
        assert_eq!(state.interpolation_current_tick, 8);
    }

    #[test]
    fn nav_choice_handler_3_returns_without_phase_bit() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };

        let effect = run_ship_3d_nav_choice_handler_3(&mut state, 0x6756);

        assert_eq!(effect, Ship3dNavChoiceHandlerEffect::default());
        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
    }

    #[test]
    fn nav_choice_handler_4_runs_layout_snapshot_and_waits_for_interpolation() {
        let mut state = Ship3dNavChoiceState {
            handler_phase: SHIP_3D_NAV_CHOICE_HANDLER_PHASE,
            interpolation_current_tick: 9,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();
        let layout_rect = [0x10, 0x20, 0x30, 0x40];

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            layout_rect,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(state.handler_phase, SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING);
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(handler_state.layout_rect_snapshot, layout_rect);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                ran_layout_prepass: true,
                copied_layout_rect_snapshot: true,
                reset_interpolation_tick: true,
                phase_gate_blocked: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_4_no_selection_leaves_choice_armed_after_phase_clear() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            handler_phase: SHIP_3D_NAV_CHOICE_PHASE_INTERPOLATING,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_handler_phase: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.handler_phase, 0);
        assert_eq!(state.selected_choice, 5);
        assert_eq!(state.hud_flags, SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG);
    }

    #[test]
    fn nav_choice_handler_4_menu_choice_sets_both_menu_gates_and_clears_choice() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State::default();

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            0,
        );

        assert!(handler_state.menu_gate);
        assert!(handler_state.secondary_menu_gate);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn nav_choice_handler_4_voc_choice_toggles_tablo2_playback() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State {
            voc_enabled: true,
            tablo2_voc_reset_gate: true,
            ..Ship3dNavChoiceHandler4State::default()
        };

        let start_effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            1,
        );

        assert_eq!(handler_state.voc_stream_phase, 0);
        assert!(handler_state.tablo2_voc_active);
        assert!(!handler_state.tablo2_voc_reset_gate);
        assert_eq!(
            handler_state.active_target_list_offset,
            SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_ON_TARGET_LIST_OFFSET
        );
        assert_eq!(
            start_effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                load_voc_path: Some(SHIP_3D_NAV_CHOICE_TABLO2_VOC_PATH_OFFSET),
                start_voc_playback: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );

        state.selected_choice = 5;
        state.hud_flags = SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG;
        let stop_effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            1,
        );

        assert_eq!(handler_state.voc_stream_phase, 0);
        assert!(!handler_state.tablo2_voc_active);
        assert_eq!(
            handler_state.active_target_list_offset,
            SHIP_3D_NAV_CHOICE_HANDLER4_TOGGLE_OFF_TARGET_LIST_OFFSET
        );
        assert_eq!(
            stop_effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
    }

    #[test]
    fn nav_choice_handler_4_motion_choices_set_left_and_right_gates() {
        let mut left_state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut left_handler_state = Ship3dNavChoiceHandler4State::default();

        let left_effect = run_ship_3d_nav_choice_handler_4(
            &mut left_state,
            &mut left_handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            2,
        );

        assert!(left_handler_state.shared_motion_gate);
        assert!(left_handler_state.left_motion_gate);
        assert!(!left_handler_state.right_motion_gate);
        assert!(left_effect.cleared_selected_choice);
        assert!(left_effect.cleared_hud_target_list_flag);

        let mut right_state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut right_handler_state = Ship3dNavChoiceHandler4State::default();

        let right_effect = run_ship_3d_nav_choice_handler_4(
            &mut right_state,
            &mut right_handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            3,
        );

        assert!(right_handler_state.shared_motion_gate);
        assert!(!right_handler_state.left_motion_gate);
        assert!(right_handler_state.right_motion_gate);
        assert!(right_effect.cleared_selected_choice);
        assert!(right_effect.cleared_hud_target_list_flag);
    }

    #[test]
    fn nav_choice_handler_4_sound_choice_blocks_dispatch_and_clears_activation() {
        let mut state = Ship3dNavChoiceState {
            selected_choice: 5,
            hud_flags: SHIP_3D_NAV_CHOICE_TARGET_LIST_FLAG,
            ..Ship3dNavChoiceState::default()
        };
        let mut handler_state = Ship3dNavChoiceHandler4State {
            target_activate_flag: true,
            target_activate_secondary_flag: true,
            ..Ship3dNavChoiceHandler4State::default()
        };

        let effect = run_ship_3d_nav_choice_handler_4(
            &mut state,
            &mut handler_state,
            [0; SHIP_3D_INTERPOLATION_WORDS],
            true,
            4,
        );

        assert_eq!(
            handler_state.sound_gate,
            SHIP_3D_NAV_CHOICE_SOUND_GATE_SUPPRESS_TARGETS
        );
        assert!(!handler_state.target_activate_flag);
        assert!(!handler_state.target_activate_secondary_flag);
        assert_eq!(
            effect,
            Ship3dNavChoiceHandlerEffect {
                cleared_selected_choice: true,
                cleared_hud_target_list_flag: true,
                ..Ship3dNavChoiceHandlerEffect::default()
            }
        );
        assert_eq!(state.selected_choice, 0);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn procedural_update_rotates_active_hud_toward_hold_angle() {
        let mut state = Ship3dProceduralUpdateState {
            hud_flags: SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG,
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING,
            mouse_y: 0x0064,
            hold_ticks: 100,
            nav_timer: 0,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                initialized_nav_timer: true,
                applied_hud_rotation: true,
                updated_projection_angle: true,
                mouse_set_position: Some((0x0640, 0x0064)),
                carry_set: true,
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 30);
        assert_eq!(state.nav_timer, 40);
        assert_eq!(state.mouse_delta_accumulator, 160);
        assert_eq!(state.mouse_button_state, 0);
        assert!(state.rotation_direction_positive);
        assert_eq!(state.projection_angle, 30);
        assert_eq!(state.rotation_offset, 80);
        assert_eq!(state.mouse_x, 80);
        assert_eq!(state.mouse_sector, 40);
    }

    #[test]
    fn procedural_update_auto_rotates_angle_when_hud_inactive() {
        let mut state = Ship3dProceduralUpdateState {
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x01e0,
            mouse_y: 0x0070,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                auto_rotated_angle: true,
                updated_projection_angle: true,
                mouse_set_position: Some((0x0780, 0x0070)),
                carry_set: true,
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 45);
        assert_eq!(state.projection_angle, 45);
        assert_eq!(state.rotation_offset, 200);
        assert_eq!(state.mouse_x, 280);
        assert_eq!(state.mouse_sector, 120);
        assert!(state.rotation_direction_positive);
    }

    #[test]
    fn procedural_update_target_list_flag_adjusts_mouse_without_rotating_angle() {
        let mut state = Ship3dProceduralUpdateState {
            hud_flags: SHIP_3D_PROCEDURAL_TARGET_LIST_FLAG,
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x01e0,
            mouse_y: 0x0080,
            projection_angle: 77,
            rotation_offset: 0x0020,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                adjusted_target_list_mouse: true,
                mouse_set_position: Some((0x0690, 0x0080)),
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 10);
        assert_eq!(state.projection_angle, 77);
        assert_eq!(state.rotation_offset, 0x0020);
        assert_eq!(state.mouse_x, 208);
        assert_eq!(state.mouse_sector, 120);
    }

    #[test]
    fn procedural_update_close_angle_only_applies_existing_rotation_offset() {
        let mut state = Ship3dProceduralUpdateState {
            angle: 10,
            mouse_x: SHIP_3D_PROCEDURAL_MOUSE_RING + 0x0078,
            mouse_y: 0x0090,
            projection_angle: 66,
            rotation_offset: 0x0010,
            ..Ship3dProceduralUpdateState::default()
        };

        let effect = run_ship_3d_procedural_update(&mut state);

        assert_eq!(
            effect,
            Ship3dProceduralUpdateEffect {
                mouse_set_position: Some((0x0618, 0x0090)),
                ..Ship3dProceduralUpdateEffect::default()
            }
        );
        assert_eq!(state.angle, 10);
        assert_eq!(state.projection_angle, 66);
        assert_eq!(state.rotation_offset, 0x0010);
        assert_eq!(state.mouse_x, 104);
        assert_eq!(state.mouse_sector, 30);
    }

    #[test]
    fn projection_matrix_builds_basis_orientation() {
        let angle_table = [
            Ship3dAngleTableEntry {
                cosine: 0x4000,
                sine: 0,
            },
            Ship3dAngleTableEntry {
                cosine: 0,
                sine: 0x4000,
            },
        ];

        let matrix = build_ship_3d_projection_matrix(
            &angle_table,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 0,
            },
        )
        .unwrap();

        assert_eq!(
            matrix,
            Ship3dProjectionMatrix {
                terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000]
            }
        );
    }

    #[test]
    fn projection_matrix_preserves_binary_fixed_point_operation_order() {
        let angle_table = [
            Ship3dAngleTableEntry {
                cosine: 0x4000,
                sine: 0,
            },
            Ship3dAngleTableEntry {
                cosine: 0,
                sine: 0x4000,
            },
            Ship3dAngleTableEntry {
                cosine: 0x2000,
                sine: 0x2000,
            },
        ];

        let matrix = build_ship_3d_projection_matrix(
            &angle_table,
            Ship3dMatrixAngles {
                angle_2f71: 1,
                projection_angle_2f6d: 2,
                angle_2f6f: 0,
            },
        )
        .unwrap();

        assert_eq!(
            matrix.terms,
            [0, -32768, 0, -16384, 0, 16384, 16384, 0, 16384]
        );
    }

    #[test]
    fn projection_matrix_rejects_missing_angle_table_entry() {
        let angle_table = [Ship3dAngleTableEntry {
            cosine: 0x4000,
            sine: 0,
        }];

        assert_eq!(
            build_ship_3d_projection_matrix(
                &angle_table,
                Ship3dMatrixAngles {
                    angle_2f71: 0,
                    projection_angle_2f6d: 1,
                    angle_2f6f: 0,
                },
            ),
            None
        );
    }

    #[test]
    fn projection_point_uses_matrix_depth_and_screen_centers() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        let projected = project_ship_3d_point(
            Ship3dProjectionPoint {
                x: 10,
                y: 20,
                z: 1000,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
        )
        .unwrap();

        assert_eq!(
            projected,
            Ship3dProjectedPoint {
                x: 162,
                y: 95,
                depth: 1000,
            }
        );
    }

    #[test]
    fn projection_point_rejects_zero_and_negative_depth_like_binary_branch() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        assert_eq!(
            project_ship_3d_point(
                Ship3dProjectionPoint { x: 0, y: 0, z: 0 },
                Ship3dProjectionOrigin::default(),
                matrix,
            ),
            None
        );
        assert_eq!(
            project_ship_3d_point(
                Ship3dProjectionPoint {
                    x: 0,
                    y: 0,
                    z: 0xffff,
                },
                Ship3dProjectionOrigin::default(),
                matrix,
            ),
            None
        );
    }

    #[test]
    fn projection_point_subtracts_origin_as_wrapping_words_before_sign_extend() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };

        let projected = project_ship_3d_point(
            Ship3dProjectionPoint {
                x: 0,
                y: 0,
                z: 1000,
            },
            Ship3dProjectionOrigin {
                x: 0xfc18,
                y: 0,
                z: 0,
            },
            matrix,
        )
        .unwrap();

        assert_eq!(projected.x, 416);
        assert_eq!(projected.y, 100);
        assert_eq!(projected.depth, 1000);
    }

    #[test]
    fn projection_plot_clips_occupied_pixels_and_writes_depth_shade() {
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: 320,
            top: 0,
            bottom: 200,
        };
        let projected = Ship3dProjectedPoint {
            x: 10,
            y: 2,
            depth: 0x3000,
        };
        let mut depth_buffer = vec![0; SHIP_3D_PROJECTION_SCREEN_WIDTH * 200];

        let pixel = plot_ship_3d_projected_point(&mut depth_buffer, viewport, projected).unwrap();

        assert_eq!(
            pixel,
            Ship3dProjectedPixel {
                offset: 650,
                shade: 0xec,
            }
        );
        assert_eq!(depth_buffer[650], 0xec);

        assert_eq!(
            plot_ship_3d_projected_point(&mut depth_buffer, viewport, projected),
            None
        );
        assert_eq!(depth_buffer[650], 0xec);

        assert_eq!(
            plot_ship_3d_projected_point(
                &mut depth_buffer,
                viewport,
                Ship3dProjectedPoint {
                    x: 320,
                    y: 2,
                    depth: 0x3000,
                },
            ),
            None
        );
    }

    #[test]
    fn object_sprite_projection_scales_and_centers_visible_descriptor() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        let projection = project_ship_3d_object_sprite(
            Ship3dProjectionPoint {
                x: 10,
                y: 20,
                z: 1000,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
            &mut descriptor,
        )
        .unwrap();

        assert_eq!(
            projection,
            Ship3dObjectSpriteProjection {
                projected: Ship3dProjectedPoint {
                    x: 162,
                    y: 95,
                    depth: 1000,
                },
                depth_scale: 1048,
                scaled_width: 65,
                scaled_height: 32,
                draw_x: 130,
                draw_y: 79,
            }
        );
        assert_eq!(
            descriptor,
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_OBJECT_VISIBLE_FLAG
                    | SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG,
                source_width: 64,
                source_height: 32,
                draw_x: 130,
                draw_y: 79,
                extent_width: 65,
                extent_height: 32,
                ..Ship3dObjectSpriteDescriptor::default()
            }
        );
    }

    #[test]
    fn object_sprite_projection_skips_hidden_descriptor_and_zero_depth() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: 0,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            project_ship_3d_object_sprite(
                Ship3dProjectionPoint {
                    x: 10,
                    y: 20,
                    z: 1000,
                },
                Ship3dProjectionOrigin::default(),
                matrix,
                &mut descriptor,
            ),
            None
        );
        descriptor.flags = SHIP_3D_OBJECT_VISIBLE_FLAG;
        assert_eq!(
            project_ship_3d_object_sprite(
                Ship3dProjectionPoint { x: 0, y: 0, z: 0 },
                Ship3dProjectionOrigin::default(),
                matrix,
                &mut descriptor,
            ),
            None
        );
    }

    #[test]
    fn object_sprite_projection_wraps_negative_depth_before_scaling() {
        let matrix = Ship3dProjectionMatrix {
            terms: [0x8000, 0, 0, 0, -0x8000, 0, 0, 0, 0x8000],
        };
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG,
            source_width: 64,
            source_height: 32,
            draw_x: 0,
            draw_y: 0,
            extent_width: 64,
            extent_height: 32,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        let projection = project_ship_3d_object_sprite(
            Ship3dProjectionPoint {
                x: 0,
                y: 0,
                z: 0xffff,
            },
            Ship3dProjectionOrigin::default(),
            matrix,
            &mut descriptor,
        )
        .unwrap();

        assert_eq!(
            projection,
            Ship3dObjectSpriteProjection {
                projected: Ship3dProjectedPoint {
                    x: 160,
                    y: 100,
                    depth: 0xffff,
                },
                depth_scale: 16,
                scaled_width: 1,
                scaled_height: 0,
                draw_x: 160,
                draw_y: 100,
            }
        );
        assert_eq!(descriptor.extent_width, 1);
        assert_eq!(descriptor.extent_height, 0);
        assert_eq!(descriptor.draw_x, 160);
        assert_eq!(descriptor.draw_y, 100);
        assert_eq!(
            descriptor.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG
                | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG
        );
    }

    #[test]
    fn sprite_slot_position_update_marks_dirty_only_when_active_and_changed() {
        let mut inactive = Ship3dObjectSpriteDescriptor {
            draw_x: 1,
            draw_y: 2,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_position(&mut inactive, 3, 4),
            Ship3dSpriteSlotUpdateEffect::default()
        );
        assert_eq!(inactive.draw_x, 1);
        assert_eq!(inactive.draw_y, 2);

        let mut active = Ship3dObjectSpriteDescriptor {
            flags: 0x0001,
            draw_x: 10,
            draw_y: 20,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_position(&mut active, 10, 21),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                updated_position: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(active.draw_x, 10);
        assert_eq!(active.draw_y, 21);
        assert_eq!(
            active.flags,
            SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
    }

    #[test]
    fn sprite_slot_extent_update_matches_binary_dirty_and_bit4_rules() {
        let mut natural = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG,
            source_width: 64,
            source_height: 32,
            extent_width: 65,
            extent_height: 33,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            update_ship_3d_sprite_slot_extent(&mut natural, 64, 32),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                cleared_extent_changed_flag: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(
            natural.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
        assert_eq!(natural.extent_width, 65);
        assert_eq!(natural.extent_height, 33);

        assert_eq!(
            update_ship_3d_sprite_slot_extent(&mut natural, 80, 40),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                marked_dirty: true,
                updated_extent: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(
            natural.flags,
            SHIP_3D_OBJECT_VISIBLE_FLAG
                | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                | SHIP_3D_SPRITE_SLOT_EXTENT_CHANGED_FLAG
        );
        assert_eq!(natural.extent_width, 80);
        assert_eq!(natural.extent_height, 40);
    }

    #[test]
    fn sprite_slot_dirty_commit_copies_current_geometry_for_active_dirty_slots() {
        let mut descriptor = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            draw_y: 20,
            extent_width: 30,
            extent_height: 40,
            committed_draw_x: 1,
            committed_draw_y: 2,
            committed_extent_width: 3,
            committed_extent_height: 4,
            ..Ship3dObjectSpriteDescriptor::default()
        };

        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut descriptor),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                committed_geometry: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(descriptor.committed_draw_x, 10);
        assert_eq!(descriptor.committed_draw_y, 20);
        assert_eq!(descriptor.committed_extent_width, 30);
        assert_eq!(descriptor.committed_extent_height, 40);
        assert_eq!(
            descriptor.flags,
            SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
        );
    }

    #[test]
    fn sprite_slot_dirty_commit_skips_clean_or_inactive_slots() {
        let mut clean = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG,
            draw_x: 10,
            committed_draw_x: 1,
            ..Ship3dObjectSpriteDescriptor::default()
        };
        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut clean),
            Ship3dSpriteSlotUpdateEffect::default()
        );
        assert_eq!(clean.committed_draw_x, 1);

        let mut inactive_dirty = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            committed_draw_x: 1,
            ..Ship3dObjectSpriteDescriptor::default()
        };
        assert_eq!(
            commit_ship_3d_sprite_slot_dirty_geometry(&mut inactive_dirty),
            Ship3dSpriteSlotUpdateEffect {
                ran: true,
                ..Ship3dSpriteSlotUpdateEffect::default()
            }
        );
        assert_eq!(inactive_dirty.committed_draw_x, 1);
    }

    #[test]
    fn dirty_rect_clip_snapshot_replaces_list_and_clears_flag() {
        let mut dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 1,
                right: 2,
                top: 3,
                bottom: 4,
            }],
            sentinel: 0,
        };
        let mut snapshot_armed = true;
        let clip = Ship3dProjectionViewport {
            left: 5,
            right: 100,
            top: 0x23,
            bottom: 0xa5,
        };

        assert_eq!(
            commit_ship_3d_global_clip_snapshot(&mut dirty_rects, &mut snapshot_armed, clip),
            Ship3dDirtyRectSnapshotEffect {
                ran: true,
                wrote_clip_rect: true,
                wrote_sentinel: true,
                cleared_snapshot_flag: true,
            }
        );
        assert!(!snapshot_armed);
        assert_eq!(
            dirty_rects,
            Ship3dDirtyRectList {
                rects: vec![clip],
                sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
            }
        );
    }

    #[test]
    fn dirty_rect_clip_snapshot_without_flag_is_noop() {
        let mut dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 1,
                right: 2,
                top: 3,
                bottom: 4,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };
        let original = dirty_rects.clone();
        let mut snapshot_armed = false;

        assert_eq!(
            commit_ship_3d_global_clip_snapshot(
                &mut dirty_rects,
                &mut snapshot_armed,
                Ship3dProjectionViewport {
                    left: 5,
                    right: 100,
                    top: 0x23,
                    bottom: 0xa5,
                },
            ),
            Ship3dDirtyRectSnapshotEffect::default()
        );
        assert!(!snapshot_armed);
        assert_eq!(dirty_rects, original);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_collects_intersections_descending_and_clears_dirty() {
        let mut slots = vec![
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | 0x0008
                    | 0x0100,
                draw_x: 1,
                draw_y: 1,
                extent_width: 4,
                extent_height: 4,
                ..Ship3dObjectSpriteDescriptor::default()
            },
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
                draw_x: 10,
                draw_y: 10,
                extent_width: 4,
                extent_height: 4,
                ..Ship3dObjectSpriteDescriptor::default()
            },
            Ship3dObjectSpriteDescriptor {
                flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG
                    | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG
                    | 0x000c
                    | 0x0020
                    | 0x0040
                    | 0x0300,
                draw_x: 20,
                draw_y: 20,
                extent_width: 8,
                extent_height: 8,
                ..Ship3dObjectSpriteDescriptor::default()
            },
        ];
        let dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 0,
                right: 30,
                top: 0,
                bottom: 30,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };

        let commands =
            collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty_rects, 0, 2);

        assert_eq!(
            commands,
            vec![
                Ship3dSpriteSlotRenderCommand {
                    slot_index: 2,
                    dispatch_index: 7,
                    destination_remap_mode: 3,
                    flip_x: true,
                    flip_y: true,
                    slot_rect: Ship3dProjectionViewport {
                        left: 20,
                        right: 28,
                        top: 20,
                        bottom: 28,
                    },
                    dirty_rect: dirty_rects.rects[0],
                },
                Ship3dSpriteSlotRenderCommand {
                    slot_index: 0,
                    dispatch_index: 5,
                    destination_remap_mode: 1,
                    flip_x: false,
                    flip_y: false,
                    slot_rect: Ship3dProjectionViewport {
                        left: 1,
                        right: 5,
                        top: 1,
                        bottom: 5,
                    },
                    dirty_rect: dirty_rects.rects[0],
                },
            ]
        );
        assert_eq!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
        assert_eq!(slots[1].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
        assert_eq!(slots[2].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_without_dirty_rects_is_noop() {
        let mut slots = vec![Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 1,
            draw_y: 1,
            extent_width: 4,
            extent_height: 4,
            ..Ship3dObjectSpriteDescriptor::default()
        }];

        assert_eq!(
            collect_ship_3d_dirty_sprite_slot_render_commands(
                &mut slots,
                &Ship3dDirtyRectList::default(),
                0,
                0,
            ),
            Vec::<Ship3dSpriteSlotRenderCommand>::new()
        );
        assert_ne!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn dirty_sprite_slot_render_walk_uses_exclusive_edges() {
        let mut slots = vec![Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
            draw_x: 10,
            draw_y: 10,
            extent_width: 5,
            extent_height: 5,
            ..Ship3dObjectSpriteDescriptor::default()
        }];
        let dirty_rects = Ship3dDirtyRectList {
            rects: vec![Ship3dProjectionViewport {
                left: 15,
                right: 30,
                top: 10,
                bottom: 30,
            }],
            sentinel: SHIP_3D_DIRTY_RECT_SENTINEL,
        };

        assert_eq!(
            collect_ship_3d_dirty_sprite_slot_render_commands(&mut slots, &dirty_rects, 0, 0),
            Vec::<Ship3dSpriteSlotRenderCommand>::new()
        );
        assert_eq!(slots[0].flags & SHIP_3D_SPRITE_SLOT_DIRTY_FLAG, 0);
    }

    #[test]
    fn temp_snd_setup_without_trigger_is_noop() {
        let mut state = Ship3dTempSndState {
            phase: 1,
            plane_copy_enabled: false,
            scene_selector: 0x1234,
            hold_ticks: 0x0055,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(effect, Ship3dTempSndEffect::default());
        assert_eq!(
            state,
            Ship3dTempSndState {
                phase: 1,
                plane_copy_enabled: false,
                scene_selector: 0x1234,
                hold_ticks: 0x0055,
                setup_flag_a: true,
                setup_flag_b: true,
                ..Ship3dTempSndState::default()
            }
        );
    }

    #[test]
    fn temp_snd_setup_cycles_phase_and_runs_sequence_branch() {
        let mut state = Ship3dTempSndState {
            trigger: true,
            auxiliary_trigger: true,
            phase: 0,
            sequence_active: true,
            plane_copy_enabled: false,
            scene_selector: 0x2222,
            hold_ticks: 0x0040,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(
            effect,
            Ship3dTempSndEffect {
                ran: true,
                selected_callback_offset: Some(0x0087),
                next_phase: Some(1),
                load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
                restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
                preserved_mouse_position: true,
                reset_callback_bank_gate: true,
                called_presentation_callback: true,
                reset_hold_ticks: true,
                wrote_viewport_descriptor: true,
                sequence_branch: true,
                temporarily_disabled_plane_copy: true,
                enabled_plane_copy: true,
                reset_scene_selector: true,
                ..Ship3dTempSndEffect::default()
            }
        );
        assert!(!state.trigger);
        assert!(!state.auxiliary_trigger);
        assert_eq!(state.phase, 1);
        assert!(state.plane_copy_enabled);
        assert_eq!(
            state.scene_selector,
            SHIP_3D_TEMP_SND_SCENE_SELECTOR_SENTINEL
        );
        assert_eq!(state.hold_ticks, 0);
        assert!(state.fullscreen_refresh);
        assert_eq!(
            state.viewport_descriptor,
            SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR
        );
        assert!(state.setup_flag_a);
        assert!(state.setup_flag_b);
    }

    #[test]
    fn temp_snd_setup_wraps_phase_and_runs_non_sequence_branch() {
        let mut state = Ship3dTempSndState {
            trigger: true,
            auxiliary_trigger: true,
            phase: 2,
            sequence_active: false,
            plane_copy_enabled: false,
            scene_selector: 0x3333,
            hold_ticks: 0x0040,
            setup_flag_a: true,
            setup_flag_b: true,
            ..Ship3dTempSndState::default()
        };

        let effect = run_ship_3d_temp_snd_setup(&mut state).unwrap();

        assert_eq!(
            effect,
            Ship3dTempSndEffect {
                ran: true,
                selected_callback_offset: Some(0x009c),
                next_phase: Some(0),
                load_snd_bank_path: Some(SHIP_3D_TEMP_SND_PATH_OFFSET),
                restore_snd_bank_path: Some(SHIP_3D_TB_SND_PATH_OFFSET),
                preserved_mouse_position: true,
                reset_callback_bank_gate: true,
                called_presentation_callback: true,
                reset_hold_ticks: true,
                wrote_viewport_descriptor: true,
                non_sequence_branch: true,
                reset_setup_flags: true,
                ..Ship3dTempSndEffect::default()
            }
        );
        assert!(!state.trigger);
        assert!(!state.auxiliary_trigger);
        assert_eq!(state.phase, 0);
        assert!(!state.plane_copy_enabled);
        assert_eq!(state.scene_selector, 0x3333);
        assert_eq!(state.hold_ticks, 0);
        assert!(state.fullscreen_refresh);
        assert_eq!(
            state.viewport_descriptor,
            SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR
        );
        assert!(!state.setup_flag_a);
        assert!(!state.setup_flag_b);
    }

    #[test]
    fn navigation_final_reset_without_exit_pending_is_noop() {
        let mut state = Ship3dNavigationFinalResetState {
            hud_flags: 0xaaaa,
            status_flags: 0xff,
            scroll_mode: 0x1234,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(effect, Ship3dNavigationFinalResetEffect::default());
        assert_eq!(
            state,
            Ship3dNavigationFinalResetState {
                hud_flags: 0xaaaa,
                status_flags: 0xff,
                scroll_mode: 0x1234,
                ..Ship3dNavigationFinalResetState::default()
            }
        );
    }

    #[test]
    fn navigation_final_reset_reenters_active_sequence_while_opening() {
        let mut state = Ship3dNavigationFinalResetState {
            exit_pending: true,
            opening: true,
            hud_flags: 0xaaaa,
            status_flags: 0xff,
            scroll_mode: 0x1234,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(
            effect,
            Ship3dNavigationFinalResetEffect {
                reentered_active_sequence: true,
                ..Ship3dNavigationFinalResetEffect::default()
            }
        );
        assert_eq!(
            state,
            Ship3dNavigationFinalResetState {
                exit_pending: true,
                opening: true,
                hud_flags: 0xaaaa,
                status_flags: 0xff,
                scroll_mode: 0x1234,
                ..Ship3dNavigationFinalResetState::default()
            }
        );
    }

    #[test]
    fn navigation_final_reset_applies_binary_teardown_state() {
        let mut state = Ship3dNavigationFinalResetState {
            exit_pending: true,
            hud_flags: 0x1111,
            nav_choice_hold_ticks: 0x2222,
            nav_choice_timer: 0x3333,
            dialogue_state: 0x4444,
            scene_band_top: 0x5555,
            scene_selector: 0x6666,
            active_record: 0x7777,
            presentation_gate: true,
            pending_state_byte: true,
            subtitle_gate: true,
            presentation_defer_active: true,
            secondary_presentation_defer_active: true,
            plane_copy_enabled: true,
            sequence_active: true,
            status_flags: 0xff,
            secondary_status_flag: true,
            dirty_marker: 0x12,
            scroll_value: 0x8888,
            scroll_mode: 0x9999,
            ..Ship3dNavigationFinalResetState::default()
        };

        let effect = run_ship_3d_navigation_final_reset(&mut state);

        assert_eq!(
            effect,
            Ship3dNavigationFinalResetEffect {
                ran: true,
                cleared_dialogue_state: true,
                reset_hud_state: true,
                reset_presentation_gates: true,
                reset_sequence_flags: true,
                reset_status_flags: true,
                copied_backbuffer_restore_block: true,
                cleared_overlay_scratch: true,
                reset_scroll_state: true,
                called_render_clear: true,
                called_input_reset: true,
                called_target_cleanup: true,
                ..Ship3dNavigationFinalResetEffect::default()
            }
        );
        assert_eq!(state.hud_flags, SHIP_3D_FINAL_RESET_HUD_FLAGS);
        assert_eq!(state.nav_choice_hold_ticks, 0);
        assert_eq!(state.nav_choice_timer, SHIP_3D_FINAL_RESET_NAV_TIMER);
        assert!(state.post_reset_gate);
        assert!(state.navigation_gate);
        assert_eq!(state.dialogue_state, 0);
        assert_eq!(state.scene_band_top, 0);
        assert_eq!(state.scene_selector, SHIP_3D_FINAL_RESET_SELECTOR_SENTINEL);
        assert_eq!(
            state.active_record,
            SHIP_3D_FINAL_RESET_ACTIVE_RECORD_SENTINEL
        );
        assert!(!state.presentation_gate);
        assert!(!state.exit_pending);
        assert!(!state.pending_state_byte);
        assert!(!state.subtitle_gate);
        assert!(!state.presentation_defer_active);
        assert!(!state.secondary_presentation_defer_active);
        assert!(!state.plane_copy_enabled);
        assert!(!state.sequence_active);
        assert_eq!(state.status_flags, SHIP_3D_FINAL_RESET_STATUS_FLAG_MASK);
        assert!(!state.secondary_status_flag);
        assert_eq!(state.dirty_marker, SHIP_3D_FINAL_RESET_DIRTY_MARKER);
        assert_eq!(state.scroll_value, 0);
        assert_eq!(state.scroll_mode, SHIP_3D_FINAL_RESET_SCROLL_MODE);
    }

    #[test]
    fn navigation_sequence_active_path_runs_temp_snd_and_blocks_on_presentation() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            true,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                blocked_by_presentation_active: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(!state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_copies_framebuffer_without_target_query_when_duration_differs() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION - 1,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, true, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_waits_while_interpolation_is_active() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, false, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                interpolation_active: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_complete_selection_arms_exit_pending() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(&mut state, false, false, true, 0);

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                queried_target_list: true,
                armed_exit_pending: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.framebuffer_dirty);
        assert!(!state.sequence_active);
        assert!(state.exit_pending);
    }

    #[test]
    fn navigation_sequence_complete_no_selection_keeps_sequence_active() {
        let mut state = Ship3dNavigationSequenceState {
            sequence_active: true,
            interpolation_duration_ticks: SHIP_3D_NAVIGATION_INTERPOLATION_DURATION,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                ran_temp_snd_setup: true,
                ran_procedural_update: true,
                copied_framebuffer: true,
                queried_target_list: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.sequence_active);
        assert!(!state.exit_pending);
    }

    #[test]
    fn navigation_sequence_inactive_without_defer_arms_opening_exit() {
        let mut state = Ship3dNavigationSequenceState::default();

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            false,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                armed_opening_exit: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.exit_pending);
        assert!(state.opening);
    }

    #[test]
    fn navigation_sequence_exit_pending_without_opening_reports_final_reset() {
        let mut state = Ship3dNavigationSequenceState {
            exit_pending: true,
            opening: false,
            ..Ship3dNavigationSequenceState::default()
        };

        let effect = run_ship_3d_navigation_sequence_update(
            &mut state,
            false,
            false,
            true,
            SHIP_3D_TARGET_LAYOUT_SELECTOR_RETURN,
        );

        assert_eq!(
            effect,
            Ship3dNavigationSequenceEffect {
                final_reset_pending: true,
                ..Ship3dNavigationSequenceEffect::default()
            }
        );
        assert!(state.exit_pending);
        assert!(!state.opening);
    }

    fn nav_record(
        offset: u16,
        kind_flags: u16,
        state_flags: u8,
        counter_link: u16,
        related_target: u16,
    ) -> Ship3dNavigationRuntimeRecord {
        Ship3dNavigationRuntimeRecord {
            offset,
            kind_flags,
            state_flags,
            counter_link,
            related_target,
            source_parent: None,
        }
    }

    fn nav_record_with_source_parent(
        offset: u16,
        kind_flags: u16,
        state_flags: u8,
        counter_link: u16,
        related_target: u16,
        source_parent: Option<u16>,
    ) -> Ship3dNavigationRuntimeRecord {
        Ship3dNavigationRuntimeRecord {
            offset,
            kind_flags,
            state_flags,
            counter_link,
            related_target,
            source_parent,
        }
    }

    #[test]
    fn navigation_source_records_follow_selector_11_tree_depth_first() {
        let records = [
            nav_record_with_source_parent(0x3000, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x3000)),
            nav_record_with_source_parent(0x3200, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3300, 0, 0, 0, 0, Some(0x9999)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3200,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3300,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3400,
                entry_kind: 0,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(
            source_records,
            vec![0x3000, 0x3100, 0x3200, SHIP_3D_TARGET_EXIT_SENTINEL]
        );
    }

    #[test]
    fn navigation_source_records_stop_before_first_non_kind1_next_entry() {
        let records = [
            nav_record_with_source_parent(0x3000, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x2000)),
            nav_record_with_source_parent(0x3200, 0, 0, 0, 0, Some(0x2000)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 0,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3200,
                entry_kind: 1,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(source_records, vec![0x3000, SHIP_3D_TARGET_EXIT_SENTINEL]);
    }

    #[test]
    fn navigation_source_records_skip_kinds_without_selector_11_parent() {
        let records = [
            nav_record(0x3000, 0, 0, 0, 0),
            nav_record_with_source_parent(0x3100, 0, 0, 0, 0, Some(0x2000)),
        ];
        let source_entries = [
            Ship3dNavigationSourceEntry {
                record_offset: 0x3000,
                entry_kind: 1,
            },
            Ship3dNavigationSourceEntry {
                record_offset: 0x3100,
                entry_kind: 1,
            },
        ];

        let source_records =
            build_ship_3d_navigation_source_records(&source_entries, &records, 0x2000).unwrap();

        assert_eq!(source_records, vec![0x3100, SHIP_3D_TARGET_EXIT_SENTINEL]);
    }

    #[test]
    fn navigation_source_records_require_at_least_one_source_entry() {
        assert_eq!(
            build_ship_3d_navigation_source_records(&[], &[], 0x2000),
            None
        );
    }

    fn position_record(
        offset: u16,
        kind_flags: u16,
        parent_link: Option<u16>,
        kind100_match_word: Option<u16>,
        kind100_relation_word: Option<u16>,
    ) -> Ship3dPositionRecord {
        Ship3dPositionRecord {
            offset,
            kind_flags,
            parent_link,
            kind100_match_word,
            kind100_relation_word,
        }
    }

    fn position_field(offset: u16, x: u16, y: u16) -> Ship3dPositionField {
        Ship3dPositionField { offset, x, y }
    }

    #[test]
    fn position_field_resolves_direct_coordinate_kinds() {
        let records = [
            position_record(
                0x1000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
            position_record(
                0x1100,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10,
                None,
                None,
                None,
            ),
            position_record(
                0x1200,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_40,
                None,
                None,
                None,
            ),
            position_record(
                0x1300,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_200,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x1018)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1100, 0x2000, 0),
            Some(0x1118)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1200, 0x2000, 0),
            Some(0x1200)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1300, 0x2000, 0),
            Some(0x1306)
        );
    }

    #[test]
    fn position_field_follows_selector_11_parent_chain() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1100), None, None),
            position_record(0x1100, 0x0002, Some(0x1200), None, None),
            position_record(
                0x1200,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x1218)
        );
    }

    #[test]
    fn position_field_uses_arche_for_selector_11_sentinel() {
        let records = [
            position_record(0x1000, 0x0002, None, None, None),
            position_record(
                0x2000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_10,
                None,
                None,
                None,
            ),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            Some(0x2018)
        );
    }

    #[test]
    fn position_field_kind100_chooses_match_or_mismatch_block() {
        let records = [position_record(
            0x1000,
            SHIP_3D_OBJECT_KIND_POSITION_KIND100,
            None,
            Some(0x2222),
            None,
        )];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0x2222),
            Some(0x1018)
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0x3333),
            Some(0x101c)
        );
    }

    #[test]
    fn position_field_rejects_unresolvable_parent_chain() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1000), None, None),
            position_record(0x2000, 0x0020, Some(0x1000), None, None),
        ];

        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x1000, 0x2000, 0),
            None
        );
        assert_eq!(
            resolve_ship_3d_position_field(&records, 0x2000, 0x1000, 0),
            None
        );
    }

    #[test]
    fn position_distance_uses_binary_sqrt_distance() {
        let first = position_field(0x1000, 10, 20);
        let second = position_field(0x2000, 13, 24);

        assert_eq!(ship_3d_position_field_distance(first, second), Some(5));
    }

    #[test]
    fn position_distance_uses_binary_rounded_sqrt() {
        assert_eq!(ship_3d_binary_sqrt(24), Some(5));
        assert_eq!(ship_3d_binary_sqrt(20), Some(4));
        assert_eq!(
            ship_3d_position_field_distance(
                position_field(0x1000, 0, 0),
                position_field(0x2000, 2, 4),
            ),
            Some(4)
        );
    }

    #[test]
    fn position_distance_uses_wrapping_signed_word_diffs() {
        assert_eq!(
            ship_3d_position_field_distance(
                position_field(0x1000, 0xffff, 0),
                position_field(0x2000, 0x0001, 0),
            ),
            Some(2)
        );
    }

    #[test]
    fn position_distance_resolves_kind100_against_other_relation_word() {
        let first = position_record(
            0x1000,
            SHIP_3D_OBJECT_KIND_POSITION_KIND100,
            None,
            Some(0x2222),
            None,
        );
        let second_match = position_record(
            0x2000,
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
            None,
            None,
            Some(0x2222),
        );
        let second_mismatch = position_record(
            0x2100,
            SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
            None,
            None,
            Some(0x3333),
        );
        let fields = [
            position_field(0x1018, 0, 0),
            position_field(0x101c, 10, 0),
            position_field(0x2018, 3, 4),
            position_field(0x2118, 3, 4),
        ];

        assert_eq!(
            ship_3d_position_distance(&[first, second_match], &fields, 0x1000, 0x2000, 0, 0),
            Some(5)
        );
        assert_eq!(
            ship_3d_position_distance(&[first, second_mismatch], &fields, 0x1000, 0x2100, 0, 0),
            Some(8)
        );
    }

    #[test]
    fn position_distance_follows_parent_chain_with_inherited_kind100_compare_word() {
        let records = [
            position_record(0x1000, 0x0002, Some(0x1100), None, None),
            position_record(
                0x1100,
                SHIP_3D_OBJECT_KIND_POSITION_KIND100,
                None,
                Some(0x4444),
                None,
            ),
            position_record(
                0x2000,
                SHIP_3D_OBJECT_KIND_POSITION_DIRECT_8,
                None,
                None,
                None,
            ),
        ];
        let fields = [
            position_field(0x1118, 0, 0),
            position_field(0x111c, 10, 0),
            position_field(0x2018, 3, 4),
        ];

        assert_eq!(
            ship_3d_position_distance(&records, &fields, 0x1000, 0x2000, 0, 0x4444),
            Some(5)
        );
        assert_eq!(
            ship_3d_position_distance(&records, &fields, 0x1000, 0x2000, 0, 0x5555),
            Some(8)
        );
    }

    #[test]
    fn object_table_bit_test_uses_selector5_kind2_field_offset() {
        assert_eq!(
            vm::vm_field_offset(SHIP_3D_SOURCE_BITSET_SELECTOR, SHIP_3D_SOURCE_BITSET_KIND),
            Some(0x1e)
        );
    }

    #[test]
    fn object_table_bit_test_uses_high_bit_first_masks() {
        let object_table = [
            0x1000, 0x1014, 0x1028, 0x103c, 0x1050, 0x1064, 0x1078, 0x108c, 0x10a0,
        ];
        let mut bitset = [0u8; 0x21];
        bitset[0x1e] = 0x81;
        bitset[0x1f] = 0x80;

        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x1000),
            Some(true)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x1014),
            Some(false)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x108c),
            Some(true)
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x10a0),
            Some(true)
        );
    }

    #[test]
    fn object_table_bit_test_requires_known_object_and_available_byte() {
        let object_table = [0x1000, 0x1014];
        let bitset = [0xffu8; 0x1f];

        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset, 0x9999),
            None
        );
        assert_eq!(
            ship_3d_object_table_bit_is_set(&object_table, &bitset[..0x1e], 0x1000),
            None
        );
    }

    #[test]
    fn c1_source_selection_accepts_kind2_when_operand_bit_is_set() {
        let records = [nav_record(0x3000, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0)];
        let object_table = [0x2000];
        let mut source_list_bytes = [0u8; 0x21];
        source_list_bytes[0x20] = 0x80;

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(Some(0x3000))
        );
    }

    #[test]
    fn c1_source_selection_falls_through_clear_bit_to_kind1_operand_flag() {
        let records = [
            nav_record(0x3000, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0),
            nav_record(0x3100, SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG, 0, 0, 0),
        ];
        let object_table = [0x2000];
        let source_list_bytes = [0u8; 0x21];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, 0x3100, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG,
            ),
            Some(Some(0x3100))
        );
    }

    /// A kind that is NEITHER 1 nor 2 resumes the scan instead of ending it.
    /// The handler's two arms are `cmp ax,2 / jne 0x6C36` @`0x6C27` and
    /// `cmp ax,1 / jne 0x6C1C` @`0x6C36`: the second `jne` targets the `lodsw`
    /// at the top of the loop, so an unrecognised kind advances to the next
    /// source entry. That is the `_ => {}` arm, and without a test the arm reads
    /// like defensive padding rather than the decoded control flow it is.
    /// Re-DERIVES `SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR` by executing the store
    /// sequence at `0xB629`..`0xB643` instead of restating the array. Asserting
    /// the constant equals itself would prove nothing; running the instructions
    /// that build it is the part that can disagree.
    ///
    /// It also pins the structure the two `stosd`s impose: indices 3 and 7 are
    /// the HIGH HALVES of 32-bit stores, not independent fields.
    #[test]
    fn temp_snd_viewport_descriptor_matches_the_stores_that_build_it() {
        // The write cursor `di` walks a 16-byte destination (`les di,[0x522d]`).
        let mut out: Vec<u16> = Vec::new();
        let stosw = |v: u16, out: &mut Vec<u16>| out.push(v);
        let stosd = |v: u32, out: &mut Vec<u16>| {
            out.push(v as u16); // low half
            out.push((v >> 16) as u16); // HIGH half -- the index that is not a field
        };

        let mut eax: u32 = 0; // xor eax, eax        @0xB62D
        stosw(eax as u16, &mut out); //  stosw        @0xB630
        eax = eax.wrapping_add(1); // inc ax          @0xB631
        stosw(eax as u16, &mut out); //  stosw        @0xB632
        eax = eax.wrapping_add(3); // add ax, 3       @0xB633
        stosd(eax, &mut out); //          stosd       @0xB636
        eax = 0x140; //                mov ax, 0x140  @0xB638
        stosw(eax as u16, &mut out); //  stosw        @0xB63B
        eax = 0xc8; //                 mov ax, 0xc8   @0xB63C
        stosw(eax as u16, &mut out); //  stosw        @0xB63F
        eax = 0; //                    xor eax, eax   @0xB640
        stosd(eax, &mut out); //          stosd       @0xB642

        assert_eq!(
            out.len(),
            SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR.len(),
            "the stores must fill the descriptor exactly -- 2 stosw + 1 stosd + \
             2 stosw + 1 stosd = 8 words"
        );
        assert_eq!(out.as_slice(), &SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[..]);

        // The high halves, called out so a future edit cannot quietly treat them
        // as fields: index 3 belongs to the dword 4, index 7 to the dword 0.
        assert_eq!(SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[3], 0);
        assert_eq!(SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[7], 0);
        assert_eq!(
            (SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[2] as u32)
                | ((SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[3] as u32) << 16),
            4
        );

        // 320x200 -- a full-screen viewport, the one reading the values alone
        // already supported.
        assert_eq!(SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[4], 320);
        assert_eq!(SHIP_3D_TEMP_SND_VIEWPORT_DESCRIPTOR[5], 200);
    }

    #[test]
    fn c1_source_selection_skips_unknown_kind_and_keeps_scanning() {
        // 0x10 is a real record kind elsewhere in the C1 handler (the target
        // whose match ENTERS this scan, `cmp ax,0x10` @`0x6C07`), so it is the
        // kind most likely to reach the loop without being one of its two arms.
        let records = [
            nav_record(0x3000, SHIP_3D_C1_KIND10_RECORD_KIND, 0, 0, 0),
            nav_record(0x3100, SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG, 0, 0, 0),
        ];
        let object_table = [0x2000];
        let source_list_bytes = [0u8; 0x21];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, 0x3100, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG,
            ),
            Some(Some(0x3100)),
            "the unknown kind at 0x3000 must be skipped, not matched or fatal"
        );
    }

    #[test]
    fn c1_source_selection_uses_current_source_cursor_for_kind2_bitset() {
        let records = [
            nav_record(0x3000, 0x0003, 0, 0, 0),
            nav_record(0x3100, SHIP_3D_C1_SOURCE_KIND_BITSET, 0, 0, 0),
        ];
        let object_table = [0x2000];
        let mut source_list_bytes = [0u8; 0x23];
        source_list_bytes[0x20] = 0x00;
        source_list_bytes[0x22] = 0x80;

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, 0x3100, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(Some(0x3100))
        );
    }

    #[test]
    fn c1_source_selection_reaches_sentinel_without_match() {
        let records = [nav_record(
            0x3000,
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG,
            0,
            0,
            0,
        )];
        let object_table = [0x2000];
        let source_list_bytes = [0xffu8; 0x1f];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000, SHIP_3D_TARGET_EXIT_SENTINEL, 0x9999],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            Some(None)
        );
    }

    #[test]
    fn c1_source_selection_requires_known_records_and_sentinel() {
        let records = [nav_record(
            0x3000,
            SHIP_3D_C1_SOURCE_KIND_OPERAND_FLAG,
            0,
            0,
            0,
        )];
        let object_table = [0x2000];
        let source_list_bytes = [0xffu8; 0x1f];

        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x9999, SHIP_3D_TARGET_EXIT_SENTINEL],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                SHIP_3D_C1_SOURCE_OPERAND_STATE_FLAG,
            ),
            None
        );
        assert_eq!(
            select_ship_3d_c1_source_record(
                &[0x3000],
                &records,
                &object_table,
                &source_list_bytes,
                0x2000,
                0,
            ),
            None
        );
    }

    #[test]
    fn c1_kind10_destination_uses_selector13_kind10_field_offset() {
        assert_eq!(
            vm::vm_field_offset(
                SHIP_3D_C1_DESTINATION_SELECTOR,
                SHIP_3D_C1_KIND10_RECORD_KIND
            ),
            Some(0x1c)
        );
        assert_eq!(
            resolve_ship_3d_c1_kind10_destination_record(0x4000, SHIP_3D_C1_KIND10_RECORD_KIND,),
            Some(0x401c)
        );
        assert_eq!(
            resolve_ship_3d_c1_kind10_destination_record(0x4000, 0x0002),
            None
        );
    }

    #[test]
    fn c1_kind10_destination_write_records_c1_operand_and_aux2() {
        let mut slot = Ship3dRecordStateSlot::default();

        let write = write_ship_3d_c1_kind10_destination_slot(
            0x4000,
            SHIP_3D_C1_KIND10_RECORD_KIND,
            &mut slot,
            0x2000,
        );

        let expected_slot = Ship3dRecordStateSlot {
            opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
            operand: 0x2000,
            aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
        };
        assert_eq!(
            write,
            Some(Some(Ship3dC1DestinationWrite {
                destination_record_offset: 0x401c,
                slot: expected_slot,
            }))
        );
        assert_eq!(slot, expected_slot);
    }

    #[test]
    fn c1_kind10_destination_write_branches_when_destination_occupied() {
        let mut slot = Ship3dRecordStateSlot {
            opcode: 0x00c4,
            operand: 0x1111,
            aux_word: 0x2222,
        };

        let write = write_ship_3d_c1_kind10_destination_slot(
            0x4000,
            SHIP_3D_C1_KIND10_RECORD_KIND,
            &mut slot,
            0x2000,
        );

        assert_eq!(write, Some(None));
        assert_eq!(
            slot,
            Ship3dRecordStateSlot {
                opcode: 0x00c4,
                operand: 0x1111,
                aux_word: 0x2222,
            }
        );
    }

    #[test]
    fn c1_kind10_destination_write_checks_only_first_destination_word() {
        let mut slot = Ship3dRecordStateSlot {
            opcode: 0,
            operand: 0x1111,
            aux_word: 0x2222,
        };

        assert_eq!(
            write_ship_3d_c1_kind10_destination_slot(
                0x4000,
                SHIP_3D_C1_KIND10_RECORD_KIND,
                &mut slot,
                0x2000,
            )
            .map(|write| write.map(|write| write.destination_record_offset)),
            Some(Some(0x401c))
        );
        assert_eq!(
            slot,
            Ship3dRecordStateSlot {
                opcode: SHIP_3D_C1_RECORD_STATE_OPCODE,
                operand: 0x2000,
                aux_word: SHIP_3D_C1_RECORD_STATE_AUX_WORD,
            }
        );
    }

    #[test]
    fn navigation_candidates_filter_kind2_active_records_and_skip_honk() {
        let records = [
            nav_record(0x1000, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x01, 0, 0),
            nav_record(0x1100, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x00, 0, 0),
            nav_record(0x1200, 0x0003, 0x01, 0, 0),
            nav_record(0x1300, SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE, 0x01, 0, 0),
        ];

        let candidates = build_ship_3d_navigation_candidate_records(
            &[0x1000, 0x1100, 0x1200, 0x1300, 0xffff, 0x1400],
            &records,
            0x1300,
        )
        .unwrap();

        assert_eq!(candidates, vec![0x1000]);
    }

    #[test]
    fn navigation_candidates_require_source_sentinel() {
        let records = [nav_record(
            0x1000,
            SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
            0x01,
            0,
            0,
        )];

        assert_eq!(
            build_ship_3d_navigation_candidate_records(&[0x1000], &records, 0),
            None
        );
    }

    #[test]
    fn navigation_trigger_defers_first_matching_c4_candidate() {
        let records = [
            nav_record(0x2000, 0x0000, 0x00, 0x2550, 0),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x2000,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            render_clip_bottom: 0x9999,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0x0007,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(
            effect,
            Ship3dNavigationTriggerEffect {
                candidate_records: vec![0x3100],
                copied_pending_presentation_state: true,
                incremented_counter_record: Some(0x2000),
                deferred_record_type: Some(SHIP_3D_NAVIGATION_DEFERRED_RECORD_TYPE),
                deferred_record_related: Some(0x3100),
                candidate_handler_record: Some(0x3104),
                cleared_trigger: true,
                started_sequence: true,
                set_scene_band: true,
                restored_render_clip: true,
                cleared_active_dialogue_record: true,
                requested_closing: true,
                ..Ship3dNavigationTriggerEffect::default()
            }
        );
        assert!(!state.trigger_active);
        assert!(state.sequence_active);
        assert_eq!(state.requested_presentation_state, 0x0007);
        assert_eq!(state.scene_band_top, SHIP_3D_NAVIGATION_SCENE_BAND_TOP);
        assert_eq!(
            state.render_clip_bottom,
            SHIP_3D_NAVIGATION_RENDER_CLIP_RESTORED_BOTTOM
        );
        assert_eq!(state.active_dialogue_record, SHIP_3D_TARGET_EXIT_SENTINEL);
        assert!(state.closing);
        assert_eq!(state.depth_step, SHIP_3D_NAVIGATION_TRIGGER_CLOSE_STEP);
        assert_eq!(state.hud_flags, 0);
    }

    #[test]
    fn navigation_trigger_match_any_flag_ignores_candidate_related_target() {
        let records = [
            nav_record(
                0x2000,
                0x0000,
                SHIP_3D_NAVIGATION_CURRENT_TARGET_MATCH_ANY_FLAG,
                0x2550,
                0,
            ),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x9999,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0,
            [0; SHIP_3D_INTERPOLATION_WORDS],
        )
        .unwrap();

        assert_eq!(effect.deferred_record_related, Some(0x3100));
        assert!(!effect.opened_target_list);
    }

    #[test]
    fn navigation_trigger_ark_related_candidate_opens_target_list() {
        let records = [
            nav_record(0x2000, 0x0000, 0x00, 0, 0),
            nav_record(
                0x3100,
                SHIP_3D_NAVIGATION_RECORD_KIND_CANDIDATE,
                0x01,
                0,
                0x6758,
            ),
        ];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            layout_rect_snapshot: [0xaaaa, 0xbbbb, 0xcccc, 0xdddd],
            interpolation_current_tick: 5,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0x3100, 0xffff],
            0x6754,
            0x6758,
            0,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(
            effect,
            Ship3dNavigationTriggerEffect {
                candidate_records: vec![0x3100],
                copied_pending_presentation_state: true,
                incremented_counter_record: Some(0x2000),
                opened_target_list: true,
                reset_interpolation_tick: true,
                ran_layout_prepass: true,
                copied_layout_x_and_width: true,
                cleared_trigger: true,
                started_sequence: true,
                set_scene_band: true,
                restored_render_clip: true,
                cleared_active_dialogue_record: true,
                requested_closing: true,
                ..Ship3dNavigationTriggerEffect::default()
            }
        );
        assert_eq!(state.hud_flags, SHIP_3D_NAVIGATION_TARGET_LIST_FLAG);
        assert_eq!(
            state.interpolation_duration_ticks,
            SHIP_3D_NAVIGATION_INTERPOLATION_DURATION
        );
        assert_eq!(state.interpolation_current_tick, 0);
        assert_eq!(state.layout_rect_snapshot, [0x10, 0xbbbb, 0x30, 0xdddd]);
        assert!(!state.target_query_mode);
    }

    #[test]
    fn navigation_trigger_no_candidate_opens_target_list_and_redirects_counter_increment() {
        let records = [nav_record(
            0x2000,
            SHIP_3D_NAVIGATION_REDIRECT_COUNTER_FLAG,
            0x00,
            0x2a00,
            0,
        )];
        let mut state = Ship3dNavigationTriggerState {
            trigger_active: true,
            current_target: 0x2000,
            ..Ship3dNavigationTriggerState::default()
        };

        let effect = run_ship_3d_navigation_trigger_prelude(
            &mut state,
            &records,
            &[0xffff],
            0x6754,
            0x6758,
            0,
            [0x10, 0x20, 0x30, 0x40],
        )
        .unwrap();

        assert_eq!(effect.candidate_records, Vec::<u16>::new());
        assert_eq!(effect.incremented_counter_record, Some(0x2a00));
        assert!(effect.opened_target_list);
        assert_eq!(state.hud_flags, SHIP_3D_NAVIGATION_TARGET_LIST_FLAG);
    }

    fn axis_aligned_projection_matrix() -> Ship3dProjectionMatrix {
        // Rows 1/2/3 pick x, y, z directly at ~unit Q15 scale so a translated
        // point (tx,ty,tz>0) projects near screen centre with depth ~= tz.
        Ship3dProjectionMatrix {
            terms: [
                0x7fff, 0, 0, // screen-x numerator uses tx
                0, 0x7fff, 0, // screen-y numerator uses ty
                0, 0, 0x7fff, // depth uses tz
            ],
        }
    }

    #[test]
    fn render_point_cloud_matches_manual_primitive_loop_and_writes_once() {
        let matrix = axis_aligned_projection_matrix();
        let origin = Ship3dProjectionOrigin { x: 0, y: 0, z: 0 };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: SHIP_3D_PROJECTION_SCREEN_WIDTH as u16,
            top: 0,
            bottom: SHIP_3D_PROJECTION_SCREEN_HEIGHT as u16,
        };

        // A spread of points, including the on-axis (0,0,z) point that projects
        // to screen centre, and a duplicate of it to exercise write-once.
        let p = |x, y, z| Ship3dProjectionPoint { x, y, z };
        let points = vec![
            p(0, 0, 0x0100),
            p(0, 0, 0x0100), // duplicate cell -> write-once
            p(0x40, 0x30, 0x0180),
            p(0x80, 0x20, 0x0200),
            p(0, 0, 0), // depth 0 -> skipped
            p(0x20, 0x60, 0x0140),
        ];

        // Expected buffer/count from calling the primitives directly.
        let mut expected_buffer =
            vec![0u8; SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT];
        let mut expected_plotted = 0usize;
        for &point in &points {
            if let Some(projected) = project_ship_3d_point(point, origin, matrix) {
                if plot_ship_3d_projected_point(&mut expected_buffer, viewport, projected).is_some()
                {
                    expected_plotted += 1;
                }
            }
        }

        let render = render_ship_3d_point_cloud(&points, origin, matrix, viewport);
        assert_eq!(render.plotted, expected_plotted);
        assert_eq!(render.buffer, expected_buffer);

        // The on-axis point must land somewhere, and the duplicate must not be
        // double-counted (write-once): fewer plotted than non-degenerate points.
        assert!(render.plotted >= 1);
        assert!(render.plotted < points.len());
        // Every drawn cell carries a depth shade, never a stray zero.
        assert!(render.buffer.iter().filter(|&&p| p != 0).count() == render.plotted);
    }

    #[test]
    fn blood_prng_first_call_from_zero_state_returns_zero_and_advances_bytes() {
        // Hand-traced from the shipped all-zero state: the 8-iteration carry
        // chain over two zero bytes yields 0, XOR seed 0 stays 0, then the byte
        // advance sets counter=1, b -= 1 (0x00 -> 0xFF), a ^= rol(1,1)=2.
        let mut prng = BloodPrng::default();
        assert_eq!(prng.next(0xffff), 0);
        assert_eq!(
            prng,
            BloodPrng {
                seed_word: 0,
                a: 2,
                b: 0xff,
                counter: 1,
            }
        );
    }

    #[test]
    /// The invariant the per-frame call depends on: same seed -> same cloud.
    ///
    /// The game randomizes ONCE at `0x0FD3` (boot). `EngineState` calls
    /// `render_ship_3d_starfield` every frame instead, which re-randomizes — safe
    /// only while the seed is constant. If that ever becomes a real RTC read the
    /// stars start boiling, and nothing else in the tree would notice.
    #[test]
    /// `0x44F2`'s `jge`/`jle` are SIGNED, and a slot hanging off the left edge is
    /// the case that tells the two readings apart.
    #[test]
    fn a_slot_off_the_left_edge_still_intersects() {
        let rect = |left: u16, top: u16, right: u16, bottom: u16| Ship3dProjectionViewport {
            left,
            right,
            top,
            bottom,
        };
        // draw_x = -16, so the slot spans x in [-16, 16) and overlaps a dirty rect
        // at x in [0, 32). Read unsigned, `left` is 65520 and this misses entirely.
        let slot = rect(0xFFF0, 0, 16, 32);
        let dirty = rect(0, 0, 32, 32);
        assert!(
            ship_3d_rects_intersect(slot, dirty),
            "a negative draw_x must compare as negative (0x44F2 jge is signed)"
        );
        // The port must not simply answer `true`: genuinely disjoint stays disjoint,
        // with the same negative coordinate in play.
        assert!(
            !ship_3d_rects_intersect(rect(0xFFF0, 0, 0xFFF8, 32), dirty),
            "a slot entirely left of the dirty rect must not intersect"
        );
        // And the bounds are STRICT — touching edges do not overlap (`jle`/`jge`).
        assert!(
            !ship_3d_rects_intersect(rect(32, 0, 64, 32), dirty),
            "slot.left == dirty.right must not intersect"
        );
    }

    fn the_starfield_is_stable_only_because_the_seed_is() {
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: 11,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin { x: 0x8000, y: 0x8000, z: 0x8000 };
        let viewport = Ship3dProjectionViewport { left: 0, right: 320, top: 0, bottom: 200 };
        let shot = |seed: u8| {
            let mut prng = BloodPrng::seeded_from_rtc_seconds(seed);
            render_ship_3d_starfield(&mut prng, angles, origin, viewport)
                .expect("in-range angles")
                .buffer
        };
        // Two frames at the engine's fixed seed are the SAME field.
        assert_eq!(shot(17), shot(17), "same seed must give the same cloud");
        // NOT VACUOUS: a different seed really does move the stars, so the equality
        // above is a property of the seed and not of a renderer that ignores it.
        assert_ne!(shot(17), shot(42), "a different seed must give a different cloud");
    }

    fn render_ship_3d_starfield_uses_real_table_and_plots_points() {
        // Full faithful path: PRNG -> randomized cloud -> recovered angle table
        // -> camera matrix -> depth-shaded buffer. The point cloud spans the
        // full u16 range, so an origin near its centre keeps points in front of
        // the camera and on screen.
        let mut prng = BloodPrng::seeded_from_rtc_seconds(17);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: 0,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin {
            x: 0x8000,
            y: 0x8000,
            z: 0x8000,
        };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: SHIP_3D_PROJECTION_SCREEN_WIDTH as u16,
            top: 0,
            bottom: SHIP_3D_PROJECTION_SCREEN_HEIGHT as u16,
        };
        let render = render_ship_3d_starfield(&mut prng, angles, origin, viewport).unwrap();
        assert_eq!(
            render.buffer.len(),
            SHIP_3D_PROJECTION_SCREEN_WIDTH * SHIP_3D_PROJECTION_SCREEN_HEIGHT
        );
        // Some points project in front of the camera and shade the buffer, and
        // every drawn cell carries a nonzero depth shade (write-once contract).
        assert!(render.plotted > 0);
        assert_eq!(
            render.buffer.iter().filter(|&&p| p != 0).count(),
            render.plotted
        );
    }

    #[test]
    fn angle_table_matches_binary() {
        // Byte-exact vs the little-endian (cosine, sine) i16 pairs at DS:0x4F45
        // (file 0xD420 + 0x4F45). Skips when the binary is not checked out.
        let candidates = ["re/bin/BLOODPRG.EXE", "../re/bin/BLOODPRG.EXE"];
        let Some(data) = candidates.iter().find_map(|p| std::fs::read(p).ok()) else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };
        let base = 0xD420 + 0x4F45;
        for (i, entry) in SHIP_3D_ANGLE_TABLE.iter().enumerate() {
            let off = base + i * 4;
            let cos = i16::from_le_bytes([data[off], data[off + 1]]);
            let sin = i16::from_le_bytes([data[off + 2], data[off + 3]]);
            assert_eq!(entry.cosine, cos, "cosine mismatch at index {i}");
            assert_eq!(entry.sine, sin, "sine mismatch at index {i}");
        }
    }

    #[test]
    fn angle_table_is_a_consistent_trig_table() {
        assert_eq!(SHIP_3D_ANGLE_TABLE.len(), 180);
        // 0deg, 90deg (index 45), 180deg (index 90) at Q14 amplitude 0x4000.
        let entry = |c, s| Ship3dAngleTableEntry { cosine: c, sine: s };
        assert_eq!(SHIP_3D_ANGLE_TABLE[0], entry(0x4000, 0));
        assert_eq!(SHIP_3D_ANGLE_TABLE[45], entry(0, 0x4000));
        assert_eq!(SHIP_3D_ANGLE_TABLE[90], entry(-0x4000, 0));
        // Every entry sits on the Q14 unit circle within rounding.
        for (i, e) in SHIP_3D_ANGLE_TABLE.iter().enumerate() {
            let mag = (i32::from(e.cosine).pow(2) + i32::from(e.sine).pow(2)) as f64;
            assert!(
                (mag.sqrt() - 16384.0).abs() < 2.0,
                "index {i} off the unit circle: {}",
                mag.sqrt()
            );
        }
        // The table feeds the matrix builder without an index-out-of-range.
        let angles = Ship3dMatrixAngles {
            angle_2f71: 10,
            projection_angle_2f6d: 45,
            angle_2f6f: 179,
        };
        assert!(build_ship_3d_projection_matrix(&SHIP_3D_ANGLE_TABLE, angles).is_some());
    }

    #[test]
    fn blood_prng_rtc_seed_duplicates_seconds_into_both_seed_bytes() {
        // `mov ah,al` before `mov cs:[0xAEE],ax` puts the seconds byte in both
        // halves of the seed word, and different seconds give different streams.
        assert_eq!(BloodPrng::seeded_from_rtc_seconds(0x2a).seed_word, 0x2a2a);
        assert_eq!(BloodPrng::seeded_from_rtc_seconds(0).seed_word, 0);
        let mut s5 = BloodPrng::seeded_from_rtc_seconds(5);
        let mut s6 = BloodPrng::seeded_from_rtc_seconds(6);
        assert_ne!(
            (0..8).map(|_| s5.next(0xffff)).collect::<Vec<_>>(),
            (0..8).map(|_| s6.next(0xffff)).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn blood_prng_is_deterministic_for_a_given_seed() {
        let mut lhs = BloodPrng {
            seed_word: 0x1234,
            a: 0x9a,
            b: 0x57,
            counter: 3,
        };
        let mut rhs = lhs;
        let lhs_seq: Vec<u16> = (0..64).map(|_| lhs.next(0xffff)).collect();
        let rhs_seq: Vec<u16> = (0..64).map(|_| rhs.next(0xffff)).collect();
        assert_eq!(lhs_seq, rhs_seq);
        assert_eq!(lhs, rhs);
        // A non-trivial seed must actually produce variation, not a constant.
        assert!(
            lhs_seq
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 1
        );
    }

    #[test]
    fn blood_prng_respects_modulus_range() {
        let mut prng = BloodPrng {
            seed_word: 0xbeef,
            a: 0x11,
            b: 0x22,
            counter: 0,
        };
        for modulus in [1u16, 2, 7, 100, 320, 0x8000, 0xffff] {
            for _ in 0..500 {
                assert!(
                    prng.next(modulus) < modulus,
                    "value out of range for {modulus}"
                );
            }
        }
        // modulus 0 returns the raw 16-bit word (no range reduction path).
        let _ = prng.next(0);
    }

    /// Both baked geometry tables must equal the image. `NAV_DESTINATION_POINTS`
    /// holding ten IDENTICAL points looks like a placeholder bug and is not: the
    /// shipped table really is ten copies of `(10200, 12100, 900)`, which is why
    /// the destination layout is the runtime-gated piece of the nav render.
    #[test]
    fn baked_geometry_tables_match_the_image() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        const DS_BASE: usize = 0xD420;
        let read_triples = |base: usize, n: usize| -> Vec<[i16; 3]> {
            (0..n)
                .map(|i| {
                    let at = base + i * 6;
                    let w = |k: usize| i16::from_le_bytes([exe[at + k * 2], exe[at + k * 2 + 1]]);
                    [w(0), w(1), w(2)]
                })
                .collect()
        };
        // DS:0x4F09 (nav points) and DS:0x5D98 (HUD pyramid vertices).
        assert_eq!(
            read_triples(DS_BASE + 0x4F09, NAV_DESTINATION_POINTS.len()),
            NAV_DESTINATION_POINTS.to_vec()
        );
        assert_eq!(
            read_triples(DS_BASE + 0x5D98, SHIP_3D_HUD_PYRAMID_VERTICES.len()),
            SHIP_3D_HUD_PYRAMID_VERTICES.to_vec()
        );
        // The nav table closes exactly on the angle table at DS:0x4F45, which is
        // what bounds its ten entries.
        assert_eq!(0x4F09 + NAV_DESTINATION_POINTS.len() * 6, 0x4F45);
        // And the vertices are the SAME BYTES as palette colours 192..255 — a
        // deliberate overlay the campaign already resolved, re-pinned here so a
        // change to either constant surfaces the alias instead of hiding it.
        let vert_base = DS_BASE + 0x5D98;
        assert_eq!(
            &exe[vert_base..vert_base + 192],
            &crate::palette::GAME_SCREEN_PALETTE_DAC[576..768]
        );
    }

    #[test]
    fn randomize_point_cloud_fills_all_records_and_consumes_three_rng_calls_each() {
        let mut prng = BloodPrng::default();
        let points = randomize_ship_3d_point_cloud(&mut prng);
        // Not `len() == THE_CONSTANT` (unfalsifiable — the builder used it): the
        // count and the record base are the randomizer's own immediates.
        assert_eq!(points.len(), SHIP_3D_POINT_CLOUD_LEN);
        if let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            let imm16 = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]) as usize;
            assert_eq!(
                imm16(SHIP_3D_POINT_CLOUD_COUNT_IMMEDIATE),
                SHIP_3D_POINT_CLOUD_LEN,
                "mov cx,imm at 0x9B6A"
            );
            assert_eq!(
                imm16(SHIP_3D_POINT_CLOUD_BASE_IMMEDIATE),
                SHIP_3D_POINT_CLOUD_BASE_DS as usize,
                "mov di,imm at 0x9B71"
            );
        }
        // Each x/y/z came from next(0xffff), so all are strictly below 0xffff.
        for point in &points {
            assert!(point.x < 0xffff && point.y < 0xffff && point.z < 0xffff);
        }
        // 3 * 0x3E8 = 3000 rng calls advanced the counter (3000 mod 256 = 184).
        assert_eq!(
            prng.counter,
            (3 * SHIP_3D_POINT_CLOUD_LEN as u32 % 256) as u8
        );
        // Not a degenerate all-zero fill.
        assert!(points.iter().any(|p| p.x != 0 || p.y != 0 || p.z != 0));
    }

    /// The nav-choice dispatch table decoded in audit-fixes #494: five near
    /// pointers at `cs:0x0F29` (file `0x8709`), one per choice, sitting directly
    /// after the `ret` of the routine that calls through them.
    ///
    /// The segment had to be SOLVED — the routine is reached by a near call, so
    /// no far pointer anywhere names its `cs`. Pinning the table to the image
    /// keeps that solution honest: if `cs` were wrong, these five words would not
    /// be five contiguous routine entries.
    #[test]
    fn nav_choice_handler_table_holds_five_contiguous_handlers() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        const CS_BASE: usize = 0x077E0; // file address of cs:0 for cs = 0x071E
        assert_eq!(CS_BASE, 0x600 + 0x071E * 16);
        let table = CS_BASE + 0x0F29;
        assert_eq!(table, 0x8709);

        let entries: Vec<usize> = (0..5)
            .map(|i| {
                let at = table + i * 2;
                CS_BASE + u16::from_le_bytes([exe[at], exe[at + 1]]) as usize
            })
            .collect();
        assert_eq!(entries, vec![0x8713, 0x872C, 0x87BD, 0x8848, 0x886C]);

        // One entry per choice, and the count is bounded by `cmp al,5 / jge`.
        assert_eq!(entries.len(), super::SHIP_3D_NAV_CHOICE_COUNT as usize);
        // Ascending and non-overlapping: these are five separate routines.
        assert!(entries.windows(2).all(|w| w[0] < w[1]));
        // Every handler tests the phase cell `[0x2565]` early on
        // (`test byte [0x2565],1` = f6 06 65 25 01). NOT at a fixed offset: the
        // handler at 0x872C does `push es / mov es,[0x6726] / mov si,0x2b13`
        // first, so requiring it at byte 0 or 1 fails on a correct decode -- the
        // first version of this test asserted exactly that and was wrong.
        let phase_test = [0xF6, 0x06, 0x65, 0x25, 0x01];
        for e in &entries {
            assert!(
                exe[*e..*e + 24]
                    .windows(phase_test.len())
                    .any(|w| w == phase_test),
                "handler at {e:#07x} tests the phase cell in its opening"
            );
        }
        // The table begins immediately after the dispatcher's `ret` @0x8708.
        assert_eq!(exe[0x8708], 0xC3);
    }

    /// The panorama auto-turn's constants, pinned to the image rather than
    /// restated (audit-fixes #497). The last assertion is the important one: it
    /// encodes WHY the port may model frames while the binary counts degrees.
    #[test]
    fn panorama_auto_turn_immediates_match_the_image() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let word = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]);

        // `cmp ax,0xb4` @0x9748 then `sub ax,0x168` @0x974D -- the shortest-distance fold.
        assert_eq!(word(0x9749), super::SHIP_3D_PROCEDURAL_HALF_TURN);
        assert_eq!(word(0x974E), super::SHIP_3D_PROCEDURAL_FULL_TURN);
        // thresholds and steps
        assert_eq!(exe[0x9754] as u16, super::SHIP_3D_PROCEDURAL_CLOSE_ANGLE_THRESHOLD);
        assert_eq!(exe[0x9764] as u16, super::SHIP_3D_PROCEDURAL_TARGET_LIST_THRESHOLD);
        assert_eq!(exe[0x977E] as u16, super::SHIP_3D_PROCEDURAL_TARGET_LIST_STEP);
        assert_eq!(exe[0x97C6] as u16, super::SHIP_3D_PROCEDURAL_AUTO_ROTATE_STEP);
        // the cursor ring: `add cx,0x5a0` @0x979D, wrap `add bx,0x5a0` @0x9807
        assert_eq!(word(0x979F), super::SHIP_3D_PROCEDURAL_MOUSE_RING);
        assert_eq!(word(0x9809), super::SHIP_3D_PROCEDURAL_MOUSE_CENTER_X);
        assert_eq!(
            super::SHIP_3D_PROCEDURAL_MOUSE_RING,
            super::SHIP_3D_PROCEDURAL_MOUSE_CENTER_X,
            "one value in two roles: ring origin and ring modulus"
        );
        // `sub ax,0xa0` @0x97F0 and `and word [0xa2a],0xfff8` @0x97F6
        assert_eq!(word(0x97F1), super::SHIP_3D_PROCEDURAL_ROTATION_OFFSET_BIAS);
        // `and word ptr [0xa2a], 0xfff8` is `83 26 2a 0a f8` -- the `83 /N ib`
        // form, whose immediate is ONE BYTE, SIGN-EXTENDED. Reading a word here
        // yields 0xF80A, which is the next instruction's bytes, not a mask.
        assert_eq!(&exe[0x97F6..0x97F8], &[0x83, 0x26], "83 /4: and r/m16, imm8");
        assert_eq!(
            exe[0x97FA] as i8 as i16 as u16,
            super::SHIP_3D_PROCEDURAL_MOUSE_ALIGN_MASK
        );

        // THE FRAME IS HALF A DEGREE-COUNT: `shr bx,1` (d1 eb) @0x97E1 immediately
        // before `mov [0x2795],bx` (89 1e 95 27) @0x97E3. This is what licenses the
        // port's `angle * 2` when it hands frames to degree-space arithmetic.
        assert_eq!(&exe[0x97E1..0x97E3], &[0xD1, 0xEB], "shr bx,1");
        assert_eq!(&exe[0x97E3..0x97E7], &[0x89, 0x1E, 0x95, 0x27], "mov [0x2795],bx");
        // and the panorama really is 180 frames of 2 degrees
        assert_eq!(
            super::SHIP_3D_PROCEDURAL_FULL_TURN / 2,
            super::SHIP_3D_PROCEDURAL_HALF_TURN
        );
    }
}

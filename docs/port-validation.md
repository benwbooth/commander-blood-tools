# Port validation matrix — every module vs the original assembly/data

Standing directive: systematically validate each ported function/struct against
BLOODPRG.EXE's assembly and the game's data files. Status values:

- **ASM** — behavior derived from cited disassembly (address in the code/labels.csv).
- **DATA** — a faithful parser/interpreter of a game file format, cross-validated
  (decoded output matches known-good content, e.g. plays/renders correctly vs captures).
- **CAPTURE** — matched against DOSBox captures of the real game (screen-level truth).
- **APPROX** — reconstruction consistent with evidence but not derived from a specific
  routine; must not be presented as decoded. Listed with what would settle it.
- **UNVERIFIED** — porter invention or unchecked guess. Highest priority to fix/verify.

The matrix is maintained by hand as modules are audited; every status change needs the
evidence in the row. Re-audit pass 1: 2026-07-22..23.

| Module | What it is | Status | Evidence / gap |
|---|---|---|---|
| vm.rs `VmMachine` | script bytecode executor | **ASM** | every handler cited (dispatch 0x142D0; 0x6462/0x6830/0x65EB/0x6596/0x6588/0x6863/0x6946/0x6902/0x6B06/0x6AA7/0x64xx); flow verified vs live-oracle tutorial lines |
| vm.rs `decompile_script` | listings generator | ASM | same semantics as VmMachine; listings complete for SCRIPT1-5 |
| vm.rs walk/LineState | token scanner | ASM+DATA | descriptor table 0x6F18 transcribed; A6 layout decoded |
| vm.rs dos_save | blood.sav I/O | **ASM** | save path 0x1C3F / load 0x1CBD; block order+sizes cited; round-trip test. Tail work-buffer block written empty (rebuilt state) |
| bas_vm.rs / concept_menu.rs | conversation menus | DATA | 0xA3/0xA6 BAS blocks decoded; labels verified vs live captures (menu tree) |
| script.rs | speech-event assembly | DATA | offsets match VM Text events exactly; actor talk-ref +58 verified vs DEB names |
| descript.rs | DESCRIPT.DES records | DATA | drives intro/TV/music; verified against real-game behavior |
| hnm.rs | HNM video decoder | DATA+CAPTURE | frames match DOSBox captures (logos/montage checkpoints) |
| tbbig.rs | bridge panorama | CAPTURE | pixel test vs live game (mean_abs 2.58) |
| bridge.rs | bridge steering/stations | ASM | 0x9656 state machine decompiled; BRIDGEPROBE replays |
| font.rs GAME_FONT | proportional dialogue font | ASM | byte-identical to EXE tables 0x14C22/0x14CD2/0x14D28 (test) |
| font.rs BoldConsoleFont | subtitle/console font | ASM | tables 0x1451A/0x145CA; subtitle renderer 0x3630 uses it (decoded) |
| engine.rs subtitle draw | reveal + colors | ASM | 0x3630 colors 0xFF/0xFE/0xFD (baked palette greens); reveal pump 0x93F8 |
| engine.rs chatter | honk burble | ASM | 0xB898: tb.snd clip 7+rand(0..9), 4-tick throttle |
| palette.rs | baked game palette | DATA | extracted from file 0x12F78 |
| snd.rs / audio.rs | SND banks + playback | DATA | voices/clips play; clip-index mapping decoded (0x661E) |
| lbm.rs | LBM/PBM images | DATA | CHART.FD/FRIGO.FD/fd rooms decode correctly |
| ext.rs | world files | DATA(partial) | framing validated (magic/nodes/objects/payload refs); record semantics under study — needs the consumer load path |
| levels.rs | level manifest | ASM+DATA | filename table at 0xCF04 decoded |
| ship3d.rs nav projection | destination projection | ASM | 0x9B98 decompiled (matrix at 0x2F95) |
| ship3d.rs pyramid render | OPTION menu visuals | **APPROX** | flat-shaded stand-in; the real render path (manu3 3D + dither) not ported. Settle: decode the manu3 render loop or capture the real OPTION screen |
| manu3.rs | menu 3D data | DATA(partial) | camera-pan entries decoded; item sprites/RLE not |
| engine.rs console band | intro/tutorial pyramid band | CAPTURE | pixel-exact harvest from native DOSBox raws (static across times) |
| engine.rs hand cursor | pointing-hand | CAPTURE(atlas)+**APPROX**(pose) | sprites harvested from live bridge captures; pose-by-nearest-capture approximates the real manu3 3D hand render. Now the ONLY cursor, drawn on every screen at the mouse position (host crosshair removed) |
| engine.rs intro flow | logos/montage/credits | CAPTURE+DATA | DESCRIPT present record + real-args DOSBox captures (rows 69/79 credits, band rows 99..200) |
| engine.rs TV | broadcast channels | DATA | 7 self-identified Sequence records; chained clips+music+cues |
| engine.rs telephone/cryobox | console screens | DATA+CAPTURE | bappel/character sprites; oracle-observed flows |
| engine.rs cyberspace | tunnel minigame | **APPROX** | presentation from real assets; goal decoded from SCRIPT2 text (BIOXX/BIONIUM) but the interaction logic is a stand-in. Settle: the cyber .ext consumer + input handler |
| engine.rs OPTION menu | 3D pyramid menu | **APPROX** | see ship3d pyramid render; item glyphs await manu3 sprite decode |
| engine.rs world visit | on-planet screens | DATA+APPROX | rooms/objects from decoded data; click=talk + room-step wiring is an interpretation. Settle: on-planet input handler in asm |
| engine.rs nav view | star chart + list | CAPTURE+ASM | CHART.FD bg; tablo2 music toggle 0x886C; center-relative steer ~0x102/0x216 |
| save.rs | port save format | n/a (port-own) | DOS interop via vm dos_save |
| progress.rs / entity.rs | progression FSM | DATA(partial) | entity records decoded; completion rule (all visited → ending) is an interpretation |
| recomp/* | interpreter runtime | oracle | separate: runs the real EXE for cross-checks |

## Active fix queue (from the matrix, user-reported first)
1. [x] Host crosshair removed; hand = the only cursor, all screens (this pass).
2. [ ] Hand tracking feel: sprite anchored at the exact hotspot vs the real game (verify against a capture with the DOS mouse at a known position).
3. [ ] OPTION menu render fidelity (largest APPROX on screen): decode the manu3 render or re-source from captures of the real OPTION screen (needs input injection — xdotool on the DOSBox Xvfb).
4. [ ] Cyberspace interaction (BIOXX touch loop) from the cyber consumer.
5. [ ] On-planet input handler decode to replace the interpretation.
6. [ ] ext.rs record semantics via the consumer load path.

# New Ryu Shinku Hadoken in World Super Turbo

World `ssf2t` 940223 in MAME 0.289, with New Ryu versus New Ken. The new timings use Turbo 2, with Turbo 1 controls for motion recognition. The earlier audit at `9646b61` uses the default World Turbo 1 EEPROM. That audit includes live meter building and a larger set of normal cancel input recipes. The new test sets full meter directly so the two input phases and spacings can be compared from the same round. [ROM and cabinet setup](ryu-st-turbo2-contact-evidence.md#replay-setup) · [Earlier source evidence and replay](https://github.com/johnstreetfighter/sf1-port/tree/9646b61/docs/research/evidence/ryu-super-turbo) · [Shinku work #143](https://github.com/johnstreetfighter/sf1-port/issues/143)

## Raw command recognition

The exact final-button window is `0…+10` game ticks from recognition of the last Forward. The game checks fresh punch presses and releases in that window. All three punch strengths enter the same Super path. Old Ryu has no Super meter or Shinku in the prior comparison.

| World opcode location | Behavior |
|---|---|
| `$07fc5a–$07fc72` | Tick the preceding direction timer. Recognize the last Forward from relative input `+0x3a6 & 0x0f == 2`. Advance motion stage `+0xa0` to `12` and load timer `+0xa1 = 12`. |
| `$07fc78–$07fc8c` | On the completion tick, test punch press and release edges with mask `0x70`. |
| `$07fc8e–$07fca8` | On each later tick, predecrement the timer before testing the same edge mask. |
| `$07fcaa–$07fcc4` | LP, MP and HP choose the same strength parameter `4`. The edge is eligible while the remaining timer is at least 2. |
| `$07fcc6–$07fd06` | Check the live projectile pointer and action/cancel gate, then set `+0x88 = 8` and enter the Super action. The completed direction alone grants the recognition window. Current state determines whether the game can spend it. |
| `$0044d0` and `$0044de` | Detect positive and negative button edges from current `+0x392` and previous `+0x394`. |

In one Turbo 2 replay the last Forward loads the timer at tick 217. LP at exact tick 227 succeeds with 2 remaining and sets the Super action. Screen samples record that LP at displayed tick 228. A second phase gives the same successful +10 interval, while +11 becomes ordinary Hadoken with Super meter intact. Turbo 1 replays also give +10 success and +11 failure. The write trace anchors acceptance to the CPU write inside the display sample. The initial direction phases use other timer writes, including `$07fd10`. Their full leniency and shortcuts were outside this final-button check.

## Normal cancels

The normal-cancel countdown is a distinct byte. `$07f044` loads `+$2b0 = 7` for New Ryu's ordinary grounded normals, while `$07f03e` loads the ordinary special-cancel byte `+$195 = 6`. The Super branch of `$0045b4–$0045c6` tests `+$2b0`. The ordinary special branch tests `+$195`. Input recognition precedes the normal-update decrement at `$078dea`. The Super byte advances during the opener's ordinary freeze. The tested crouching LK and MK still accept on action-entry offset +7 with a Super byte of 1, and fail the next tick after it expires. In a whiff replay, Super acceptance at +7 occurred with the ordinary special-cancel byte already zero.

| Turbo 2 route | Normal entry | Opener contact | LP selection and remaining Super gate | First Shinku animation | Flash | First Super life write |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| 2LK clean, 32 px, phase 0 | 216 | 220 | `221 / 3`, `222 / 2`, or `223 / 1` | 235 | 244 | 7 |
| 2LK clean, 32 px, phase 1 | 217 | 221 | `224 / 1` succeeds. `225 / 0` declines. | 236 | 245 | 8 |
| 2MK crouch-guarded, 32 px, phase 1 | 217 | 221 | `224 / 1` succeeds. `225 / 0` declines. | 236 | 245 | 8 |
| 2LK whiff, 120 px, phase 0 | 216 | — | `223 / 1` succeeds. `224 / 0` declines. | 223 | 232 | 251 |

The Super selection changes state while the opener's attacker freeze is still active. Its first animation and 9-tick pre-flash clock start after that freeze. The accepted clean phase-zero 2LK replays put the first Super life write 43 ticks after the opener's life write. That interval includes the 18-tick Super freeze, which holds both players. Several later projectile hits can connect after the cancel because the opener's push moves Ken into the projectile's later boxes. [Selected CPU writes](ryu-st-shinku-hadoken-writes.tsv) · [Earlier contact and whiff windows](https://github.com/johnstreetfighter/sf1-port/blob/9646b61/docs/research/evidence/ryu-super-turbo/arcade-super-windows.tsv)

The first active 2LK rectangle has center (+56, +9) and radii (33, 12). The 2MK rectangle has center (+58, +9) and radii (40, 12). These values are relative to right-facing Ryu's axis and the floor. In both crouch-guard capture phases the fighters stay 32 px apart. Ken's foot box has center (−6, +11) relative to his left-facing axis and radii (22, 12). Both kicks intersect this box at action +4 and load 14 ticks of defender freeze.

### Grounded normal countdowns

In 18 Turbo 2 trials with neutral New Ken, every ordinary grounded variant loaded `+$2b0 = 7` on the attack-state write and wrote `6, 5, 4, 3, 2, 1, 0` at offsets +1 through +7. Close and crouching normals used a 32 px gap. The first pose identifies the close, far or crouching action. The contact column is the first write of 14 to Ken's ordinary freeze byte.

| Button | Close first pose · contact | Far gap · first pose · contact | Crouch first pose · contact |
| --- | --- | --- | --- |
| LP | `0x1100fe` · +3 | 65 px · `0x110162` · +3 | `0x11078e` · +3 |
| MP | `0x1101c6` · +4 | 71 px · `0x11028a` · +4 | `0x1107f2` · +4 |
| HP | `0x11031e` · +4 | 81 px · `0x1103b2` · +6 | `0x110886` · +4 |
| LK | `0x110446` · +6 | 41 px · `0x1104da` · +5 | `0x11091a` · +4 |
| MK | `0x110556` · +4 | 41 px · `0x1105ea` · +8 | `0x11097e` · +4 |
| HK | `0x110666` · +8 | 96 px · `0x1106fa` · +7 | `0x110a12` · +4 |

Close HK and far MK first contacted after the gate expired in these fixtures. Far HK first contacted on the tick of the zero write. The isolated-normal trials measured the shared byte and contact clocks. Final-input acceptance at +7 and rejection at +8 were directly tested for 2LK and 2MK. The earlier Turbo 1 replays separately show a successful Shinku combo after close LP, MP, HP, LK or MK, and after crouching LP, MP or HP. In its close HK and sweep examples, Shinku started and the projectile missed the airborne defender. Those samples used their own input and contact conventions. [Earlier normal-first and buffered recipes](https://github.com/johnstreetfighter/sf1-port/blob/9646b61/docs/research/ryu-super-turbo-audit.md#shinku-hadoken-input-windows)

## Startup, freeze and projectile

In phase zero with all three punch strengths, Ryu's first Shinku animation pointer is `0x111382` at exact game tick 216. The first Super flag write and meter spend occur at 225, the flag clears at 243, and the projectile is allocated at 244. The exact relative schedule is `0` action, `+9` flash and spend, `+27` flag clear and `+28` projectile. The flag remains asserted for 18 game ticks. `+$47` is a different freeze counter used for contact. It remains zero on Ryu through the sampled clean projectile contacts.

Ryu's neutral animation write occurs at action +69, 41 ticks after projectile spawn, in the four raw Turbo 2 fixtures. The projectile can finish after Ryu is neutral. At 32 and 90 px, holding Back moves Ryu from X 552 to 550 on that same tick 29. A one-display Back input at event 2177 fails, at 2178 moves at exact tick 29, and at 2179 moves at tick 30.

Recovery follows the action animation records. At spawn the current record is `$1113ca`, duration 4 and flag 2. At spawn +4 the action enters `$1113e2`, duration 36 and flag 0. At +40 it enters `$1113fa`, flag `0x8000`. The next tick `$080680` reads the signed action flag `+$14` and takes the terminal branch to `$0044ec`. The normal animation advance uses `$002ee2`. The first movement write is observed at callback PC `$00439e`.

The code writes `+$1fa = 1` at `$0805e6`, and `$027fa8` clears meter `+$2b4`. `$080600` clears the Super flag after the `+$2ad` freeze object has released. The projectile allocation at `$08061e` and type write at `$080624` precede its first collision on the spawn tick. In the recorded slots the shot object is `0xff9ce2`. The initial shot life word is `0x0100`. In the five-contact 90 px probe it is marked `0xfffe` on the fifth contact. The opponent's health write is at `$0bebac`.

The source shot starts 93 px forward of Ryu and 54 px above his ground position. At right-facing spawn, X is 645 and Y 94 with Ryu at X 552 and Y 40. Flight advances by 5 px per unfrozen projectile update. During hits the projectile loads its own 10-tick freeze and Ken loads 14. Ken is still in reaction when the projectile advances to the next overlap. At standing gaps 32 and 90, the next hit occurs 10 game ticks later and the shot has moved one 5 px step. The final interval at 150 px takes 11 ticks and two flight steps. At 240 px it takes 12 ticks and three flight steps.

| Projectile advances from spawn | Attack ID | Right-facing box center offset X/Y | X/Y radius |
| --- | ---: | ---: | ---: |
| 0–2 | 2 | `−16 / 0` | `31 / 13` |
| 3–8 | 3 | `0 / 0` | `15 / 14` |
| 9 onward | 1 | `−3 / 0` | `21 / 12` |

The box table stores unsigned-facing-convention X offsets 16, 0 and 3. Facing byte 0 uses the negative of that value in world X. The narrow box at advance 3 explains the range dependence in the clean cases. At 32 px the shot hits on advances 0, 1 and 2 and then clears Ken's changing hurtboxes. At 90 px it continues to connect after the box change. The five-hit budget must survive the near whiff after hit three.

The frozen hurtboxes also change between hits. The table gives the raw box record as `ID, X, Y, X radius, Y radius`, where X uses the left-facing Ken convention. Ken's axis and floor height stay unchanged through the listed contacts. The first three rows match at both 32 and 90 px. The last two apply to the 90 px replay.

| Contact | Head | Torso | Legs |
| ---: | --- | --- | --- |
| 1 | `1, −4, 79, 12, 8` | `1, 6, 52, 20, 21` | `1, 6, 14, 20, 16` |
| 2 | `27, 1, 75, 14, 9` | `19, 10, 47, 21, 21` | `18, 9, 16, 21, 17` |
| 3 | `22, 15, 78, 10, 10` | `15, 13, 50, 20, 23` | `15, 10, 17, 20, 19` |
| 4 | Same as contact 2 | Same as contact 2 | Same as contact 2 |
| 5 | `24, 42, 76, 10, 10` | `17, 33, 47, 20, 23` | `16, 13, 17, 20, 19` |

| Actual gap at activation | Exact damage-write ticks from action start | Damage in this run | Contacts |
| --- | --- | --- | ---: |
| 32 px | `28, 38, 48` | `14, 14, 14` | 3 |
| 90 px | `28, 38, 48, 58, 68` | `16, 16, 14, 14, 14` | 5 |
| 150 px | `34, 44, 54, 64, 75` | `14, 14, 14, 13, 14` | 5 |

At 32 px the last hit stays grounded. Ken's freeze reaches zero at last contact +14. He advances on +15 through +25 by `4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1` px, 30 total, and returns to neutral at +26. At 90 and 150 px the first four hits keep him grounded and leave his integer X unchanged between contacts. The fifth launches. In the original positioning probes Ken reached the right side during the fifth-hit reaction, so their total displacement is stage-limited. The first 16 horizontal advances on that reaction are `4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4, 5` px. These are Ken-specific measured outcomes for the recorded shot and stage state.

An uncapped launch replay used Ryu X 472 and Ken X 562, which the left stage boundary adjusted to a 74 px actual gap. Ken stayed at X 562 through hit five. At +1 his Y changes from 40 to 56. His freeze ends at +14. The first movement at +15 reaches Y 60, the arc peaks at Y 97 beginning at +29, and first reaches the floor at +52. Horizontal travel on +15…+52 is 156 px, with 5 px on +22, +30, +38 and +46 and 4 px otherwise. X holds on +53…+56. The rebound at +57…+74 alternates 4 and 3 px for another 63 px, reaching Y 50 at +63 and the floor at +73. Landing slides `2, 2, 1, 2, 1, 1, 1` px on +75…+81. Total displacement is 229 px. Ken is down by sampled +91, in the wakeup animation sequence by sampled +97, and returns to neutral at exact +120. The vertical path matches the sampled near HP Red trajectory at one tick earlier from contact. The horizontal velocities and wakeup boundary differ. [Red Hadoken comparison](ryu-st-red-hadoken-evidence.md#hp-red-clean-launch-rebound-and-wakeup)

The displayed game combo counter can return to zero during the 150 px sequence. Count each exact health decrement separately from whether the fifth hit remained a true combo in that setup. Damage varies across phases. The same 32 and 90 px tests with LP, MP and HP agree on the number of contacts. [Selected CPU writes](ryu-st-shinku-hadoken-writes.tsv)

### Guard

| Ken posture | Actual fighter gaps at spawn | Contacts |
| --- | --- | --- |
| Standing guard | 36, 92, 152 px | 3, 5, 5 |
| Crouching guard | 32, 90, 150 px | 1, 5, 5 |

All guarded contacts cost 3–4 HP and stay grounded. In these six probes, consecutive hits arrive 10 game ticks apart. Ken uses the neutral standing boxes throughout standing guard, including after successive contacts. His crouch-guard boxes use the same raw field order as above. The head is `14, −14, 51, 12, 9`, the torso `8, −6, 33, 22, 12` and the legs `8, −6, 11, 22, 12`. In the close crouch replay the first shot's left edge is X 598 and Ken's torso right edge is X 600. Their Y ranges intersect. One flight step moves the shot's left edge to X 603, beyond the torso.

The last hit loads 14 defender freeze. In the multi-hit guard fixtures it reaches zero at +14. Push starts at +15, follows the same 20 steps as HP Red guard, and covers 78 px. Ken's guard-ready state is first written at +35. At the 32 px one-contact crouch guard the corresponding offsets are +15, +16 and +36. Standing-guard total is measured through the guard-ready write, since continuing to hold Back also makes Ken walk after release. [HP Red guard trajectory](ryu-st-red-hadoken-evidence.md#hp-red-guard-reaction)

## Replay

Use the New Ryu versus New Ken boot events in the pinned audit. Retain the initial menu events through 2040 and the default EEPROM. At the end of display sample 2070, set cabinet speed selector `0xff83a9` to `2` and pacing threshold `0xff8cd3` to `0x60` for Turbo 2. Set Ryu and Ken integer X at `+0x06` to 552 and `552 + gap`, and clear their X fractions at `+0x08`. At the end of sample 2100 set Ryu's `+0x2b4` meter byte to 48. Use `N = 2120` for phase zero or `2121` for phase one. The input schedule uses inclusive display event ranges.

| Input | Event range |
| --- | --- |
| `2` | `N−10…N−9` |
| `3` | `N−8…N−7` |
| `6` | `N−6…N−5` |
| `2` | `N−4…N−3` |
| `3` | `N−2…N−1` |
| `6` | `N…N+delay+1` |
| `LP`, `MP` or `HP` | `N+delay…N+delay+1` |

Tap Ryu `+0x88`, `+0xa0/+0xa1`, `+0x1a`, `+0x1fa` and `+0x2b4`, plus Ken `+0x2a`. In the taps read the running game tick at `0xff801d` and the CPU PC. The memory callback sees the old byte or word before a write. Extract the incoming byte from the value according to the 68000 byte mask. For the projectile, capture the first write from `$080624`, resolve the object from the write's address, and follow its X/Y, freeze, animation pointer and hitbox pointer. The selected trace stores game-counter bytes modulo 256. Temporary full logs are in the `st-shinku-*` directories under `%TEMP%/ssf2-ryu-audit`. [Original boot script and replay fixtures](https://github.com/johnstreetfighter/sf1-port/tree/9646b61/docs/research/evidence/ryu-super-turbo) · [Hitbox record layout](https://github.com/peon2/fbneo-training-mode/blob/master/hitboxes/st-hitboxes.lua#L39)

For the normal-cancel fixtures, queue `2` at `N−8…N−7`, `3` at `N−6…N−5`, `6` at `N−4…N−3`, `2` at `N−2…N−1`, and `3+K` at `N…N+1`. Send a one-display `6+LP` at `N+4`, `N+5` or `N+6`. Hold Ken `1` from `N−4` through `N+60` for the guard probe. Tap Ryu's `+$195`, `+$2b0`, and `+$392` in addition to the raw command fields. Use 32 px for clean and guard, 120 px for the LK whiff.

For the isolated countdowns, use the World Turbo 2 setup above at each table gap and leave Ken neutral. Hold the chosen button on display events 2120 through 2122. For crouching attacks also hold Down on 2114 through 2122. Tap Ryu's state at `+$03`, animation pointer at `+$1a`, and Super gate at `+$2b0`, plus Ken's freeze at `+$47`. Read the tick byte at each write and subtract the tick of the state-10 write modulo 256. The first pose write identifies the action. Read gate `+$2b0` from the high byte of the incoming aligned word. Scratch logs use `st-shinku-normal-loads` under `%TEMP%/ssf2-ryu-audit`.

The uncapped launch replay uses the raw command schedule with delay 0 and the adjusted left-stage positions described above. Tap Ken's state, X, Y and freeze at `+$03`, `+$06`, `+$0a`, and `+$47`. In the selected trace, the game-PC column on write rows is the callback PC after the opcode. The table in this report gives the opcode addresses. Recipe keys starting `lk-0` and `mk-1` label the clean/whiff LK and guarded MK cancel sweeps. The rest follow `Turbo-gap-phase-delay-button`. `launch-74` is the uncapped five-hit launch replay.

For guard, hold Ken `4` or `1` from `N−1` through `N+105` during the raw command replay. The starting gaps are 32, 90 and 150 px. Standing guard movement makes the spawn gaps 36, 92 and 152 px. `guard-stand` and `guard-crouch` trace keys retain these starting gaps.

For the recovery check, keep Ken neutral and hold Ryu Back from events 2168 through 2200, or queue one event at 2177, 2178 or 2179. Use delay zero and the phase-zero Turbo 2 recipe. Tap Ryu X at `+$06` and compare the first displacement with the neutral animation write. These evidence rows use `ready` recipe labels.

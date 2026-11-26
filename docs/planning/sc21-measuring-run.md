# SC21: SF1 and SF1+ stronger Fair CPU measuring report

2026-11-26. Version `1.5.33`. Ticket [SC21 #213](https://github.com/johnstreetfighter/sf1-port/issues/213).

191 of 192 new-versus-old cells meet the nominal win and exact weighted-idle bars; 130 of 144 adjacent-level cells meet the nominal higher-level win bar. 15 win bars miss under the all-attempts denominator, including 0 additional misses beyond nominally passing cells.

**Readiness:** The recorded win/activity gate is unmet: nominal and/or all-attempts win failures are listed below. These measurements do not supply the required maintainer playtest or whole-cast enablement approval. The developer-only stronger CPU remains off by default. Its CPU wire version is unchanged. [SC24 #216](https://github.com/johnstreetfighter/sf1-port/issues/216) retains the whole-cast enablement decision and maintainer approval.

## Source and implementation

Every final row uses measured source [13a3e21](https://github.com/johnstreetfighter/sf1-port/commit/13a3e219b131e924e469d8d6f29d6369c8ee177e) (`13a3e219b131e924e469d8d6f29d6369c8ee177e`), frozen executable SHA-256 `95a31d9ba80719d77d7317fed5ef832338f20509dc6f5fd4f4f38cf6f200efcc`. Delivery source: `fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`.

All 192 primary weighted-idle bars pass. The 15 remaining measured misses are win bars.

The rollout brings the twelve tournament fighters in the 1987 and SF1+ era styles into the stronger Fair CPU plans, pressure, punishes and habit memory under overhaul presets. Native Arcade and World Tour control and replay continuity remain behind the existing developer switch.

- Native control binds the actual opponent style and player move table in both cabinet orientations. Mode 4 preserves independent human and CPU pad and recognizer history. Faithful 1987 and switch-off play retain their prior ownership.
- Baseline plans use permitted routes, throws, on-hit pressure, knockdown setups, guarded jump-ins and native Gen, Birdie and Lee Advance specials. The 1987 style disables routes, combos, cancels and throws; SF1+ respects the independent rules. Bookless pairs retain legal ordinary fallback.
- Baseline feedback uses felt own position, readiness and facing while preserving delayed opponent information. Guard direction, reach and hit feedback, special button timing and bounded approach readiness are corrected. The old scorer and CPU wire identity remain for SC24.
- Joe SF1+ Normal uses ordinary fallback after the original admission rolls only when a dedicated knockdown Meaty plan wins admission. Embedded poke continuations remain. The 28-cell screen gained 51 wins, repaired one cell, introduced no new failed cells, and retained two misses. All 6,720 rows match the final owned executable.
- Replay takeover preserves recorded control until external ownership is needed. Recorded handoffs restore recognizer history; fresh control starts with clean input history. Rewind restores the original configuration and snapshot.

The comparison baseline is the committed version 1.5.32 source [ea4da73](https://github.com/johnstreetfighter/sf1-port/commit/ea4da73cb5db6d2c351d0bf8c05846ec5ba58441), after the completed SC20 SF6 rollout and its evidence/report delivery. This report does not infer another native bug fix from CPU scores or classify an unfinished row as a proven native hang.

## Method and complete accounting

The grid covers all twelve tournament fighters in Arcade1987/Standard and SF1Plus/Standard: 24 pairs. Each pair has 8 primary cells (four levels on both sides), 6 adjacent-level cells, and 1 old/old Very Difficult control: 360 cells and 86,400 attempted matches. These are Local two-pad Fair CPU comparisons under Definitive Overhaul. They do not compare against Capcom's native Arcade CPU; native Arcade activity and other preset rule gates require separate explicit diagnostics below.

**86,400 attempted; 86,400 completed; 0 engine-stopped; 0 unfinished; 0 completed draws.** Seeds 1-240 occur exactly once per cell across raw match rows and explicit failure lines. Worker threads: 2.

| Group | Cells | Nominal pass / miss | Attempted | Completed | Stopped | Unfinished | Completed draws | Pessimistic win misses |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| primary | 192 | 191 / 1 | 46080 | 46080 | 0 | 0 | 0 | 1 |
| adjacent | 144 | 130 / 14 | 34560 | 34560 | 0 | 0 | 0 | 14 |
| old-old | 24 | not graded | 5760 | 5760 | 0 | 0 | 0 | not graded |

Completed wins/losses/draws require `ended=yes`. Unfinished rows are separate, including a `winner=-` row, and prevent a nominal pass. Primary nominal bars require wins at least half of all recorded rows and strictly lower idle; adjacent levels require higher-level wins greater than half. The separate pessimistic win bar includes all 240 attempted seeds, including every stop and unfinished match, without relabeling them as completed losses or draws. A pessimistic win pass does not waive idle or unfinished failures.

Idle and attack rates use all recorded live frames. Each two-decimal metric is parsed as exact Decimal hundredths, then multiplied by integer frame counts. Strict idle decisions compare integer weighted sums before any division; mathematically equal idle cannot pass from float rounding. Division and two-place rounding are for display only. Stops have no live rates. Missing/duplicate seeds, invalid identity/metrics, unexpected grids and partial files are rejected; cached summary JSON does not establish any result.

Levels 1-4 mean Easy, Normal, Difficult and Very Difficult. First/second means new/old in primary rows, higher/lower in adjacent rows, and P1/P2 in old/old rows.

Equivalent grid commands reconstructed from manifests, not an invented invocation history:

```text
rtk proxy python target/sc21/measure_sc21.py final --frozen "D:\sf1-port-storage\build\original-target\sc21\bin\measurement-13a3e219b131-current.json" --out "C:\Users\George\Projects\sf1-port\target\sc21\final-13a3e219b131" --threads 2
rtk proxy python target/sc21/measure_sc21.py controls --frozen "D:\sf1-port-storage\build\original-target\sc21\bin\measurement-ea4da73cb5db-baseline.json" --out "C:\Users\George\Projects\sf1-port\target\sc21\baseline-controls" --threads 2
rtk proxy python target/sc21/measure_sc21.py controls --frozen "D:\sf1-port-storage\build\original-target\sc21\bin\measurement-13a3e219b131-current.json" --out "C:\Users\George\Projects\sf1-port\target\sc21\controls-13a3e219b131" --threads 2
```

## Switch-off and established-era compatibility

The 434 controls preserve all 388 SC20 cells (12 SF6 switch-off pairs, 14 Ryu SF6 level/adjacent cells, 118 SF5, 120 SF4, 96 SF3 and 28 representative SF2/SFZ cells), add 22 non-Ryu SF6 cells with each stronger side, and add 24 SF1/SF1+ switch-off cells. Every control file accounts for 40 attempted seeds. Both complete grids were revalidated from raw data and all 434 TSVs, including stop lines and unfinished rows, compared byte for byte.

**434 of 434 control files match byte for byte; 0 differ.** Compatibility here means unchanged measured output for these cases. It does not turn their baseline stops or unfinished rows into successes.

| Controls | Attempted | Completed | Stopped | Unfinished | Completed draws |
| --- | --- | --- | --- | --- | --- |
| Baseline | 17360 | 17162 | 198 | 0 | 0 |
| Current | 17360 | 17162 | 198 | 0 | 0 |

The SF1/SF1+ switch-off subset is reported separately so failures outside the new rollout are visible:

| SF1/SF1+ switch-off subset | Cells | Attempted | Completed | Stopped | Unfinished | Completed draws |
| --- | --- | --- | --- | --- | --- | --- |
| Baseline | 24 | 960 | 960 | 0 | 0 | 0 |
| Current | 24 | 960 | 960 | 0 | 0 | 0 |

### All 434 paired control files

| Raw file | Baseline completed / stopped / unfinished | Current completed / stopped / unfinished | Raw SHA-256 (identical) |
| --- | --- | --- | --- |
| ryu__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c2bad074b3dcbeaf7b86818f1833c43265c6313a72ec55d27a7aea00709b9785 |
| ken__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3df49539d37fa8e68dd47c22a4cc95d11c90c68617546f3059bee1d52f3cdb32 |
| retsu__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | aee7a11f1d48ebb83073597111108880e55915a646440d9bae0bebd12578138a |
| geki__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d990531d3dad95a97374300ea98892fe0c28d56203c96d1b5ba8b170457562cb |
| joe__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b1b029766dcd69f1226d7043f2bd3aae83f90a84b9ba4238ad713218d8481851 |
| mike__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | aec737b81a4e2e3c82f8ef5e8501a0b6c5e099c72b192aa460758ceb9c55522b |
| lee__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 257a969eb776a46039521ac93edab9b6271514ea2f70079306d36261b638366f |
| gen__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 89c2155d9b020d97212c3b3b7b29ea21af5c321eece722ad37d245937e660310 |
| birdie__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a3a06f3fffe2f336d4c30659a6d095d2a752508cca93820175f328d603a35a7e |
| eagle__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0324fbdb2fe3d43efed059ebe84c0ecd56328452f46e94bb60d42377ab4a2074 |
| adon__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 60cf1d9e7b7b9b248fe4954c72a8a957b761784a794c7c70f16e95c5bac44194 |
| sagat__sf6__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4218b0055c4228bedd2b662ed4904811c8700c2e59dbf68d19d7864ea5d9153b |
| ryu__sf6__new__old__1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f1e7ee52bce2ccf2d28d5fd290ad7b28f95b51454d3ec156029d508c6ac0be90 |
| ryu__sf6__old__new__1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 653291c50f0003a036553517056ffc322b8522c83dd393baf3492d75a940f314 |
| ryu__sf6__new__old__2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 46462994dcfaa1f7aed0be4f7ef218f6e9535def7e401d180856df40585c142e |
| ryu__sf6__old__new__2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 33856b774a0cd7c0ef0197acaa4926c9850bfca8e7ee0e4b425f007be5f2b9be |
| ryu__sf6__new__old__3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a48feab4df019b2d7f14704395c0bb880ccfe53c2d5f1d9a5d6f74fd6944f3bc |
| ryu__sf6__old__new__3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dab34b2321f54dfbbf60352723f89c88fc4676f87ccc96c473e341dfe5ba27bc |
| ryu__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1cc4392fab667f4d65aa2284938cc56f22cd75791325cce979ae2f5adf64e3c6 |
| ryu__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e9f86413d5b133d5a0275a3ec194920b6b5d1ddcf74bc51b405e42794e381373 |
| ryu__sf6__new__new__2-1.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 025cfaeba69f8df94cb96e4b501582b11f5dd646858164f6273108e57cef79e2 |
| ryu__sf6__new__new__1-2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 15507cc38aab49168a5db49f9a69298844230e19e096016ae60a5aceeeeb89c8 |
| ryu__sf6__new__new__3-2.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c8393088fe80577c019de1623a57c0ca9d73a73a544efb4408fd294fd40fa50 |
| ryu__sf6__new__new__2-3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3799d3d3ff2d958344599d125c2624e5bf67f18554c3a8f88b646f771205d69a |
| ryu__sf6__new__new__4-3.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3d538593fabefecfc61747f9115444521ead62ff96cf8b5442f966780416bfb1 |
| ryu__sf6__new__new__3-4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6cd0d1768cd8b9ac91ca0b19b1a9613faf808ba7ea3a4f8f839d39a532619514 |
| ryu__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 25f4d4d053f3b61f77b600f614c8e2440457603d85b4fae9787a6e0da0a44395 |
| ryu__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2e0a95c0c5e204d047e80fa45f249e845a6aac1ed8d92036ab7dc78ccd617f57 |
| ryu__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 23291714924f60dfa0c1b104ccadc0f33b80e8a462c48a236fb8d7a39d5ef2bb |
| ryu__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f0af39496512791321b04d5cad33685ff22af49fb140a21698ee53d71707344d |
| ryu__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 775022e22480fd5def63bcd435cdf8a0f375bdc93ab3fe34c0877e0d5989879a |
| ryu__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7aed515211f7e1123e0531eab17a363b85196c347e2a3364843805d2420cc2cb |
| ryu__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6eb306047e729e0c898dec963bbb34b24f40f6d559779544554f550829427b35 |
| ryu__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4fe678e31f70a39ca4fc59adb991803992fa8bacab128b1290614a481235533 |
| ryu__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 949d1905918dbd55fafc28cbc5ce000791411433d6e9769cca2fdeb7654ba8ed |
| ryu__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b4165a65ecf5a6fd7635751c5f4c9f1d0f032d03518dc42107b72aeb993f5e9 |
| ryu__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fb928b38d5b535732df6f7f1d3cc920c18c4b4a05622596574f4baf91085caec |
| ryu__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 746cc6dd8927b8dc1800e9946a98083afd12b0f15f09911c1bb5d5e1091ea02e |
| ryu__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3bdb568c5503ec2dea7da4c724ec56c73700f18a2ff7d63d27353060fd4a90bb |
| ryu__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 52be4a1e5b2d1ffd1956d4ac54c657e1caa9beacdee9361e6b25886c47e6486e |
| ken__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 76516cc93f822835bdbde271a21f07ba76f30902775677602dad84aa3fbd0a47 |
| ken__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b4c83bee864f0a4bfb45b7ee50d180965961a9fb45d4f59184bb0047b088793d |
| ken__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ecc998d236bd8901f2ffa3d6fa19c812741e2fa3f7ee888639f723623ed4fc81 |
| ken__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6f4e4524d1169730619a327b95caf16f5eaa31a33704d55037b31809d7c4aa1a |
| ken__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0791d2700ccbbe9a8888407ee051eaafde344ebe75b94be1a49fe011f895998d |
| ken__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b837851d7ab7a920d6ba1d41e1348d48974eff3784bb660cecb9dd7f008f8a8c |
| ken__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6ba65240feab674d04a8d5940df4c9850c0acb9823751fbe233034b414bc39aa |
| ken__sf5ce-skill1trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | eff681ac9ffff502ee5ad0e271fd2260df6ceb1f594680d1c4315e4d4b70baa0 |
| ken__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9b73a3a3616ae0359a863ef3da55ddc2358266b7cebebb383ae4e84c7b9f9624 |
| ken__sf5ce-skill1trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4f21ae1f111a7d5a930cb7dfe4150114d31734e9986109cddd75297d5e25a355 |
| ken__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 76fec1c140dfd222fdf2b39788d25c51aca626b8f00cc3e703fecc8b30346d74 |
| ken__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a80ffe6e9efb510047426665f3a55420e4b139228bd975c25c2913e6266e09b4 |
| ken__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9084a2839e375d1a0f947d1a2213ecdbffdc80eb353fd63389353b33bfc56a3d |
| ken__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 975f8771698e6ca6486dc8b6e000b6d1260c361b3bdaeb710245a79494c6afbe |
| retsu__sf5ce-skill1trigger1__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 51fc25ad38d0f656dc4ef90e61ac0e3e78d1cefa1c26744c36f9e43ca368a71c |
| retsu__sf5ce-skill1trigger1__old__new__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 3f2aa90663482b3be9770a1d2618ec3626af80ea82726298c179f822415f9418 |
| retsu__sf5ce-skill1trigger2__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | d3ae6db2936b62629b226c755a3f9f2e3ac17dd90b173b4928854e391551dc7a |
| retsu__sf5ce-skill1trigger2__old__new__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 66d098b6aa839c9650bda64044bc71bc8a8757a2805067cb53253e84b8694c72 |
| retsu__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b90bc3df7da60c703efc6039ec8d8cb8e9b8044a44b5979fbe07feb32e65c8c5 |
| retsu__sf5ce-skill2trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | fa9c41d040429270d0a4fa3ff2aac3abb6567a799ee84ba8a73848562e0ca0e5 |
| retsu__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3f678b11a6de44fef78cf20e16357e295d7c8140aafd77800e42d8570a482c52 |
| retsu__sf5ce-skill2trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 1ca481ef091728cfcc96ec92ed42cf324c5c6d2e40d2482184048d120b424c91 |
| geki__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 86f59e2434d7b6fa190d1d312c6b2bee2c52ecf9838f8c7bda9172322ad5db8e |
| geki__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b090468bdbcd04d837e2ac240ddcdda0409ad00a65500b55265b317dbd26c0bd |
| geki__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07e86923ed1f22da63a1ab18d7d8dc8480486158ac5d7bcabe11792050520402 |
| geki__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0788b3653f06ddee2e22381659e9e1bba535a28cfce736c74f3bc2a01e1d3b15 |
| geki__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b94e2208287af0f49fd16725afb52474eeea9403edc016a23c313e821d2309aa |
| geki__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5d93137a31bff64a740690f9860c8197359c5f9ec969bad0bd66fadbf2a1d273 |
| geki__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 44040e210033ec97eee2b831e340fa8c511740e882457cdbeb86274b1dd7fed5 |
| geki__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ed120e7de419028fb3a54ca88fcf4eff749d04571e40e215123e1009d3c77c14 |
| joe__sf5ce-skill1trigger1__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 71557d583324d010c088f248fae258c8ec2eb6074c21565f540dfadc0641f2b9 |
| joe__sf5ce-skill1trigger1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | b60de7413a4068eadc218930e257987fa7e6676afcf2f5c8054813b5ecde60b6 |
| joe__sf5ce-skill1trigger2__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | af623f68a64d6b7b0d0bdfbb1042b023ae7110a63b368366212ce958a40a84f2 |
| joe__sf5ce-skill1trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | bbf12c0f692a547fc46d0f48c8411c594ba4e9efaa32bdb326cbadf13024b1f4 |
| joe__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 322f6089354879b4b5bd6b17cf2a69a57734a6b5cc902f9520549f5aeaf14d98 |
| joe__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2ac2c5f4eff57106e2793946a91b88600e354cd1039c8b76c657e022772f501b |
| joe__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 91bfbdbcfa14f18a858bad41d508a6ff3e85f8015e7caa098e3dbf24ce51dbde |
| joe__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c004ae4df4595dcc778f3f9023f6b4df88ff66d5221fe8f87673019650cb5d11 |
| mike__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9ea83e71626dd022e416b7dd4f8fd42f4585919db64b9ddc6967be2c2edcf5eb |
| mike__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bd1e88e50c247e468a96723fcf37b6dd98af65e9e168cf535bea883c23cfee40 |
| mike__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 62ce70e1b68d3c72bd95c258d5ab5a9368c721cbfb4c551a20e7dba505fa3d42 |
| mike__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abef35a7d778e38e6b1329f0945818bf326ce1412d58f73b4d18d02dd3513b13 |
| mike__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2062e1c37515ff0047e3dbca64c43a5f13160e51c7a293e4eb80598e944f449f |
| mike__sf5ce-skill2trigger1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 353ac079f4a920e8caf73bb738688d6c896cff8396035548618be8519d866a30 |
| mike__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8b0eb87a3e46d665bc8933c7d1d81e5cf061932c324084c9afce33ac10d09e29 |
| mike__sf5ce-skill2trigger2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | bbe027f89cde8da96d9659c1717f6994fec466a25617b710df5f436a02f43b11 |
| lee__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e5363df0b1d3ddc909e2da4d01089e94df00590c65224156d3d0474835a4a1f3 |
| lee__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d830fee27d5d63bde2bb889872957dd07c5cacbba1ba1a228070c75824898174 |
| lee__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fc5ef8e7b8169b9ff93920bbd4c2bb7ce6671b3fb6b014d4ab4f66bad177d945 |
| lee__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4b532b472991ef4cfc28e60e5bda544039696f6b49f4b4a3b0774b14716aa957 |
| lee__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1e88fd486f686d0c443937ae918935d09815c50f2ceed5c28ec460b9f899b5cb |
| lee__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cfaa17febfd7cebe7b37fda6afd8c5fe86dc3e73925ddeb4096df427fd82297a |
| lee__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0ff6bccfccbb16b6ed5ced023b0c0a1209e40805efde92be88f1ded83a8bad71 |
| lee__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e8f7c21d0d40ab746a7bfc2fcc3306765a8c47434c2eef5461104b97b5202c8c |
| gen__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b266b12a67fc9e6a579c3f9d3e8e2841749999833e0f0c611f6976bd655bf603 |
| gen__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5164790a238e935cbd35085daa9908c63254abbdcd76a6c1118c3180a43c6d84 |
| gen__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dd90d066caed7207539cbeac4dc3060f4786406cf4587bfee2fa702249127e4d |
| gen__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4f6b934bad412e55ff77dcfef2bd138764d3661457444452c732e79f82d18bf7 |
| gen__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 974411ebbc85b043403bf9a780f29a687afb2eb372bfd0b20187b2878f488590 |
| gen__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bb99474cd85c87bdc81282a6da868e42a56f69fb6e3fe2c1b15bc9749aa2c732 |
| gen__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 78bd9ce4f65ea1ab1dd4bcc9272d444fdd1b5cad94bf74bec352d835a98e254d |
| gen__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7a8a9c0c35b2e850fa985b9b0fd1d8e15057de4f0f57f1dfbc3f7ff7168322f0 |
| birdie__sf5-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3a26a21b127694fda7d22109ec0aaf463b80dab93cc2e1057fe6ce9acf09e3a2 |
| birdie__sf5-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | db25566a38c448edcee18dbde39c7ab1ddb43b61537d709b69e19e0bcd868634 |
| birdie__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4327160a3a7dbadc69b5d4a6615a47f190cc8057c3f802e7e7e813e65b0d70e9 |
| birdie__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a6130c14775eee9f29266224e4caf958cb90383141e85c2ebdb0cc41b68a41e6 |
| birdie__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ae71a9d22882774b7cab92f48d27a4ba48beeefd5b89ddbd6613bca0f5dd99b2 |
| birdie__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6a425f0f7c2f12a4a8fff98ad5db282b450cb11681fcdfcd926193fd2db640f1 |
| birdie__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a7bb4d6b6b3ddc102730879642112aeec71206287798f223f94e6ffb26373a1f |
| birdie__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5e364434d5f8aa9653b7acb159364d9d950ba6c6d204190200a590eb8b3846c4 |
| birdie__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abaeb87e7e5d0874157d88e69da6430088378261bc34fbb3db39464c4ed68848 |
| birdie__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ff6695eae1f3bf2bf5c63c5616fb7ac5974f9e906af95a781633856c95c5176c |
| birdie__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 71013ceb2d7ae115fa3a83019722ddf31303e60fcfcb3dd0b7110226f1693895 |
| birdie__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2d01900ee830989ae26fb15cc3b3f4a32df2f25a89e794c030826f555eca7855 |
| birdie__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 219088e443292f5ac02bc9fac4055ca8ec36da9a460773f7c3c35f47c33edd98 |
| birdie__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e71712434415ec6b459d711eb0c52bdd8045b20e2a1eb84288e480edb3834cc5 |
| eagle__sf5ce-skill1trigger1__new__old__4.tsv | 33 / 7 / 0 | 33 / 7 / 0 | 3b21ced1c6a1b5e95d9d14de06126073698ec939dc7a730ad139cd4ec85faa83 |
| eagle__sf5ce-skill1trigger1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | e33e0a5c8252175356cb738c8d713959fea4f10943ab3a8eddda471b0e2f8e3f |
| eagle__sf5ce-skill1trigger2__new__old__4.tsv | 33 / 7 / 0 | 33 / 7 / 0 | d8f806cb855f6c904425b6d989e11d466b34b93cae38ef2fde0f8a922396ecd0 |
| eagle__sf5ce-skill1trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | c36a0c2c78238ef6603d711d0e7ca3e161fe6972efc401053068b83ca1abfacf |
| eagle__sf5ce-skill2trigger1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 91e9e596464612846f820e0cb4ccff4984d5b0165d9accb4911a10e034ae6d83 |
| eagle__sf5ce-skill2trigger1__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | 69abbfd317578946282787cbc1743286430a109933171fffa98685be92bb6624 |
| eagle__sf5ce-skill2trigger2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3a761229b947cdd93885e88bd14205cac415ab279dab5d2efe7f513711e49625 |
| eagle__sf5ce-skill2trigger2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 5002f5eb1bc330384524c9d212e76ac18dddb2939978fb8d6e9864764ccdf19d |
| adon__sf5ce-skill1trigger1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 459e8b6405d7db939e33161f5c2e0566e61fef2efbbf83620424200eb1368ad8 |
| adon__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ab0d593e0ccd6e591a6a9ebe9e73c899063c9c6d03ab42dbfbbec6213032a82a |
| adon__sf5ce-skill1trigger2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 897fb63ab769e142ce34490d3b927ec5b928df088fc15a9e4d2bbc3f09bb4093 |
| adon__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 65b022a63f3d8b143555a0864150872c851de256e99151ea06da04ff18e800fd |
| adon__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 99e76f3adba63f9dc1300ae490b1e2531862931af50d6c50882e629564371913 |
| adon__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e4edec6e5eccbc11cccfcbaa18e90a1c1c647e6d731f9666d38f91865d375794 |
| adon__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b274f50911208fc0e55270fd10b767ae135d804ef46c378180c63ee2ac27aaf3 |
| adon__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c5f93bc72c8096bac01b71d95520093a7ec6656f74f826f16b55424e037819cf |
| sagat__sf5ae-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 408aba69d622ab41d415f85c1e8611bba9948d7ece61d1ebb522a0cff2aa0716 |
| sagat__sf5ae-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e4b9ecd98ff1f59ac78dd21b7ce49252dff37707a097fe44bd11c137d84749c0 |
| sagat__sf5ae-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2e12252d32ed200eb0d4056c6fe4887e729a584f69b9c4b10da22da665630e50 |
| sagat__sf5ae-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8d5bc621697e129aaeaf9125353af4b7ac30419e3b35fcced1e416e4e32682c3 |
| sagat__sf5ce-skill1trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 410d3abc5610b5d0658606262138268f84eec32f2f9825fd07b00a644fac7fcc |
| sagat__sf5ce-skill1trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fc27f8aeea44fd26351180896a632d36ec1eeb5e34dd0569b12b30b32d0203b5 |
| sagat__sf5ce-skill1trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1d1c52ec988110e4c44ee805adb2e1145f2705f5429de15eea323cf8c9fa33f2 |
| sagat__sf5ce-skill1trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d3b26722af198a5227925cade526a8670a8b4a193604d96b75e2f66857c15d61 |
| sagat__sf5ce-skill2trigger1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6e1f157f86ac64139b20080953616691fa74a2ac5ea0f97dc6a4ccee300a5d8b |
| sagat__sf5ce-skill2trigger1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 54ecb29a1af2fcb590957103e41dcb7f38281adb073163120db176d7a90487e5 |
| sagat__sf5ce-skill2trigger2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1cff8062ee19c2df9b748b2f49e67ad88d76fa39946aacf6eab3b87f6bbe56b6 |
| sagat__sf5ce-skill2trigger2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ea01eb6aab0f2eb6490cd8af37df1322d89fc1dbeeb3733ee2ed84bafd37da7f |
| ryu__sf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cb169808fb5128dc1c8369362e1262644800ab06b9dbcdb35b7b2a2a294d191d |
| ryu__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 323017c6af20fb274788a9dbdc431f8c510c8bc9a99a1a61be204ea9750e0176 |
| ryu__ssf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a6a3777000ed7980eb04dc2ad8484728a84d11eb443d922b4b549b5a7e82fc81 |
| ryu__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9d03f5aa79bca9222aa4776fede207be19ab04c44c7a57039d7898318a618c10 |
| ryu__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c925fdb2c87f6dab9744da17a9edf6a578dfbb4f3b2a891d8b4707976a23d2f2 |
| ryu__ssf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 18cdc78f1aa2273bae065cf3faef16101b4d2b5d7e1892cfdfe91a9d250c4f7e |
| ryu__ae2012-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c8e7d37237b131b60c1b321b7e2e1c589b5a436f877f4117f35858de303a57fd |
| ryu__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 78a01dfae46b0e069d2eb44fc9cda65074bcc43621aa8278721891b98f25f54d |
| ryu__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | abd36f70d6998cae68595b823ae2859360c7fb89e5cdfe4c1b8d077a2ab8968f |
| ryu__ae2012-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c2a6adffed4ad0df87027e486028722f243c866fb513ac87557b9fc95a092922 |
| ryu__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2c9ec1a1fb798639097dba0b05e913474b9930269da761bb7889fe395728b376 |
| ryu__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b1c7df8ab6f0018c239d795192260d1ba35f5e89ac47b1db3d8bb29374b772c |
| ryu__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a42b2e620a2cf362b0555122ec90b039f1577701d1e54f52783b4fbfda007f24 |
| ryu__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1125ae8a4d12ec6c2f497d6a1fb7b96161bb5e2d54b1528124d7c043f918cae2 |
| ryu__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6361c3dfb6020da315da5c2b6b56175be787e692f07c43a599f03e90f1dd2d72 |
| ryu__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 11511c7b45825a301281b5f2f82a0c9383c2377e9503750ac785244b116b0615 |
| ken__sf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2d686c53fc6d62c3fdefd4f3c784979d2a261135ff407149eec9b74e0344e6aa |
| ken__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7e763a16b83a3f136eba4991fa7c88490bd00944d7d9080ccdd86fcd0ae2ca5b |
| ken__ssf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a140eb84bf66e168cfc3957ecc485431e0bbd100ad0f418ec6b591908aa07d3b |
| ken__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0db3f4f8485f7c41ae8f4be5abe65f7b60ffc0e00cc1b06221e8efc8948a1239 |
| ken__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 35cfc446f27f79d84587ae85e68fbecd8ecc4891df151235eaf3cd8a6d8abc11 |
| ken__ssf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 301992d7234722a1f72e47782403cfdee1dd7b7ab55928324d85da35ff160af4 |
| ken__ae2012-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f1baf3565168d8ca3746b9c910c0cb082f846a297e0036c1e07d2483a7d1f78a |
| ken__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 12cef878a635eeb611368d8238a49e8dbf6a52d413323e5699f35fb558f6db90 |
| ken__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 139e9410d7ee6d64abeece5a27d8d17d021ad8d395d4367843fdc84d66d5c67b |
| ken__ae2012-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | da8f6a3a57ccdb5d3853388b2e9e62133545cc3d6ab389e301c205d0b6cff0db |
| ken__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a0cc4d92725ff969ca1f30fd14807fe07fd85adf7bfb01501d9eadd3927e1581 |
| ken__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e84813e9d056c5ca6271355f915293e8435bec497a7411558f6c359633b1c6ee |
| ken__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f4ef05077a74519bc824d236c538fb253242562566f9ef033313a5d335b2e166 |
| ken__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cc7bd99423cd8edf737d0050a525ee0ababb8094ddc1394c53d6f6e0f5afa0a5 |
| ken__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | daa9073a129f6f0a7c7e6d0b535a8b1a887face3ca382b26e9b913fc071ba0cf |
| ken__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 25cb507bb79de1d7e0447eec4e66cecb3a12412388c0ffeaa8cb17f9d67c199d |
| retsu__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | add0bff7c9f1fa729056f27320bf96881218ac39e71c0bd06bf4cb4e626fbce6 |
| retsu__usf4-ultra1__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | b5e13815d201f9aa8802ddf4dafd8708efe704cf7e97b456c0b50e74f12be75c |
| retsu__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a620316620dca7dd390caedf8fceb0aacb9852c13df64fa841a88089de1801d9 |
| retsu__usf4-ultra2__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 94347d5ba366a7039a776ff906255c5a02a4b5cfa1b650ad53baf878449b0310 |
| retsu__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bf0ecfa1f01b5d0cb4b2adb4fe93b76f13a71b98116e9d9df419914a5b7ad2fe |
| retsu__usf4-ultradouble__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0160dc7a31cd74ca98b5e56df7fad98e621d082cd12dded093788958574a3ff3 |
| geki__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b029b0998e7b7b4a0a2e4f83c19522da5e47bbd2faf8e113d3a36a40c0e93896 |
| geki__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4e38faf24771903e7551c8d9d46eeec1875a5c99847ff944cd00f23e154f78d2 |
| geki__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eaa33cdb9193f09a9b349a10c0b0bb4ab0337f4c7bbc61a38d9d47144ad619ea |
| geki__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 581262c27eb25c7267d70de83cc3adc4713aed65a13d9616bed73a35f96f2173 |
| geki__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7c274459fcf1b0b4cb24c170839b42467059cf26be333cee177110f63b0cdfd9 |
| geki__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6b37fdab4c0dcd54fa8186568679bcc537f8e0a084d5a18a91490b4f5c45f0fd |
| joe__usf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 43edabeaa5b90179f49b1ee5ed78c48b5305c60813f17cb519a710b62ba7ec30 |
| joe__usf4-ultra1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | c7ecac8756880eb9e1571fac0798048c937ff78c6081d7a40f48b4915fb1c828 |
| joe__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3c3666733e70530352fc3f0f6c2f9ec888f6174a5a3be67ad3e3e2a11a07111c |
| joe__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4f22a93f116b0a37204c24abee1eb6c50ae78e04dbbef0ccb22d34738b089e9 |
| joe__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | db55ec5f8402f0d4179ab2eb1b2a534fa1f82ab5a5acf2383094cd36a5e8f854 |
| joe__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8ea5f9711aa3a01773a0e29376e7bf178d1af5b7556807f21a33b385daeda386 |
| mike__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2a1c843ae95e60c992bc489f484f3510014073f3c62f2f76c3bab8fc31689739 |
| mike__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f5f4359c6605412ca37c5be2fb55a2ac813b9d4c6c0d86e93fb86db1284a49d9 |
| mike__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 49f8d975af211d4a49bf1708e10758964537ef80ee096ce5f36f18c2d2ca1cd6 |
| mike__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dc9c87752af1f57ac03a6db1630020b346ae34b6cddbda7a940c9ce2d987fd78 |
| mike__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 56fda10132c3ea85acea0e7707cde660b443969aecaf96488a9ab2eea8fbe198 |
| mike__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1035320779692a58fd074a731ce1d98a100badfef60cc477c56f4ec769b5cee7 |
| lee__usf4-ultra1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0b401de4ec9e836524257cd87b496b5cc433444694781d1c528ef3e8818b26bf |
| lee__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ef85b52e323a39bc4a7930d29e22d9cbc6f4c7ad36002a115884ac9dd6010dd7 |
| lee__usf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 266b0cbe8fd786c2eb83c49d762b7f1e67d787cdc8899fe3e380614758410d5f |
| lee__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1c742662e63deee21b6869a8316ad54bc86c993114cbe7f4254878ad021eb645 |
| lee__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d346023a815cd2a2405897509c17f2fb6ac0caa5240b8614c1a6dfd3f05938f2 |
| lee__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b3ef5299e4d4fdb0fe1b278363d70e14debce388266480a93e3bc6de459af2d4 |
| gen__sf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | ee166e6a56d753a9e589cb487bcf9ea6528301ec3c2612e29d9d1157b5db19cb |
| gen__sf4-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0e8a010fed49c17ff044d97f9cbfd6ad0ee9d08a610b5083bd276d8272f3599d |
| gen__ssf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 24f0fcb27bd46747cf88af40f293170b4cae564ee26dfab18bd0794c403d8c0a |
| gen__ssf4-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 095c6afe338bfb6a184acedc3b45f18b61f82e8472fa7811ffb4406f49521560 |
| gen__ssf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 7adf2c5b6c2ee45c5cd778441438a8ce2e047409c31cd06b4b1f016079443163 |
| gen__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 5ca6ffaf383d5aff6cb7239e0eef3517e21363b77e18efd281c2122b485cff57 |
| gen__ae2012-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | b67dae63bfb12091370804b0ec81c577e8395d4e2eb41203082dcb041de3b308 |
| gen__ae2012-ultra1__old__new__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 4ee449bb83602fef55c76c04d275ddb0248175029eb447f29051f435438b4658 |
| gen__ae2012-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4e07fbe835f7ea9b56284a00108bf7ae16231ce8920645fc69c40ed6f92fcfd7 |
| gen__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 5279d6f726d46edf0d63cd987f8f1539a035249999f333bff0493146685f5465 |
| gen__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 6319823a00bd7b5ee454e2017175ab21de0927423c42ec326eab57847bf3a311 |
| gen__usf4-ultra1__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 787850f8d7be44e335a715a6a1d5ddcfb796cf0f6d42c0e7e1f754dbfd05c9ea |
| gen__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | d4a8e1d3983f33ba55cdc52ee36ce0015f57a304f247cfbcdffa7fcbcd898815 |
| gen__usf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | e7e9ae8e18eb167cab1c492ce6d82cf6875d4f8778ee06cdd7d2b016835b2bc0 |
| gen__usf4-ultradouble__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3023ffb718787a95ecfcd4fe48afabdbadc6dc4119b491b730bb9a27e366e82c |
| gen__usf4-ultradouble__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 8800a2398ac29cf496a9320e22c26b3c24b3fd0698d86b92d7c0b684ab2c92b1 |
| birdie__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 075ad18434ef1c557b942ad162ca53db9dbe05bb21d64c5581d51e3a0d0c4d5d |
| birdie__usf4-ultra1__old__new__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 01601a668b2a152b3487a7f726996dedcd7dbfd239345153cec491319cbadd93 |
| birdie__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 98dbfcfeda30efde0f2c09c0d00e86ea816502b5a2db8ddc20ee1a4c811e2660 |
| birdie__usf4-ultra2__old__new__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 2a8a772fc45d78c2961c2737603290276e3dd5f010a092076ad17faf37e9d426 |
| birdie__usf4-ultradouble__new__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 0eb0899ef65ac3e5217429212dd32c779fe9467d8a6d1127fe709c42c071f7dd |
| birdie__usf4-ultradouble__old__new__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | 2a78539648841f3fb9247fcc841fc2ce12811ce30de7c37d61e438e2dd2bf78c |
| eagle__usf4-ultra1__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 2f9634bf39d3b15044ae6fcf25bfe95e238ab1eac4da4fc98162dad8bd25e693 |
| eagle__usf4-ultra1__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 96fb0861f44311ff9100d2c8af939d42f66437ed4ac0851c43f8d0375f0fd61c |
| eagle__usf4-ultra2__new__old__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | be6c120b168cbf28e4b153520f9bda39a45c82cf5247edd55259d15f051e8e0d |
| eagle__usf4-ultra2__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 05849545c976a5c6c7ea7557c383499bc200334109c5632445a3ba72115c71f2 |
| eagle__usf4-ultradouble__new__old__4.tsv | 32 / 8 / 0 | 32 / 8 / 0 | ece890e49e3f861194b5d72373d40e7c55a673f4e15a5d7db3596e58201295d3 |
| eagle__usf4-ultradouble__old__new__4.tsv | 35 / 5 / 0 | 35 / 5 / 0 | 1a9311e4f6a1706a67c9f570cf360e76d5c210aecfe758ccedcb426c933fe0c2 |
| adon__ssf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | f1e53401ef9faacc6b5d3e8b14bf7c7d6fd0afe2968bad157caafd49c19ee3c4 |
| adon__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e247e47eeefe77f1a79be45fe56dc6f051dc6eb798ec9eacdc3c7e11cd2b7084 |
| adon__ssf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3fa2044e8284ab97c3f739c139be43531b357b09c1096007921ae37943a49343 |
| adon__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 379dbfff71e8ae0fafcc36115951e80f75e00ec130ddafc38fc3ab8d74fcc18b |
| adon__ae2012-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 0178b7e09b9f200826f341cd670701155bf1ae29308d661def17efadd49313c7 |
| adon__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f61d983972f8cc9705b354d4fde54ae1e72363c9f6df6c774fdfe70b4a308eb2 |
| adon__ae2012-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | ee9aaf4b50f85369264dcff6325c2c465ecabea920a8d9bcb051928cf2646d89 |
| adon__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 2c2dd9b475ad322e670503609115e7121f291c158a00bc88f15f046c12ea1ece |
| adon__usf4-ultra1__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | e02c8fd65058d3dd920f6f075d2606010a1737131b45b71868e9b5d40a666bc1 |
| adon__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ac457f3dc3946471b5904e10ee03d8a9be5371a8e562af31e14c31494cc088f |
| adon__usf4-ultra2__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | d8aedf683950d09e8f79429d6a39707569390180dad8379b506ae462910078a6 |
| adon__usf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 65f056527a0fa0b865a2876efefffb0ae89c4e5319b1d1b5bb317a67928cde08 |
| adon__usf4-ultradouble__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ed181f883b36f2eea1dd173f96c54e07c0f2874160f3f154798f19cf9d536b60 |
| adon__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e82fa5164b8cfeace4608d2cb854e0aee6106cb4b9b998d3445f143f4f0f5715 |
| sagat__sf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 26e5b9893c375dda0d623f68faa6a12c5c956a91e34f3103f30987c83b742b73 |
| sagat__sf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 476174ede566b6c3271b4698d1812ac42d59f9be87999d371fe19bb4484d9242 |
| sagat__ssf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 790f80b0bcf36f6831c451ecb0e53719df62f1026b15cfaddc9fd504d1af8549 |
| sagat__ssf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 224515032bf99a987af210bccf6be4f60781369e0493bd7d6083b09d3f26ba29 |
| sagat__ssf4-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f45e86bcab611752b9cde3a7b4153ccc5454792b7903dedf33a018b92e762279 |
| sagat__ssf4-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | bda53dce90a079686a85fea840b3276b3f9d87f4640003292329aece2a14caad |
| sagat__ae2012-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3c8cec5712a11f83ff057bf8f5fc615d9b24ddfc9301e4ad67ba54ca6a341ec1 |
| sagat__ae2012-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b144012aeea4e6cb90c5fcc6b0c9435944aed22ef7a5f456e91a3a02d02f8b7c |
| sagat__ae2012-ultra2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 721c2c7760178104e9d080b7e32f5703bf3f536317ef29140ace3da917ba692e |
| sagat__ae2012-ultra2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 4167ade9ea0852ca5d79ce5547c94919c3c2ca38709280cdd24ad71f932e2125 |
| sagat__usf4-ultra1__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | fd65378d1285f8794bfa9b501eaffcd4c7583cb057765d0bb10938bf125d2355 |
| sagat__usf4-ultra1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 38456a5913488f1645278fe0117ad049275a77dc2bf33423e74dbc9c7cc1b727 |
| sagat__usf4-ultra2__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 94c3ee1bc551d0e920fdc7fc8df21eeddd34c3fa50f062d7be7b96e3f2d35fcb |
| sagat__usf4-ultra2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ab3251eb7a2d854637ed643ab8d1bfa10a2b72a9c6c6902fafbbb0233d82973 |
| sagat__usf4-ultradouble__new__old__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | dcf1207539dfda7bce817b415e3a4d66993f54584dd623c7bb2017f67538a6f2 |
| sagat__usf4-ultradouble__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 6c2d4828900a57dd1892221f66f0ce3408fd1ce0865baacf197090d50e0b63e5 |
| ryu__sf3ng-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d5f3932cb7c7c9a30da5955ce82773502fc81a425995add65853c4774e07ebdc |
| ryu__sf3ng-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8ce5cd5cc78dad239b5440363b17e780a95e79a1f8568c3eff0c0bb9c4aefef7 |
| ryu__sf3ng-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 518680aacc639ca61841ccd8d1712d728faa8d91dfdc6f601a1825cc0511dd51 |
| ryu__sf3ng-art2__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | f95c4b43433479c1330e34848aa378e4d561713d432532dc8e335b48aa284de0 |
| ryu__sf3ng-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0cf493df3de46847903a0ebf9da6c61446de8f667ae286ec615a9360bda09f58 |
| ryu__sf3ng-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8669876ed72eb04c877f58ead9052a8384fac71732f1c4d16f4c9a0f3f6fec46 |
| ryu__sf3twoi-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b3de03071cb28cf878743461bda0209f6193c4aa6664dd759aa68a8e8ddbeb90 |
| ryu__sf3twoi-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 49bc7e6867b69a08ee418de8a4677c4c2b54d45e497b9a68ce3689afa47ed935 |
| ryu__sf3twoi-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7fce0912b939155d7105e5fb8339031173512cb54b4c21d75e1c7d0a9e80f267 |
| ryu__sf3twoi-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8fddf1e9def4387175a3aed5717301e31cfaaf44fb91cad0f21acca4df48b070 |
| ryu__sf3twoi-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b85ab2bb51858e53c906c3b6f10f16a05f7dd36b7d0615c85ee6caba472757a7 |
| ryu__sf3twoi-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b6720043ab38035f5180a66963dd931e4f4feb394a5224ed6d92fd49c5846134 |
| ryu__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 83bdafa9ac7c57a3669c99907251d4aa7017fc9e2e70c099b9454a0a6ec50763 |
| ryu__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | be20c6b5d0c80eab61309b19f4d2519a1a6c47b234999f6d44c1e6f0221e4d33 |
| ryu__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d9e7665dffe8e391f9ed1c4d153f573a9294cd6f6a083aff3e7a7885499a5517 |
| ryu__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 28c4195da45d81f6a32ed9d36bec1fc2fc617917ade760a5702610d055d82fad |
| ryu__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dfa58cdf639efa0be6cecac5ad44f6a4b2e3ea303db241a15f29418b56586591 |
| ryu__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 43d4a158206d5e1774ae62479bf3f155a7224328d707238f3737522f0c3d23fc |
| ken__sf3ng-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ad4700378e5c809bf6c8bac4adb328510e4ce9000182660ec24a1c76374bbbc |
| ken__sf3ng-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 66480561f5b1272abcdb8243f1128377e3661ae6acc2df5893a5be1a8d99daa0 |
| ken__sf3ng-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dcc6e65dec024b5f45c02c226d051082a73de8d5a91d3caca05409a38486b26e |
| ken__sf3ng-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eac40e65fb345a9a3f16879a641571dbe9798cfcec976080af1dbda7866c5bf2 |
| ken__sf3ng-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c1088f8971e837f706d168ecdd9f3d87c84aafbd4b25d4aad9e8627dc151af78 |
| ken__sf3ng-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 97fd45c88e06b7264054371d64c4c51cac7a7a855e8061a05d9c605d8e95b8a1 |
| ken__sf3twoi-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c0696e078aa535f397f9cf57ae1208341aec050a761682aaeaf002b81269036 |
| ken__sf3twoi-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4acb54bbc0556879a05a45a130644e75638bce52793cf7179777fef5dc338c71 |
| ken__sf3twoi-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5003481c2c98bc3ac390be26944c85590d9296adc711deef1664e21b0d321264 |
| ken__sf3twoi-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7411156432b9e23453ae391988a6cfacb25d67b5aadadcf3048dfd8c26326e0f |
| ken__sf3twoi-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ca95507096d6cf9440d70c11126e33829bcc37516258dbe8bec2397f90b8c701 |
| ken__sf3twoi-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a1942cdeeab0459f2d3bcebd6f6818058b5a95c084246e6c7e41a4d7363d90e8 |
| ken__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7b7ae4b49cd20af460c98e11f52216c661ff4f26ca98b3ccb076eb501fe7a5ae |
| ken__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1510730cab986105a32ea9cd419580073d76dce73245e6a65205648e946ed0af |
| ken__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 008b98d436c24f0370734eaf13bde9f97232ab9fd4f923ab7e46d75400143840 |
| ken__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 746b40526bbd8c622c63df74f319797dda6aa2e7c66e1ef904d3a71b2b8fb551 |
| ken__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8bd46df601a19a690910c7389a7b6b51b2653d4d99f8f6a1e99f5d54f0f800cd |
| ken__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eb9a4d593df817d7c19051d3117fa616dd206ba955e282fdc3762e5d215b9cf8 |
| retsu__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d2e412d1734b87ef44bdd4fa5b3a71428ec579c89a209d3b1a0b4498f86c58fc |
| retsu__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 97adc3e56a33caaa43fab4bf0169ee4ec7354f8afea94c622107f812beb9c4d5 |
| retsu__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1fced46fd9f7fd3337375989a63ceb223e9f05eb7fec19e09f7bdc099eb87dcd |
| retsu__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 70ad50178c5c1be7e5e08aba4087438fee997cf6d4cbd736d083b0ec2f0a4e29 |
| retsu__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d746d47702ae777ecbf99b4994d56e87986c9806528a6258cfd40bc154e468a0 |
| retsu__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 17a61077531ecaa202821b4209c009f6cf1f7f0bf122213d1a4299f9e081e7c7 |
| geki__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8076f41bf62618e60af369989c38ebe2878d3362350e85a4267317d94a519651 |
| geki__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bc83d2febbbcf2264616408deecdc96e72caec557872fae0f469a13180a0b825 |
| geki__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 59c4ed48802655add9d8adc82b5dc36a5d2c6f1a0d0b5adcabff67c99cb51e62 |
| geki__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0aa9c406f7ba674c39a644be58f48d245e0198db47e3064797df519cd2ea3472 |
| geki__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dfaa1fe4823c42ffbbffef60f087c13658237b1c7258291fd72ebad763be33d9 |
| geki__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2fa6d1cadd59931ba9b9b8b52de38ef922785785973e71ab033a0e5e6988d260 |
| joe__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07734b8a96cc45f0c8ec92834d7b5578c95dca315b78f02914c1e18a0c23b40a |
| joe__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 57b5de6ae315bb044d0f465762af62c4120da7159c07d71c5ef76621fac52b45 |
| joe__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 45754a32be1f55bd4927e93383e2bb4d27c7d462b6dd1a0d4131fd2fa8f29a2b |
| joe__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | a4a9c520208bc47f33514313c641bf0fbceffec2dcf650fed7a456cbc0498423 |
| joe__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 79cfe8248ed7f93c380c854e69ccbf0390d3c7f31d95a7dedb49186a0bdb8c34 |
| joe__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2362e4a60fe826c103c165119311b5167bd4be4a3e4f82427e5cc90f4c816a82 |
| mike__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0c6fa6c1e4aa327b6360e7298b039f991366ccbf123583c7a4ce7241c8d8165a |
| mike__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7cf1fada0405cbf7edec8930c4a5f87f28fa80f79e98cdb7093e097cfdccc25c |
| mike__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | ce0f51c7fa359a662fadfe2f043f00bfd27f3f913f045841e0affb0fb519ff0d |
| mike__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3e7a841cf2eaf12a561fb94be115e9ef92d7de13ed390a7128cdec0213bac905 |
| mike__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b609b7035480a669879025c1215bf1c82331f4f13c5b63cb5df7b89e3dd620e5 |
| mike__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b0fd96a269d585c7c30cb214d6cc7d87c8e04ebc4074bfeac05b9c91a4536a57 |
| lee__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 08e90d3828792fe94fdb6de260424ebd96609c8fb5fbcdb2f7e00187dc77232f |
| lee__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 38abc68fd11366855f3b13d9c35abb8ce85a9d79e2d187f12cd329222a96c64a |
| lee__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1f936cd939c77dd83f6d80ea30c3d9e38fb9294bcf23b0b0d87339265ea4080c |
| lee__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d6f57684713801339cc97b6fbc84337e593973f322d7f2a0786ae5eb3e5d3227 |
| lee__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 00e6dd52be6c92bea85bdf7e327ad87f7606bccb306b267e310cd0965bbb9680 |
| lee__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 46db6cb1ee5d87a2e201486542e85b98e6ee2bc9a76d3953e43d9a7ed3809d38 |
| gen__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fee6ddeb30f9ac6cedfa84a6c8f095ccc7c669a4206c18a2db7bb52f351c0a36 |
| gen__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 40f788d6cb4eda418b0264802d8edcc0e84f73e110769cc96cf7ccb3e71e9928 |
| gen__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 54974efd86f23e9a62f8c645a566ee48fda6de2f5430284f106e7cc7b1f88e9e |
| gen__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f549c3d241263dd27f0bb9f9573c6a850e3e03378ab225d550f8d1ed512be2a9 |
| gen__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c80faf97b87dd6e21889010cc46e13122bdb2063e59458c2d4413b009d64d694 |
| gen__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | bb51cc1f9ba52b995e14bcd486be0106c8982c61ea2151dd4d55c7eb4bca9495 |
| birdie__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0d8b257ecf5210bb2e58fa6469b78d563916d13ade9e404edd7b7c9b81835798 |
| birdie__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b778815574715789a1ca709029b581d055ae0cf2397bb49c3fe9ffa35b23bdc8 |
| birdie__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 27fe57dacb0a2ffb9b6871386bf2fbaffcd04270342a4d433faf420117839d38 |
| birdie__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 71a36de3c0d647270f4816ec2e39a00f5dc208917d5ff20e0f2c5f123eb3e601 |
| birdie__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4b2c11442bd32d10af408e6865e7ed2414a9fc2738e1362268da66a5399336e0 |
| birdie__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 130ac89ae3d5c9a86e93c9265d350092ca1d337dd04e0713a90570bcbe3b4114 |
| eagle__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1f816787e2d2a59989f703a016d607f5b9aff2095da79722b4fbfe5e75f795ab |
| eagle__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3aaa797e6fbf1863b7246046537bf345923a987bc32f24dd03ad43e96f3524b1 |
| eagle__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dbee23ffc08f7dc0d0e250397ca193811dbcabe0ba178fc260ef9bc0dee31549 |
| eagle__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b5f3c59bbc7e831c6ab33f18d532345f3aeaef72fb25d902069d743b58d40501 |
| eagle__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 817f35e5db97895ca62b27a26806bd5dc3f187c37377520f9390336653d10a92 |
| eagle__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9eee176ae92ae5d6c4b18abf8b5c6ffedf5e2166b7d7ce79500b107ebfea9163 |
| adon__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7dcb5ee84452bb882f3ceca5dc7e79b570096d39fa071bcc566f60718a04d639 |
| adon__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3b8b4e70e2f8f2fa40b23c0ed654f93c8d404d355de2ed06e371c465fd469f8c |
| adon__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 759ebd89dd62e1b76a1138e419318f187933ca2f09ac36d8cb3618556ca80480 |
| adon__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cd8ed71d57534ed8c242ac8d1d94f2efb5cf0e150bb9bc376bf23c917d15d67f |
| adon__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f759de39a4f360c1296e5f4a83b31c582012979cbf34027cb818ddd877c5ad9b |
| adon__sf3threes-art3__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 83802d1704f3af91499a14977de3264dfe843b521f9b0870ece877f87fa4f53a |
| sagat__sf3threes-art1__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c29715e036995f975f35f8145ad22d43a048dd709df3480ded6af1b5f2682776 |
| sagat__sf3threes-art1__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 354ae1ba595b033fffb331843989af48fecac6c40996f4252133b024d9fb4953 |
| sagat__sf3threes-art2__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1e8a49aa235863b96d3ef93576c047cd3740b04007c35f02e1aaa6397d16d300 |
| sagat__sf3threes-art2__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 45a77de7c84c0e636da4aa395de852dbf33b473d7771581dc5e573408af5ba88 |
| sagat__sf3threes-art3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1643242bdb38c833bd42f1a2f57c3637b82d5388ab35080783d9d61edbb2fc0e |
| sagat__sf3threes-art3__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 776d6e94fdcae6aeec7a0bd5a8796f165c21c8dbb77e8d281578d64a2cb52eb5 |
| ryu__st__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | a625a94079645f57dad42459a94a57f98262a3b941cdf1f372d0f1c7d4b9e20c |
| ryu__st__old__old__4.tsv | 34 / 6 / 0 | 34 / 6 / 0 | 3a956e391e4363ed225a15eae4e648325d9501062a7546870f4afe2af5c22551 |
| ryu__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1192e0aba98cf9146270df68e9ad776ff4439db61967c13fc8b7ab0cd8b96ca0 |
| ryu__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 07e231a1963fd176709086cebd0fb3d96f5041822cdfe01711cddeee4841a720 |
| retsu__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 988a1bb85bb09157dbcbd8f7a756a7bf30be651eed764da550cb36f8d23770cf |
| retsu__st__old__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 46b487d35f262098380d72ed63b5fa61fc20151a817704bf0e2393bfdd57c964 |
| retsu__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3c6248a3f53ac0a66ce8ebbfc0368e1099b209190636407459ca0bf53ac7d6ff |
| retsu__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 51d76dcd7a557d7d3e1b877c375b2e7c65fc8da8103832b85906928e28a55822 |
| geki__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2896fc2176fef3fed1f955cf737e288521aae4c991390afdb4d7d7bdb778cc67 |
| geki__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | fe5628f3c5af5b7188ac8876dc8e5603a0a22e745e442d13ddbb2e840f476312 |
| geki__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 50f61c1f874b7f5da2f7dd2a6c40fd820bf5cb76500525c328c3bf5c5eb925be |
| geki__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b80a3de3c3d1f2c762822359f3c5f3078076e96f5a268f1e56e46c5a2d633d49 |
| lee__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b8ea6101fee17e9c3e95aba2158ab39aefbcd6997bac6cbec27b86bdb50e50c3 |
| lee__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8c094847c9953cb779d0d7b929d3d956b2a13b883d4a832c58ebd3925c82cd75 |
| lee__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9743366a0491211c9be028a273f4c4619d30611df1952d5eac3bb9f5e6e95de3 |
| lee__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 55ec1dcfcc6e33cb6352215755db39ec57656c24b90048899d49ed0cf269117e |
| gen__st__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | de265b76b8ff9f7ba832879d10aec3301084f8d30a177ed50429b96383840e6a |
| gen__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | eec3d750f19be95afbcc427e13c7aa0aadbdf0aac00ab57dd2de1a7de5145c45 |
| gen__sfz3__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 57de16e5f38986fad839584a6b075c221e8f731a94467089b1add2276351bc3f |
| gen__sfz3__old__old__4.tsv | 37 / 3 / 0 | 37 / 3 / 0 | ddcf2413ed8c13308c8741e6b5d882d4941afddea518ed966c6c73a19a703495 |
| birdie__st__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 24a6697aec8034aa41ba6f03dbb6c6ecdbe7388a45a7005df003c2fbe10af4b8 |
| birdie__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1316e6a47f5bc2e15b1f7b635f24b1152a6fe70fb75a16a43712680573a8e0f2 |
| birdie__sfz3__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | feb8ec44344f7f42785783e12baa1ac6d41bef6e215e6dfbcc90f36b0519aacc |
| birdie__sfz3__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c6933e30e78298b96e07136655e96922982776ec0ccdfdb838b8ce13e1ed7e33 |
| eagle__st__new__old__4.tsv | 38 / 2 / 0 | 38 / 2 / 0 | 39a3a7b7fdc0ba091d25e923fb192aa79abbd5950743e075d426068964b04814 |
| eagle__st__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4215b82020012f4866245b91194f208be4b74d80ef7cf007d424140422ed29d5 |
| eagle__sfz3__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 3413d7166021d08c2f381cd6a7eded72c9b8b191af7b6e0f3af87f4302989e99 |
| eagle__sfz3__old__old__4.tsv | 36 / 4 / 0 | 36 / 4 / 0 | 079c4b58cbb1fcd257a3aeeea4c38035d487260040504706ae6df37e9ef3aec9 |
| ken__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dc75fb0d16ddaf0e3189846498fa7f1884b6f0d2703b85f7ff10a8f31f43c44a |
| ken__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7826d009c951b75e500a103cacd6d2fd6a5134e86ce4eb83ebe66cf979d0bb53 |
| retsu__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 910159afd0799fff4ac347543c26d3868adb4d505825dbe607a017b5f2334b98 |
| retsu__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d9b954013bb03faf2f4edef2dd2551a0b71e39a9abde799bd2e76ef9f4c8f705 |
| geki__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 159cca83d42fa2f8ad6d484517ab4cfccac2977d8499a8859f535af76395fb44 |
| geki__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 79579d16b163943613da81d8ca9fec5557ddf6f3d4a390e29fdc395040b66b34 |
| joe__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 2011741d712571b38b0f2786ced0e2ae9d60ef41a9134693215aa9ad321f5043 |
| joe__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 1dbfad0326d1dd8975cd08ad17206c84ee3dbc815e0de1ba84a06f374a759e2b |
| mike__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3ee7bb12dfcc21edf06b6de95b89f59b19ff1abc30733bbceb3605e164982f38 |
| mike__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | cb2a8ae6560b381bc9211556052b1b0c101a4aba4b0b842bc1080af05fe6d745 |
| lee__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c269b45b454d7c5e6fa0d1381cc217eb279414005f431a60255f4bd71016cd00 |
| lee__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d5677df68fd013a1e73a0b388adbd4edcbcc811510add3582cae4a966086acc2 |
| gen__sf6__new__old__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | 8277ac96918e97ae5f94dce4ba5268ebbcfe4603064594ab62f5c6859a6ecb84 |
| gen__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f97c594c6d73263e8ec953144a329e59448b5a23db16c5151308d43de4ded4d7 |
| birdie__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | af0e5f37db187aef52cfee5b8eddca43e1d81723b46b8ac7e074df73cebe50f0 |
| birdie__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5ac2a1c61174dbadea5c3787ddcc9f912e7180cb7dacc54181e5fbcaeb5192c4 |
| eagle__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3bfd12588e7a771bf6308ff0fbf900f5faf83a112d0e5bf8d7e51af340fdcf93 |
| eagle__sf6__old__new__4.tsv | 39 / 1 / 0 | 39 / 1 / 0 | db726b3d7df764b69f7fb4b4f9885dd64dcc4e77a42cea7f721e2e9a86977e5f |
| adon__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | c8cd3606d5eb3cce196cee7586beb5a6b72903a48cdf3e0b9f46101168843366 |
| adon__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 95275fd971d3b11843746cdbc16381b84e925f87e692588a92b86123b4afcee1 |
| sagat__sf6__new__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 720da26138bcc03db718b52df726587656a2d3a69c8c2ffd1ee68b9bb1752eba |
| sagat__sf6__old__new__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 93d57342bff2295fbca1775adaa253e3ad3df7263cdbf89cc006d52c30b58766 |
| ryu__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8c784937010d2b3f067566056e5b6f2eb66ce39f3b2ff6fe4fb8f805e7673cf5 |
| ryu__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 9babfe0ef29b865bd45b9b575f9f1ab9548c7bed53b91ecc71578d78e064d05b |
| ken__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 0606b85bf4f60bb4ec5df6213f460d01f7b754a66f17d7409b60605e0d7fdbe1 |
| ken__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e0dee9ce51b310158a3a8e11565b10968d9aa3019b4fb3460fca58d59cadc217 |
| retsu__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 362e6379b82b04e95f359e3dbbff7b1c1bed322aae494763f0766cf0f3ed2c3f |
| retsu__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 44aaec5d25209f0cfc42a6335d4401f722dee9a62a2311ef247503dd2271ee1b |
| geki__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 007be1ab0187e5861016983f537a8109ebadb20dd5104f65617e1508e947b2d3 |
| geki__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 4ad934df7f5438d6a3451624c9b41482d941439b11203a3cf18a8fce39420211 |
| joe__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 60a8fcbc965c5d3337e6fc16955f019f5a36f22447a6fd2a3fefebb0640b9d3e |
| joe__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3dc5b30cbb9e3f91336b1e49d5d47e542d10d58f37c1d90ed3e6c7365eef73ba |
| mike__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 3bb8ffdc5fab293f56ce53e7d72b47926a807ae9fe8fb4e90eed64d0156f1b48 |
| mike__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 70113930fbf2d5142162289eeb0b73ec095302542868638afe41832350762816 |
| lee__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | d35aa8f630620dced78ec936aad491ed062bcb915d966084da7e51631eab800e |
| lee__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | dec419af553be94f2d7207c58567770aeb4b0d5e3fcb3280490283b6671a8fad |
| gen__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7a853f922751c9be4bdd58fdd1c5338e8818a4e7ae84a3b9fe82e6ac008f6564 |
| gen__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b7582f00203e262df39e6614c46e26d852d1610e282c2cfc721f08e59baa0ce5 |
| birdie__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 8302753b9c489a9f9bfd90a4253c99ed8dc32e0b89f63f5fa55526fb20f795c4 |
| birdie__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 77e1ccb5fcff67b5f2a691d4c5b327185f8895c0f6b501f251dbebc2a262bf43 |
| eagle__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5cbaee14add585f211112062cfd024bdebe96f237fb1672d06c965cb2fa214ac |
| eagle__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | f935ea213fbadd50ffc4e38f7b007630da8b29fd4df32baf804a186fecf95b02 |
| adon__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | e751c35fe37a3eb11f9b2e5d25ebc2106b25c05bfba19c60ea4a5895c81eebb0 |
| adon__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 7af5089cb90d848cc77fb62260a602d0aa2a568885c1607973352a9c7ba714cb |
| sagat__arcade1987__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | b229dab86031dd398c38b946501dcd74e2ddc56a87be6fb594701075db6232a8 |
| sagat__sf1plus__old__old__4.tsv | 40 / 0 / 0 | 40 / 0 / 0 | 5a0763527b06fa4d451f1b76039c883af8828b310ec79deb446ee7aef18270f6 |

## Provenance and preserved artifacts

| Artifact | Source | Version | Executable SHA-256 | Manifest SHA-256 | Raw fingerprint | Frozen provenance SHA-256 | Owned build SHA-256 | Build log SHA-256 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Final | 13a3e219b131e924e469d8d6f29d6369c8ee177e | 1.5.33 | 95a31d9ba80719d77d7317fed5ef832338f20509dc6f5fd4f4f38cf6f200efcc | 152d8044d0a70378e56f5d57e3232adb8b7934f7dba89793eb6f8b43821fd6c9 | d5201908fd59231a8db766b413f2b1d318bcea844717e33996afd1cad2f932d3 | ccae1c838bec14647aea71a1bcdd8705c71dcce0b5a7f0f86d464a0dd941d1bb | 6e84b5d2625e45b06ddd66a3974775723fc396d4e8dd804c105cba995abdb1d0 | 442e68bafac5539fd53fe36bb7a619f10d07dbb5509289eeca3b3ed58176fee1 |
| Baseline controls | ea4da73cb5db6d2c351d0bf8c05846ec5ba58441 | 1.5.32 | 59bbbf99790a7a2090a371a2a6580441f6d19527fae84e03773ffbac0110cb91 | e82f89f09a88cec89cd36c44e225130f519c2898bf371cfd105ff2f53906f0a3 | 4375ebe8a73da0a4700c54d4e775c7324245f24b0f38b0cdf38b9186472d2e04 | 40d6c328f4d67f98968eb12f5ce04e78bc4e945c8851a4b9778ec1be4c0ade4c | ed2ce52331a042a7a51e56292d581ab5904c2b9a96b56589fab0145964244a7b | 8e5282daf1f263617b7c9ef20a8ce8b9cf61b318fd17964e20c4989595aa3a01 |
| Current controls | 13a3e219b131e924e469d8d6f29d6369c8ee177e | 1.5.33 | 95a31d9ba80719d77d7317fed5ef832338f20509dc6f5fd4f4f38cf6f200efcc | 6ae2b8e58f7f553f6ba0b47faf3aeb0eceb182e121bac5d756526e55206229ea | 4375ebe8a73da0a4700c54d4e775c7324245f24b0f38b0cdf38b9186472d2e04 | ccae1c838bec14647aea71a1bcdd8705c71dcce0b5a7f0f86d464a0dd941d1bb | 6e84b5d2625e45b06ddd66a3974775723fc396d4e8dd804c105cba995abdb1d0 | 442e68bafac5539fd53fe36bb7a619f10d07dbb5509289eeca3b3ed58176fee1 |

Each raw fingerprint hashes sorted UTF-8 filenames, NUL, complete raw bytes and NUL. Frozen executables, provenance, owned build records, build logs, per-cell execution records and stdout logs were present and rehashed. The owned command and zero exit status were verified; the before/after committed source/tree/version and resulting executable hashes agree. This generator does not alter those artifacts or the measurement manifests.

- Final: raw folder `C:\Users\George\Projects\sf1-port\target\sc21\final-13a3e219b131`; executable `D:\sf1-port-storage\build\original-target\sc21\bin\cpu_measure-sc21-13a3e219b131.exe`; provenance `C:\Users\George\Projects\sf1-port\target\sc21\bin\measurement-13a3e219b131-current.json`; owned build `D:\sf1-port-storage\build\original-target\sc21\owned-final-13a3e219b131.build.json`; build log `D:\sf1-port-storage\build\original-target\sc21\owned-final-13a3e219b131.log`.
- Baseline controls: raw folder `C:\Users\George\Projects\sf1-port\target\sc21\baseline-controls`; executable `D:\sf1-port-storage\build\original-target\sc21\bin\cpu_measure-sc21-ea4da73cb5db.exe`; provenance `D:\sf1-port-storage\build\original-target\sc21\bin\measurement-ea4da73cb5db-baseline.json`; owned build `D:\sf1-port-storage\build\original-target\sc21\baseline-build.build.json`; build log `D:\sf1-port-storage\build\original-target\sc21\baseline-build.log`.
- Current controls: raw folder `C:\Users\George\Projects\sf1-port\target\sc21\controls-13a3e219b131`; executable `D:\sf1-port-storage\build\original-target\sc21\bin\cpu_measure-sc21-13a3e219b131.exe`; provenance `C:\Users\George\Projects\sf1-port\target\sc21\bin\measurement-13a3e219b131-current.json`; owned build `D:\sf1-port-storage\build\original-target\sc21\owned-final-13a3e219b131.build.json`; build log `D:\sf1-port-storage\build\original-target\sc21\owned-final-13a3e219b131.log`.

## Validation and review

Full suites are permitted under the current maintainer instruction. The explicit checks below record which focused and full-suite commands actually ran, their exits and any skips. Permission alone establishes no pass.

Workspace metadata and all 11 local lockfile packages were checked against reported version `1.5.33`. The validation and review outcomes below are explicit supplied claims with their preserved evidence, not deductions from measurement wins.

- **passed**, focused-native-and-library, source `0e774d7870acc0ac0407c6c4fe9cbbecb01476ad`, observed command exit `0`: 428 library and 17 native tests passed: 445 total, with no failures, ignored tests, filtered tests or reported resource skips. This was a plain debug execution.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/diagnostics/focused-joe-ordinary-meaty-fallback-v1.log rtk proxy cargo test --locked -p sf-engine --lib --test stronger_cpu_sf1 -j1 -- --show-output --test-threads=1
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\prepared-joe-sf1plus-normal-meaty-fallback-v1-20261126\root-focused-execution-v1.json`, SHA-256 `a10344d51240d8b5770066709c0d6cdc8359a47676523afbbebebd56623871c4`.

Source qualification: Actual authoringHEAD0e774 plus ordinary Core3ed WIP, original measureE528. All seven production files are byte-identical to committed finalM13a; Git revision metadata differs. This check is not restamped as an execution at13a.

- **passed**, standard-final-execution, source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, observed command exit `0`: All 360 cells and 86,400 matches completed, with no stops, unfinished matches or draws. Primary gates pass in 191/192 cells; adjacent gates pass in 130/144. All 192 primary idle bars pass. Fifteen win bars remain unmet; 24 old/old cells are ungraded. Strict exit 0 validates execution and provenance.

```text
rtk proxy python -X utf8 target/sc21/strict_check_run.py check target/sc21/final-13a3e219b131 final --frozen target/sc21/bin/measurement-13a3e219b131-current.json --expect-role current --output target/sc21/strict-final-13a3e219b131.json
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-13a3e219b131\final-proof-v1.json`, SHA-256 `80b41941198056769974397521d47c22f17cedd23407bf8dc14f5c6cfe5c866e`.

- **passed**, compatibility, source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, observed command exit `0`: All 434 raw files match exact EA4/v1.5.32 byte for byte, including 198 legacy branch stops. Each binary has 17,360 attempts: 17,162 complete, 198 stopped and none unfinished. No raw difference was introduced.

```text
rtk proxy python -X utf8 target/sc21/compare_controls.py target/sc21/baseline-controls target/sc21/controls-13a3e219b131
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-13a3e219b131\controls-proof-v1.json`, SHA-256 `69f3adba6dff41132dae2da17c6e83e4315d978e55b078983b18a00bcf0dce31`.

- **passed**, native-arcade-capcom-versus-stronger-activity, source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, observed command exit `0`: The exact native test passed, with 16 other tests filtered out. All 40 trials naturally completed across 20 paired fixtures. Attack rate rises in 12 pairs, idle falls in 2, and both improve in 2. Pooled attacks/minute rise from 40.396557 to 47.708766; idle rises from 23.579184% to 43.735696%. This is mixed descriptive evidence.

```text
rtk proxy python -X utf8 target/sc21/analyze_arcade_activity_v5.py target/sc21/closure-13a3e219b131/arcade-activity.log --repo C:/Users/George/Projects/sf1-port --source-sha 13a3e219b131e924e469d8d6f29d6369c8ee177e --out target/sc21/arcade-activity-13a3e219b131-v5
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-13a3e219b131\arcade-activity-v5-proof-v1.json`, SHA-256 `528687a29e292a21dd0ce5133d18f249e124f6c96aceeb55ce85d6ecadf9ecd0`.

- **passed**, supported-dramatic-sf1plus, source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, observed command exit `0`: All 48 SF1+ cells and 1,920 fights completed, with no stops or unfinished fights: 341 boss wins, 1,577 allies wins and 2 draws. No never-lost-round flags were raised. Three distinct stronger controllers use independent lifebars and no friendly fire. No numerical boss-win bar was added.

```text
rtk proxy python -X utf8 target/sc21/measure_dramatic_sc21_v2.py analyze --out target/sc21/dramatic-sf1plus-v2-13a3e219b131
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-13a3e219b131\dramatic-v2-proof-v1.json`, SHA-256 `d985b3c9efd41d090ed63b9dae569383d63ec5185f1a29a8e76293b76d2f8a5d`.

- **passed**, other-preset-native-rule-gates, source `0e774d7870acc0ac0407c6c4fe9cbbecb01476ad`, observed command exit `0`: The 17 native tests include passing rules, parity, routing and replay fixtures. Classic, Pure and Custom rule exclusions are functional coverage, separate from the Definitive-only Local measuring grid.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/diagnostics/focused-joe-ordinary-meaty-fallback-v1.log rtk proxy cargo test --locked -p sf-engine --lib --test stronger_cpu_sf1 -j1 -- --show-output --test-threads=1
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\prepared-joe-sf1plus-normal-meaty-fallback-v1-20261126\root-focused-execution-v1.json`, SHA-256 `a10344d51240d8b5770066709c0d6cdc8359a47676523afbbebebd56623871c4`.

Source qualification: Same production files as finalM; actual test authoringHEAD plus Core3ed overlay as recorded.

- **passed**, game-runtime-version, source `fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`, observed command exit `0`: Street Fighter': The World Tournament v1.5.33 (fd9f7059). Final-code build and banner both exited 0; all eleven local lockfile versions are 1.5.33.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/closure-fd9f70596ddf/game-version.log rtk proxy C:/Users/George/Projects/sf1-port/target/suite/sf-game.exe --version
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-fd9f70596ddf\game-proof-v1.json`, SHA-256 `3b1f700b871837f7f4dc8147ca05832fceb64d995d6f5e30ad25a359029223cc`.

- **failed**, formatting-probe, source `0e774d7870acc0ac0407c6c4fe9cbbecb01476ad`, observed command exit `1`: The whole-file check returned exit 1. Independent archived baseline and final checks also returned 1, with 9,123 and 9,131 diff lines. The new Joe hunk differs from rustfmt output too. Manual Standards review found no documented breach; no whole-file reformat was applied.

```text
rtk proxy rustfmt --edition 2021 --config skip_children=true --check crates/sf-engine/src/fair_cpu.rs
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\standards-final-13a3-v1\review.md`, SHA-256 `605d063b0a9fc531f80bfddf737836c7952291c40d368071935a6915cfe759f8`.

Source qualification: Initial check ran against0e+Core3ed, the same raw file later committed atM.

- **failed**, full-suite-parallel, source `13a3e219b131e924e469d8d6f29d6369c8ee177e`, observed command exit `1`: The complete initial pass ran 2,494 tests: 2,493 passed, one stale World Tour assertion failed, and the terminal summary reports three skipped tests. Nextest exited 100; the outer wrapper exited 1. Fresh JUnit preserves the one failure and has 2,494 selected cases with no skipped case elements. The fixture rejected legal normal-only route frames. This pass is not relabeled green.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/closure-13a3e219b131/suite-parallel.log rtk proxy cargo suite-parallel
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-13a3e219b131\suite-parallel-execution-v1.json`, SHA-256 `f758b04e1cd55bbc40ce649e6ba3a2fc0d8700cf21a649ca10181fdf5e614dd6`.

- **passed**, full-suite-serial, source `fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`, observed command exit `0`: All 166 selected serial tests passed, with one skipped test reported by nextest. Archived JUnit contains 166 selected passing cases, no failures, errors or flaky results. The run completed after the corrected focused case, alone.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/closure-fd9f70596ddf/suite-serial.log rtk proxy cargo suite-serial
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-fd9f70596ddf\suite-serial-execution-v1.json`, SHA-256 `e722319a4414cc3b6670f3d9af53ef99be825d198d7d03355a1226c4d4139513`.

- **passed**, corrected-world-tour-mission-test, source `fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`, observed command exit `0`: The corrected test passed at committed fd9: one passed, ten filtered, no ignored tests or resource skips. Both SF6 and SF1+ exercise unrestricted special choices and prohibit special-bearing routes under the mission, while retaining legal normal links, native-stop checks and continued attacks. The initial full-pass failure remains recorded separately.

```text
rtk proxy python target/sc19/run_test_logged.py target/sc21/diagnostics/world-tour-pure-footsies-expectation-green-v1.log rtk proxy cargo test --locked --profile suite -p sf-engine --test dramatic_world_tour_stronger_cpu -j1 a_world_tour_mission_of_normals_and_throws_is_kept_to_by_the_stronger_play -- --exact --show-output --test-threads=1
```

Evidence: `C:\Users\George\Projects\sf1-port\target\sc21\closure-fd9f70596ddf\focused-world-tour-correction-execution-v1.json`, SHA-256 `8f6aa7e657df77536e0d3a590acfd8d0a1093260b864898bc0cf8785f194800d`.

- **Standards review**, `ea4da73cb5db6d2c351d0bf8c05846ec5ba58441..fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`; remaining actionable findings: 0. No documented breaches or actionable smells. The original complete source review plus the exact test-only correction bind to final code. The formatting probe remains separately reported as nonzero. Evidence: Exact committed scope and independent review evidence; no performance or human approval inferred.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc21\standards-final-fd9-v1\review.md`, SHA-256 `d5354869025b5195b2d48a80c96c9a565aaa3963f6822ee89c77f9a1f075f614`.

- **Spec review**, `ea4da73cb5db6d2c351d0bf8c05846ec5ba58441..fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`; remaining actionable findings: 0. No actionable implementation or scope defects: the complete EA4-to-M review, exact test candidate review and independent committed M-to-N tree binding cover final code. All 4,246 other tracked entries are identical; M measurements remain labeled M. Evidence: Exact committed scope and independent review evidence; no performance or human approval inferred.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc21\spec-final-fd9f705-runtime-binding-v1-20261126\review.md`, SHA-256 `6af555e30d0f1ab8437f3c32a32343062178da595ab4557aaea9643325052fd7`.

- **Report review**, `ea4da73cb5db6d2c351d0bf8c05846ec5ba58441..13a3e219b131e924e469d8d6f29d6369c8ee177e`; remaining actionable findings: 0. Independent numerical/provenance review of the measured-source draft verified all 360 metric rows, 2,160 descriptive means, 1,440 integer totals, 15 failed cells and all 434 compatibility records. Editorial spacing and the explicit all-192-idle/15-win distinction were corrected; final publication delta is recorded separately. Evidence: The measured draft review remains scoped to its exact hash; later code/test-result/link/prose changes receive a separate final-delta review.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc21\review-sc21-report-draft-13a3-v1-20261126\review.md`, SHA-256 `837636394eba9d069f9b8f9226a9a3d128a13bd3fb0a980be55d132433bcfc62`.

- **Report review**, `13a3e219b131e924e469d8d6f29d6369c8ee177e..fd9f70596ddf58cf2bb0da015c7d7b43daa93abd`; remaining actionable findings: 0. Independent final test-evidence audit verified all 2,660 selected identities have passing evidence: 2,493 initial M parallel passes, the exact corrected N focused case, and 166 N serial passes. The original parallel failure and four not-selected skips remain explicit; filters are complementary and every other source entry is unchanged. Evidence: This is completed evidence coverage, not a claim that both literal full-suite passes were all green.

Preserved review file: `C:\Users\George\Projects\sf1-port\target\sc21\final-coverage-m13-fd9-independent-v1-20261126\review.md`, SHA-256 `6bcb9be4ccf1e6b846d9d823bab02b683dd08808ba5774b72159d24d128af243`.

Runtime/delivery source qualification: Every tracked entry except the reviewed World Tour integration test is identical. Production logic, plans, rules, assets, Cargo metadata/lockfile, measuring CLI/factory/observer and all other tests are unchanged. The fix checks special-bearing routes and strengthens its positive/native-stop guards. Compiled Git revision metadata may differ; every measured row and frozen executable remains historical source13a3e219.

```text
rtk proxy python -I -X utf8 target/sc21/verify-runtime-equivalence-13a-fd9-v1.py
```

Observed exit `0`; evidence `C:\Users\George\Projects\sf1-port\target\sc21\runtime-equivalence-13a-fd9-v1.json`, SHA-256 `f7719fc6febbccfc8681a5d1e2eadb0c35bdda028bbbf99724da84da558b0495`.

Human playtest status supplied for this report: No maintainer playtest or SC24 activation approval for source13a3e219 is recorded. Human review remains required.

Local primary cells compare the old Fair CPU to the new Fair CPU, as the approved measuring design defines. They do not measure former native Capcom pacing. All 192 primary exact weighted-idle bars pass. One primary and 14 adjacent win bars fail under both nominal and all-attempt denominators. Every failed cell remains listed; the thresholds are unchanged.

The [native Arcade activity report](sc21-native-arcade-activity.md) is separate: ten authored nonshotos, cabinet modes 2 and 4, at level 4 only. All 40 trials naturally completed, including six below 600 live frames. Ryu and Ken have no authored Capcom baseline here. Pooled attack rate rises while idle worsens; only Retsu mode 2 and Mike mode 2 improve both. Guard can count as idle, sampling may merge adjacent observed attacks, and trajectories differ. The synthetic ladder fixture is not an authored campaign-node test or human playtest.

The [SF1+ Dramatic report](sc21-dramatic-measuring-run.md) covers the supported 48-cell grid. Dramatic permits SF1+ and excludes 1987. The original 96-cell proposal stopped after 40 configuration rejections and zero matches; 95 cells and 3,800 slots were never executed. An initial count-one launcher also failed before any seed because it used the engine-crate working directory. A corrected repository-root calibration naturally completed one fight and matches full-run seed 1. Both failures remain preserved. The completed independent audit passed 403 checks. No numerical boss-win bar or unbeatable claim is added.

After the native activity test passed, V4 analysis rejected Cargo captured-output ordering. That failure and the separately identified V5 status-ownership counterexample remain preserved. The final V5 passed independent checks of 30 layouts, 56 inherited cases and 44 minimal-fixture removals before one actual analysis. Native matches were not rerun to repair the analysis.

Controls use 40 seeds per cell; they are not new 240-seed runs of every historical era family. All 434 TSVs match genuine EA4/v1.5.32, including its 198 existing untranslated-branch failures. Stopped attempts are not completed wins. Existing [engine branch work #270](https://github.com/johnstreetfighter/sf1-port/issues/270) remains separate from SC21 win gaps.

Descriptive rounds, openings, HPO, DPO and follow values use the validated native observer and reported ratios, with the documented rounding and sampling limits. They add no acceptance thresholds. Rejected Geki, Gen and Birdie trials and falsified causal hypotheses remain archived. The remaining win misses have unresolved causes and are not fifteen presumed gameplay bugs.

World Tour 1987 routing was proved with an explicit engine fixture; normal authored Tour nodes do not offer arbitrary 1987 opponent selection. Stronger-on replays remain in the current session because the replay file does not persist the developer flag. Old disk recordings remain valid takeover fixtures.

Validation covers all 2,660 selected test identities: 2,493 passing cases from the complete initial parallel pass, the corrected focused case at fd9, and 166 serial passes at fd9. The initial parallel pass still exited 100 because of one outdated assertion; its failure is preserved. The corrected test and all serial tests passed separately. Nextest reported three plus one not-selected skips, outside the selected JUnit cases. The independent coverage audit found no consistency defects. The unchanged tests were not repeated solely for a test-only correction; this is not a claim of two all-green full-suite passes.

Measurement source is M13a; delivery code fd9 differs only in one World Tour integration test. The old assertion prohibited all routes, although this mission permits normal-only sequences. It now prohibits motion specials, era commands and special-bearing routes, strengthens the unrestricted positive control and checks native stops. All other 4,246 tracked entries, including gameplay, collector, observer, plans, assets, Cargo metadata and other tests, are unchanged. No measurement is restamped as fd9 and no additional gameplay tuning was introduced.

The remaining fifteen win cells, exact activity values, reproduction commands and causal limits are tracked in [SC21 performance follow-up #277](https://github.com/johnstreetfighter/sf1-port/issues/277). This includes the separate native Capcom pacing observation without adding a new numerical threshold. Implementation/report delivery does not waive those failed bars or authorize SC24 activation.

The stronger CPU remains off by default. CPU wire identity is unchanged. No SC24 human approval or whole-cast readiness is supplied. Prior SC15-SC20 reports remain separate evidence; this report covers the 24 baseline pairs. See the [SC24 human playtest handoff](sc24-playtest-handoff.md) for the verified build, controls and remaining decision.

## Per-fighter and per-pair results

| Fighter | Primary pass / cells | Adjacent pass / cells | Nominal misses | Pessimistic win misses | Stopped / unfinished |
| --- | --- | --- | --- | --- | --- |
| ryu | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| ken | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| retsu | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| geki | 16 / 16 | 10 / 12 | 2 | 2 | 0 / 0 |
| joe | 16 / 16 | 10 / 12 | 2 | 2 | 0 / 0 |
| mike | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| lee | 15 / 16 | 10 / 12 | 3 | 3 | 0 / 0 |
| gen | 16 / 16 | 8 / 12 | 4 | 4 | 0 / 0 |
| birdie | 16 / 16 | 8 / 12 | 4 | 4 | 0 / 0 |
| eagle | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| adon | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |
| sagat | 16 / 16 | 12 / 12 | 0 | 0 | 0 / 0 |

| Fighter | Style | Primary pass /8 | Adjacent pass /6 | Nominal reasons | Pessimistic win misses | Stopped / unfinished |
| --- | --- | --- | --- | --- | --- | --- |
| ryu | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| ryu | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| ken | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| ken | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| retsu | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| retsu | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| geki | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| geki | sf1plus | 8 | 4 | MISS (wins) | 2 | 0 / 0 |
| joe | arcade1987 | 8 | 5 | MISS (wins) | 1 | 0 / 0 |
| joe | sf1plus | 8 | 5 | MISS (wins) | 1 | 0 / 0 |
| mike | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| mike | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| lee | arcade1987 | 7 | 5 | MISS (wins) | 2 | 0 / 0 |
| lee | sf1plus | 8 | 5 | MISS (wins) | 1 | 0 / 0 |
| gen | arcade1987 | 8 | 4 | MISS (wins) | 2 | 0 / 0 |
| gen | sf1plus | 8 | 4 | MISS (wins) | 2 | 0 / 0 |
| birdie | arcade1987 | 8 | 2 | MISS (wins) | 4 | 0 / 0 |
| birdie | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| eagle | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| eagle | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| adon | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| adon | sf1plus | 8 | 6 | none | 0 | 0 / 0 |
| sagat | arcade1987 | 8 | 6 | none | 0 | 0 / 0 |
| sagat | sf1plus | 8 | 6 | none | 0 | 0 / 0 |

## Every nominal miss and measured reason

Reasons identify failed measured bars (`wins`, `idle`, `unfinished`), not an inferred gameplay root cause. Every failed cell remains in the complete tables below.

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lee | arcade1987 | 3 | new P2 | 104-136-0 | 240 / 240 / 0 / 0 | 39.74/51.62 | 45.24/37.18 | MISS (wins) | miss | lee__arcade1987__old__new__3.tsv |
| geki | sf1plus | 2/1 | higher P1 | 106-134-0 | 240 / 240 / 0 / 0 | 34.83/36.79 | 50.41/48.14 | MISS (wins) | miss | geki__sf1plus__new__new__2-1.tsv |
| geki | sf1plus | 1/2 | higher P2 | 99-141-0 | 240 / 240 / 0 / 0 | 34.37/36.92 | 50.17/48.28 | MISS (wins) | miss | geki__sf1plus__new__new__1-2.tsv |
| joe | arcade1987 | 2/3 | higher P2 | 119-121-0 | 240 / 240 / 0 / 0 | 36.02/30.26 | 34.80/39.19 | MISS (wins) | miss | joe__arcade1987__new__new__2-3.tsv |
| joe | sf1plus | 1/2 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 33.91/35.03 | 41.71/41.78 | MISS (wins) | miss | joe__sf1plus__new__new__1-2.tsv |
| lee | arcade1987 | 3/4 | higher P2 | 117-123-0 | 240 / 240 / 0 / 0 | 43.05/42.97 | 39.98/42.99 | MISS (wins) | miss | lee__arcade1987__new__new__3-4.tsv |
| lee | sf1plus | 3/4 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 38.35/36.65 | 45.24/47.88 | MISS (wins) | miss | lee__sf1plus__new__new__3-4.tsv |
| gen | arcade1987 | 3/2 | higher P1 | 74-166-0 | 240 / 240 / 0 / 0 | 47.41/44.73 | 38.98/43.81 | MISS (wins) | miss | gen__arcade1987__new__new__3-2.tsv |
| gen | arcade1987 | 2/3 | higher P2 | 65-175-0 | 240 / 240 / 0 / 0 | 46.98/45.16 | 38.72/43.79 | MISS (wins) | miss | gen__arcade1987__new__new__2-3.tsv |
| gen | sf1plus | 3/2 | higher P1 | 109-131-0 | 240 / 240 / 0 / 0 | 42.82/41.52 | 45.21/44.82 | MISS (wins) | miss | gen__sf1plus__new__new__3-2.tsv |
| gen | sf1plus | 2/3 | higher P2 | 96-144-0 | 240 / 240 / 0 / 0 | 42.21/41.80 | 44.52/45.05 | MISS (wins) | miss | gen__sf1plus__new__new__2-3.tsv |
| birdie | arcade1987 | 3/2 | higher P1 | 116-124-0 | 240 / 240 / 0 / 0 | 27.38/30.80 | 53.66/53.58 | MISS (wins) | miss | birdie__arcade1987__new__new__3-2.tsv |
| birdie | arcade1987 | 2/3 | higher P2 | 93-147-0 | 240 / 240 / 0 / 0 | 28.04/30.35 | 52.53/54.72 | MISS (wins) | miss | birdie__arcade1987__new__new__2-3.tsv |
| birdie | arcade1987 | 4/3 | higher P1 | 76-164-0 | 240 / 240 / 0 / 0 | 27.82/29.13 | 54.12/56.61 | MISS (wins) | miss | birdie__arcade1987__new__new__4-3.tsv |
| birdie | arcade1987 | 3/4 | higher P2 | 73-167-0 | 240 / 240 / 0 / 0 | 28.22/28.42 | 53.83/57.08 | MISS (wins) | miss | birdie__arcade1987__new__new__3-4.tsv |

## Pessimistic win misses

15 total; 0 are additional to nominally passing cells. These use all attempted seeds.

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lee | arcade1987 | 3 | new P2 | 104-136-0 | 240 / 240 / 0 / 0 | 39.74/51.62 | 45.24/37.18 | MISS (wins) | miss | lee__arcade1987__old__new__3.tsv |
| geki | sf1plus | 2/1 | higher P1 | 106-134-0 | 240 / 240 / 0 / 0 | 34.83/36.79 | 50.41/48.14 | MISS (wins) | miss | geki__sf1plus__new__new__2-1.tsv |
| geki | sf1plus | 1/2 | higher P2 | 99-141-0 | 240 / 240 / 0 / 0 | 34.37/36.92 | 50.17/48.28 | MISS (wins) | miss | geki__sf1plus__new__new__1-2.tsv |
| joe | arcade1987 | 2/3 | higher P2 | 119-121-0 | 240 / 240 / 0 / 0 | 36.02/30.26 | 34.80/39.19 | MISS (wins) | miss | joe__arcade1987__new__new__2-3.tsv |
| joe | sf1plus | 1/2 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 33.91/35.03 | 41.71/41.78 | MISS (wins) | miss | joe__sf1plus__new__new__1-2.tsv |
| lee | arcade1987 | 3/4 | higher P2 | 117-123-0 | 240 / 240 / 0 / 0 | 43.05/42.97 | 39.98/42.99 | MISS (wins) | miss | lee__arcade1987__new__new__3-4.tsv |
| lee | sf1plus | 3/4 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 38.35/36.65 | 45.24/47.88 | MISS (wins) | miss | lee__sf1plus__new__new__3-4.tsv |
| gen | arcade1987 | 3/2 | higher P1 | 74-166-0 | 240 / 240 / 0 / 0 | 47.41/44.73 | 38.98/43.81 | MISS (wins) | miss | gen__arcade1987__new__new__3-2.tsv |
| gen | arcade1987 | 2/3 | higher P2 | 65-175-0 | 240 / 240 / 0 / 0 | 46.98/45.16 | 38.72/43.79 | MISS (wins) | miss | gen__arcade1987__new__new__2-3.tsv |
| gen | sf1plus | 3/2 | higher P1 | 109-131-0 | 240 / 240 / 0 / 0 | 42.82/41.52 | 45.21/44.82 | MISS (wins) | miss | gen__sf1plus__new__new__3-2.tsv |
| gen | sf1plus | 2/3 | higher P2 | 96-144-0 | 240 / 240 / 0 / 0 | 42.21/41.80 | 44.52/45.05 | MISS (wins) | miss | gen__sf1plus__new__new__2-3.tsv |
| birdie | arcade1987 | 3/2 | higher P1 | 116-124-0 | 240 / 240 / 0 / 0 | 27.38/30.80 | 53.66/53.58 | MISS (wins) | miss | birdie__arcade1987__new__new__3-2.tsv |
| birdie | arcade1987 | 2/3 | higher P2 | 93-147-0 | 240 / 240 / 0 / 0 | 28.04/30.35 | 52.53/54.72 | MISS (wins) | miss | birdie__arcade1987__new__new__2-3.tsv |
| birdie | arcade1987 | 4/3 | higher P1 | 76-164-0 | 240 / 240 / 0 / 0 | 27.82/29.13 | 54.12/56.61 | MISS (wins) | miss | birdie__arcade1987__new__new__4-3.tsv |
| birdie | arcade1987 | 3/4 | higher P2 | 73-167-0 | 240 / 240 / 0 / 0 | 28.22/28.42 | 53.83/57.08 | MISS (wins) | miss | birdie__arcade1987__new__new__3-4.tsv |

### Additional misses beyond nominal passes

None.

## Complete final measuring tables

### All 192 new-versus-old cells

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | arcade1987 | 1 | new P1 | 232-8-0 | 240 / 240 / 0 / 0 | 28.71/46.57 | 47.75/24.25 | pass | pass | ryu__arcade1987__new__old__1.tsv |
| ryu | arcade1987 | 1 | new P2 | 230-10-0 | 240 / 240 / 0 / 0 | 29.27/45.48 | 46.69/24.19 | pass | pass | ryu__arcade1987__old__new__1.tsv |
| ryu | arcade1987 | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 28.93/41.64 | 48.68/27.44 | pass | pass | ryu__arcade1987__new__old__2.tsv |
| ryu | arcade1987 | 2 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 29.40/41.74 | 49.10/26.49 | pass | pass | ryu__arcade1987__old__new__2.tsv |
| ryu | arcade1987 | 3 | new P1 | 228-12-0 | 240 / 240 / 0 / 0 | 26.99/40.39 | 51.14/32.77 | pass | pass | ryu__arcade1987__new__old__3.tsv |
| ryu | arcade1987 | 3 | new P2 | 233-7-0 | 240 / 240 / 0 / 0 | 26.90/40.50 | 51.79/33.01 | pass | pass | ryu__arcade1987__old__new__3.tsv |
| ryu | arcade1987 | 4 | new P1 | 222-18-0 | 240 / 240 / 0 / 0 | 26.94/43.32 | 53.50/36.05 | pass | pass | ryu__arcade1987__new__old__4.tsv |
| ryu | arcade1987 | 4 | new P2 | 220-20-0 | 240 / 240 / 0 / 0 | 25.97/43.60 | 54.82/35.42 | pass | pass | ryu__arcade1987__old__new__4.tsv |
| ryu | sf1plus | 1 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 28.83/46.33 | 47.64/23.57 | pass | pass | ryu__sf1plus__new__old__1.tsv |
| ryu | sf1plus | 1 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 28.48/46.04 | 48.07/24.22 | pass | pass | ryu__sf1plus__old__new__1.tsv |
| ryu | sf1plus | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 26.12/41.05 | 51.13/27.91 | pass | pass | ryu__sf1plus__new__old__2.tsv |
| ryu | sf1plus | 2 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 26.32/41.04 | 51.43/28.49 | pass | pass | ryu__sf1plus__old__new__2.tsv |
| ryu | sf1plus | 3 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 25.57/38.24 | 58.10/30.05 | pass | pass | ryu__sf1plus__new__old__3.tsv |
| ryu | sf1plus | 3 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 25.49/38.51 | 57.92/30.84 | pass | pass | ryu__sf1plus__old__new__3.tsv |
| ryu | sf1plus | 4 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 24.01/38.58 | 63.72/32.37 | pass | pass | ryu__sf1plus__new__old__4.tsv |
| ryu | sf1plus | 4 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 24.04/38.15 | 63.85/32.49 | pass | pass | ryu__sf1plus__old__new__4.tsv |
| ken | arcade1987 | 1 | new P1 | 232-8-0 | 240 / 240 / 0 / 0 | 28.71/46.57 | 47.75/24.25 | pass | pass | ken__arcade1987__new__old__1.tsv |
| ken | arcade1987 | 1 | new P2 | 230-10-0 | 240 / 240 / 0 / 0 | 29.27/45.48 | 46.69/24.19 | pass | pass | ken__arcade1987__old__new__1.tsv |
| ken | arcade1987 | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 28.93/41.64 | 48.68/27.44 | pass | pass | ken__arcade1987__new__old__2.tsv |
| ken | arcade1987 | 2 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 29.40/41.74 | 49.10/26.49 | pass | pass | ken__arcade1987__old__new__2.tsv |
| ken | arcade1987 | 3 | new P1 | 228-12-0 | 240 / 240 / 0 / 0 | 26.99/40.39 | 51.14/32.77 | pass | pass | ken__arcade1987__new__old__3.tsv |
| ken | arcade1987 | 3 | new P2 | 233-7-0 | 240 / 240 / 0 / 0 | 26.90/40.50 | 51.79/33.01 | pass | pass | ken__arcade1987__old__new__3.tsv |
| ken | arcade1987 | 4 | new P1 | 222-18-0 | 240 / 240 / 0 / 0 | 26.94/43.32 | 53.50/36.05 | pass | pass | ken__arcade1987__new__old__4.tsv |
| ken | arcade1987 | 4 | new P2 | 220-20-0 | 240 / 240 / 0 / 0 | 25.97/43.60 | 54.82/35.42 | pass | pass | ken__arcade1987__old__new__4.tsv |
| ken | sf1plus | 1 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 28.83/46.32 | 47.66/23.56 | pass | pass | ken__sf1plus__new__old__1.tsv |
| ken | sf1plus | 1 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 28.46/46.05 | 48.12/24.21 | pass | pass | ken__sf1plus__old__new__1.tsv |
| ken | sf1plus | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 26.14/41.01 | 51.10/27.95 | pass | pass | ken__sf1plus__new__old__2.tsv |
| ken | sf1plus | 2 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 26.32/41.03 | 51.40/28.50 | pass | pass | ken__sf1plus__old__new__2.tsv |
| ken | sf1plus | 3 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 25.57/38.24 | 58.07/30.08 | pass | pass | ken__sf1plus__new__old__3.tsv |
| ken | sf1plus | 3 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 25.50/38.50 | 57.94/30.86 | pass | pass | ken__sf1plus__old__new__3.tsv |
| ken | sf1plus | 4 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 24.01/38.58 | 63.72/32.37 | pass | pass | ken__sf1plus__new__old__4.tsv |
| ken | sf1plus | 4 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 24.02/38.14 | 63.89/32.52 | pass | pass | ken__sf1plus__old__new__4.tsv |
| retsu | arcade1987 | 1 | new P1 | 235-5-0 | 240 / 240 / 0 / 0 | 44.20/58.41 | 42.56/19.44 | pass | pass | retsu__arcade1987__new__old__1.tsv |
| retsu | arcade1987 | 1 | new P2 | 236-4-0 | 240 / 240 / 0 / 0 | 44.71/58.37 | 42.08/19.02 | pass | pass | retsu__arcade1987__old__new__1.tsv |
| retsu | arcade1987 | 2 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 44.93/55.98 | 40.86/18.70 | pass | pass | retsu__arcade1987__new__old__2.tsv |
| retsu | arcade1987 | 2 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 45.25/55.91 | 41.09/18.56 | pass | pass | retsu__arcade1987__old__new__2.tsv |
| retsu | arcade1987 | 3 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 42.20/54.87 | 44.32/18.51 | pass | pass | retsu__arcade1987__new__old__3.tsv |
| retsu | arcade1987 | 3 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 41.99/54.82 | 44.68/18.75 | pass | pass | retsu__arcade1987__old__new__3.tsv |
| retsu | arcade1987 | 4 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 40.18/57.94 | 47.62/15.37 | pass | pass | retsu__arcade1987__new__old__4.tsv |
| retsu | arcade1987 | 4 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 40.00/58.27 | 47.83/15.51 | pass | pass | retsu__arcade1987__old__new__4.tsv |
| retsu | sf1plus | 1 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 44.46/58.49 | 42.04/18.98 | pass | pass | retsu__sf1plus__new__old__1.tsv |
| retsu | sf1plus | 1 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 44.43/58.14 | 41.75/19.73 | pass | pass | retsu__sf1plus__old__new__1.tsv |
| retsu | sf1plus | 2 | new P1 | 238-2-0 | 240 / 240 / 0 / 0 | 42.10/57.62 | 44.19/18.89 | pass | pass | retsu__sf1plus__new__old__2.tsv |
| retsu | sf1plus | 2 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 41.83/58.19 | 44.16/18.11 | pass | pass | retsu__sf1plus__old__new__2.tsv |
| retsu | sf1plus | 3 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 35.95/58.82 | 49.81/18.35 | pass | pass | retsu__sf1plus__new__old__3.tsv |
| retsu | sf1plus | 3 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 35.73/58.71 | 49.63/18.07 | pass | pass | retsu__sf1plus__old__new__3.tsv |
| retsu | sf1plus | 4 | new P1 | 234-6-0 | 240 / 240 / 0 / 0 | 35.35/60.99 | 51.34/15.57 | pass | pass | retsu__sf1plus__new__old__4.tsv |
| retsu | sf1plus | 4 | new P2 | 234-6-0 | 240 / 240 / 0 / 0 | 35.49/60.76 | 51.62/15.42 | pass | pass | retsu__sf1plus__old__new__4.tsv |
| geki | arcade1987 | 1 | new P1 | 210-30-0 | 240 / 240 / 0 / 0 | 33.06/54.96 | 53.81/26.10 | pass | pass | geki__arcade1987__new__old__1.tsv |
| geki | arcade1987 | 1 | new P2 | 212-28-0 | 240 / 240 / 0 / 0 | 33.32/54.78 | 53.70/26.32 | pass | pass | geki__arcade1987__old__new__1.tsv |
| geki | arcade1987 | 2 | new P1 | 199-41-0 | 240 / 240 / 0 / 0 | 36.57/49.18 | 53.89/34.61 | pass | pass | geki__arcade1987__new__old__2.tsv |
| geki | arcade1987 | 2 | new P2 | 202-38-0 | 240 / 240 / 0 / 0 | 36.25/49.40 | 53.87/34.49 | pass | pass | geki__arcade1987__old__new__2.tsv |
| geki | arcade1987 | 3 | new P1 | 224-16-0 | 240 / 240 / 0 / 0 | 38.11/47.69 | 57.41/38.91 | pass | pass | geki__arcade1987__new__old__3.tsv |
| geki | arcade1987 | 3 | new P2 | 225-15-0 | 240 / 240 / 0 / 0 | 37.78/47.31 | 56.86/38.53 | pass | pass | geki__arcade1987__old__new__3.tsv |
| geki | arcade1987 | 4 | new P1 | 235-5-0 | 240 / 240 / 0 / 0 | 40.92/49.33 | 58.15/44.41 | pass | pass | geki__arcade1987__new__old__4.tsv |
| geki | arcade1987 | 4 | new P2 | 223-17-0 | 240 / 240 / 0 / 0 | 40.93/49.53 | 57.99/43.38 | pass | pass | geki__arcade1987__old__new__4.tsv |
| geki | sf1plus | 1 | new P1 | 229-11-0 | 240 / 240 / 0 / 0 | 34.34/52.63 | 52.33/27.75 | pass | pass | geki__sf1plus__new__old__1.tsv |
| geki | sf1plus | 1 | new P2 | 232-8-0 | 240 / 240 / 0 / 0 | 34.34/52.44 | 52.40/27.78 | pass | pass | geki__sf1plus__old__new__1.tsv |
| geki | sf1plus | 2 | new P1 | 228-12-0 | 240 / 240 / 0 / 0 | 31.74/49.22 | 57.72/33.25 | pass | pass | geki__sf1plus__new__old__2.tsv |
| geki | sf1plus | 2 | new P2 | 230-10-0 | 240 / 240 / 0 / 0 | 31.17/50.15 | 58.48/32.19 | pass | pass | geki__sf1plus__old__new__2.tsv |
| geki | sf1plus | 3 | new P1 | 225-15-0 | 240 / 240 / 0 / 0 | 30.89/48.53 | 62.03/38.33 | pass | pass | geki__sf1plus__new__old__3.tsv |
| geki | sf1plus | 3 | new P2 | 226-14-0 | 240 / 240 / 0 / 0 | 30.70/49.02 | 62.48/38.46 | pass | pass | geki__sf1plus__old__new__3.tsv |
| geki | sf1plus | 4 | new P1 | 219-21-0 | 240 / 240 / 0 / 0 | 32.44/49.27 | 63.24/44.47 | pass | pass | geki__sf1plus__new__old__4.tsv |
| geki | sf1plus | 4 | new P2 | 224-16-0 | 240 / 240 / 0 / 0 | 31.91/49.95 | 64.54/43.31 | pass | pass | geki__sf1plus__old__new__4.tsv |
| joe | arcade1987 | 1 | new P1 | 213-27-0 | 240 / 240 / 0 / 0 | 35.19/51.45 | 46.40/17.78 | pass | pass | joe__arcade1987__new__old__1.tsv |
| joe | arcade1987 | 1 | new P2 | 223-17-0 | 240 / 240 / 0 / 0 | 35.03/51.74 | 47.21/17.50 | pass | pass | joe__arcade1987__old__new__1.tsv |
| joe | arcade1987 | 2 | new P1 | 223-17-0 | 240 / 240 / 0 / 0 | 35.31/46.68 | 48.21/20.28 | pass | pass | joe__arcade1987__new__old__2.tsv |
| joe | arcade1987 | 2 | new P2 | 224-16-0 | 240 / 240 / 0 / 0 | 35.36/46.71 | 47.75/20.46 | pass | pass | joe__arcade1987__old__new__2.tsv |
| joe | arcade1987 | 3 | new P1 | 222-18-0 | 240 / 240 / 0 / 0 | 35.90/46.39 | 49.93/20.29 | pass | pass | joe__arcade1987__new__old__3.tsv |
| joe | arcade1987 | 3 | new P2 | 210-30-0 | 240 / 240 / 0 / 0 | 35.85/45.97 | 49.70/21.10 | pass | pass | joe__arcade1987__old__new__3.tsv |
| joe | arcade1987 | 4 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 38.64/55.83 | 54.10/15.15 | pass | pass | joe__arcade1987__new__old__4.tsv |
| joe | arcade1987 | 4 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 38.97/55.79 | 53.62/15.67 | pass | pass | joe__arcade1987__old__new__4.tsv |
| joe | sf1plus | 1 | new P1 | 172-68-0 | 240 / 240 / 0 / 0 | 35.50/50.48 | 44.17/21.28 | pass | pass | joe__sf1plus__new__old__1.tsv |
| joe | sf1plus | 1 | new P2 | 169-71-0 | 240 / 240 / 0 / 0 | 35.82/50.71 | 43.36/20.91 | pass | pass | joe__sf1plus__old__new__1.tsv |
| joe | sf1plus | 2 | new P1 | 185-55-0 | 240 / 240 / 0 / 0 | 33.73/45.95 | 47.83/25.00 | pass | pass | joe__sf1plus__new__old__2.tsv |
| joe | sf1plus | 2 | new P2 | 177-63-0 | 240 / 240 / 0 / 0 | 33.50/45.88 | 47.64/25.45 | pass | pass | joe__sf1plus__old__new__2.tsv |
| joe | sf1plus | 3 | new P1 | 189-51-0 | 240 / 240 / 0 / 0 | 32.89/45.77 | 50.69/24.65 | pass | pass | joe__sf1plus__new__old__3.tsv |
| joe | sf1plus | 3 | new P2 | 177-63-0 | 240 / 240 / 0 / 0 | 32.87/46.10 | 50.54/24.55 | pass | pass | joe__sf1plus__old__new__3.tsv |
| joe | sf1plus | 4 | new P1 | 190-50-0 | 240 / 240 / 0 / 0 | 20.17/74.52 | 89.32/11.85 | pass | pass | joe__sf1plus__new__old__4.tsv |
| joe | sf1plus | 4 | new P2 | 186-54-0 | 240 / 240 / 0 / 0 | 19.79/75.37 | 90.17/11.40 | pass | pass | joe__sf1plus__old__new__4.tsv |
| mike | arcade1987 | 1 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 39.54/49.44 | 45.39/19.13 | pass | pass | mike__arcade1987__new__old__1.tsv |
| mike | arcade1987 | 1 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 39.63/49.34 | 46.26/19.29 | pass | pass | mike__arcade1987__old__new__1.tsv |
| mike | arcade1987 | 2 | new P1 | 231-9-0 | 240 / 240 / 0 / 0 | 36.66/43.03 | 45.05/23.51 | pass | pass | mike__arcade1987__new__old__2.tsv |
| mike | arcade1987 | 2 | new P2 | 236-4-0 | 240 / 240 / 0 / 0 | 37.53/42.77 | 46.07/22.66 | pass | pass | mike__arcade1987__old__new__2.tsv |
| mike | arcade1987 | 3 | new P1 | 234-6-0 | 240 / 240 / 0 / 0 | 36.94/39.68 | 47.44/25.81 | pass | pass | mike__arcade1987__new__old__3.tsv |
| mike | arcade1987 | 3 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 37.89/39.69 | 48.33/24.69 | pass | pass | mike__arcade1987__old__new__3.tsv |
| mike | arcade1987 | 4 | new P1 | 199-41-0 | 240 / 240 / 0 / 0 | 34.55/45.09 | 50.24/25.97 | pass | pass | mike__arcade1987__new__old__4.tsv |
| mike | arcade1987 | 4 | new P2 | 198-42-0 | 240 / 240 / 0 / 0 | 34.97/44.94 | 50.53/25.82 | pass | pass | mike__arcade1987__old__new__4.tsv |
| mike | sf1plus | 1 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 39.36/49.80 | 44.13/18.37 | pass | pass | mike__sf1plus__new__old__1.tsv |
| mike | sf1plus | 1 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 39.02/50.10 | 45.17/17.83 | pass | pass | mike__sf1plus__old__new__1.tsv |
| mike | sf1plus | 2 | new P1 | 234-6-0 | 240 / 240 / 0 / 0 | 34.33/44.50 | 48.28/21.70 | pass | pass | mike__sf1plus__new__old__2.tsv |
| mike | sf1plus | 2 | new P2 | 238-2-0 | 240 / 240 / 0 / 0 | 34.89/43.97 | 48.98/21.64 | pass | pass | mike__sf1plus__old__new__2.tsv |
| mike | sf1plus | 3 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 32.75/41.25 | 53.42/22.76 | pass | pass | mike__sf1plus__new__old__3.tsv |
| mike | sf1plus | 3 | new P2 | 236-4-0 | 240 / 240 / 0 / 0 | 33.03/41.85 | 52.08/22.11 | pass | pass | mike__sf1plus__old__new__3.tsv |
| mike | sf1plus | 4 | new P1 | 194-46-0 | 240 / 240 / 0 / 0 | 27.87/51.59 | 65.64/23.62 | pass | pass | mike__sf1plus__new__old__4.tsv |
| mike | sf1plus | 4 | new P2 | 183-57-0 | 240 / 240 / 0 / 0 | 28.32/50.01 | 62.16/24.63 | pass | pass | mike__sf1plus__old__new__4.tsv |
| lee | arcade1987 | 1 | new P1 | 160-80-0 | 240 / 240 / 0 / 0 | 44.76/61.36 | 40.07/22.92 | pass | pass | lee__arcade1987__new__old__1.tsv |
| lee | arcade1987 | 1 | new P2 | 134-106-0 | 240 / 240 / 0 / 0 | 44.30/61.66 | 40.60/22.87 | pass | pass | lee__arcade1987__old__new__1.tsv |
| lee | arcade1987 | 2 | new P1 | 157-83-0 | 240 / 240 / 0 / 0 | 42.46/54.20 | 42.01/32.09 | pass | pass | lee__arcade1987__new__old__2.tsv |
| lee | arcade1987 | 2 | new P2 | 123-117-0 | 240 / 240 / 0 / 0 | 43.02/54.16 | 40.98/32.24 | pass | pass | lee__arcade1987__old__new__2.tsv |
| lee | arcade1987 | 3 | new P1 | 153-87-0 | 240 / 240 / 0 / 0 | 39.44/51.86 | 45.95/36.44 | pass | pass | lee__arcade1987__new__old__3.tsv |
| lee | arcade1987 | 3 | new P2 | 104-136-0 | 240 / 240 / 0 / 0 | 39.74/51.62 | 45.24/37.18 | MISS (wins) | miss | lee__arcade1987__old__new__3.tsv |
| lee | arcade1987 | 4 | new P1 | 193-47-0 | 240 / 240 / 0 / 0 | 37.38/53.12 | 51.50/39.91 | pass | pass | lee__arcade1987__new__old__4.tsv |
| lee | arcade1987 | 4 | new P2 | 131-109-0 | 240 / 240 / 0 / 0 | 37.49/53.87 | 50.33/40.03 | pass | pass | lee__arcade1987__old__new__4.tsv |
| lee | sf1plus | 1 | new P1 | 191-49-0 | 240 / 240 / 0 / 0 | 45.92/60.69 | 36.92/21.90 | pass | pass | lee__sf1plus__new__old__1.tsv |
| lee | sf1plus | 1 | new P2 | 178-62-0 | 240 / 240 / 0 / 0 | 45.92/60.99 | 36.86/21.95 | pass | pass | lee__sf1plus__old__new__1.tsv |
| lee | sf1plus | 2 | new P1 | 194-46-0 | 240 / 240 / 0 / 0 | 40.51/54.45 | 42.92/28.44 | pass | pass | lee__sf1plus__new__old__2.tsv |
| lee | sf1plus | 2 | new P2 | 180-60-0 | 240 / 240 / 0 / 0 | 40.30/54.39 | 42.23/29.99 | pass | pass | lee__sf1plus__old__new__2.tsv |
| lee | sf1plus | 3 | new P1 | 197-43-0 | 240 / 240 / 0 / 0 | 37.33/49.97 | 46.43/32.78 | pass | pass | lee__sf1plus__new__old__3.tsv |
| lee | sf1plus | 3 | new P2 | 177-63-0 | 240 / 240 / 0 / 0 | 37.27/50.58 | 45.99/33.45 | pass | pass | lee__sf1plus__old__new__3.tsv |
| lee | sf1plus | 4 | new P1 | 211-29-0 | 240 / 240 / 0 / 0 | 34.71/51.30 | 51.55/34.69 | pass | pass | lee__sf1plus__new__old__4.tsv |
| lee | sf1plus | 4 | new P2 | 207-33-0 | 240 / 240 / 0 / 0 | 34.61/51.51 | 50.59/34.69 | pass | pass | lee__sf1plus__old__new__4.tsv |
| gen | arcade1987 | 1 | new P1 | 212-28-0 | 240 / 240 / 0 / 0 | 39.87/63.30 | 50.29/23.20 | pass | pass | gen__arcade1987__new__old__1.tsv |
| gen | arcade1987 | 1 | new P2 | 204-36-0 | 240 / 240 / 0 / 0 | 39.57/62.74 | 50.44/23.73 | pass | pass | gen__arcade1987__old__new__1.tsv |
| gen | arcade1987 | 2 | new P1 | 207-33-0 | 240 / 240 / 0 / 0 | 45.80/53.43 | 43.44/34.16 | pass | pass | gen__arcade1987__new__old__2.tsv |
| gen | arcade1987 | 2 | new P2 | 191-49-0 | 240 / 240 / 0 / 0 | 45.72/53.61 | 43.97/34.50 | pass | pass | gen__arcade1987__old__new__2.tsv |
| gen | arcade1987 | 3 | new P1 | 167-73-0 | 240 / 240 / 0 / 0 | 46.84/50.09 | 43.83/41.27 | pass | pass | gen__arcade1987__new__old__3.tsv |
| gen | arcade1987 | 3 | new P2 | 153-87-0 | 240 / 240 / 0 / 0 | 46.62/50.19 | 43.55/42.29 | pass | pass | gen__arcade1987__old__new__3.tsv |
| gen | arcade1987 | 4 | new P1 | 161-79-0 | 240 / 240 / 0 / 0 | 47.28/49.76 | 43.42/42.95 | pass | pass | gen__arcade1987__new__old__4.tsv |
| gen | arcade1987 | 4 | new P2 | 151-89-0 | 240 / 240 / 0 / 0 | 46.92/49.68 | 43.43/43.42 | pass | pass | gen__arcade1987__old__new__4.tsv |
| gen | sf1plus | 1 | new P1 | 222-18-0 | 240 / 240 / 0 / 0 | 41.97/58.78 | 43.73/23.40 | pass | pass | gen__sf1plus__new__old__1.tsv |
| gen | sf1plus | 1 | new P2 | 203-37-0 | 240 / 240 / 0 / 0 | 42.10/58.72 | 43.65/24.07 | pass | pass | gen__sf1plus__old__new__1.tsv |
| gen | sf1plus | 2 | new P1 | 200-40-0 | 240 / 240 / 0 / 0 | 42.07/54.05 | 43.92/30.16 | pass | pass | gen__sf1plus__new__old__2.tsv |
| gen | sf1plus | 2 | new P2 | 197-43-0 | 240 / 240 / 0 / 0 | 41.78/54.15 | 44.22/30.37 | pass | pass | gen__sf1plus__old__new__2.tsv |
| gen | sf1plus | 3 | new P1 | 223-17-0 | 240 / 240 / 0 / 0 | 39.81/54.39 | 47.80/31.58 | pass | pass | gen__sf1plus__new__old__3.tsv |
| gen | sf1plus | 3 | new P2 | 214-26-0 | 240 / 240 / 0 / 0 | 40.85/53.35 | 46.41/33.27 | pass | pass | gen__sf1plus__old__new__3.tsv |
| gen | sf1plus | 4 | new P1 | 208-32-0 | 240 / 240 / 0 / 0 | 39.63/55.09 | 49.61/33.78 | pass | pass | gen__sf1plus__new__old__4.tsv |
| gen | sf1plus | 4 | new P2 | 202-38-0 | 240 / 240 / 0 / 0 | 39.77/54.94 | 49.34/34.45 | pass | pass | gen__sf1plus__old__new__4.tsv |
| birdie | arcade1987 | 1 | new P1 | 209-31-0 | 240 / 240 / 0 / 0 | 30.60/50.62 | 54.27/28.89 | pass | pass | birdie__arcade1987__new__old__1.tsv |
| birdie | arcade1987 | 1 | new P2 | 208-32-0 | 240 / 240 / 0 / 0 | 30.15/50.94 | 54.37/28.93 | pass | pass | birdie__arcade1987__old__new__1.tsv |
| birdie | arcade1987 | 2 | new P1 | 190-50-0 | 240 / 240 / 0 / 0 | 30.15/43.52 | 56.61/38.83 | pass | pass | birdie__arcade1987__new__old__2.tsv |
| birdie | arcade1987 | 2 | new P2 | 185-55-0 | 240 / 240 / 0 / 0 | 30.59/43.29 | 56.12/38.63 | pass | pass | birdie__arcade1987__old__new__2.tsv |
| birdie | arcade1987 | 3 | new P1 | 156-84-0 | 240 / 240 / 0 / 0 | 29.36/42.26 | 60.61/43.45 | pass | pass | birdie__arcade1987__new__old__3.tsv |
| birdie | arcade1987 | 3 | new P2 | 153-87-0 | 240 / 240 / 0 / 0 | 29.32/43.22 | 60.64/42.35 | pass | pass | birdie__arcade1987__old__new__3.tsv |
| birdie | arcade1987 | 4 | new P1 | 186-54-0 | 240 / 240 / 0 / 0 | 28.56/44.88 | 63.13/42.80 | pass | pass | birdie__arcade1987__new__old__4.tsv |
| birdie | arcade1987 | 4 | new P2 | 168-72-0 | 240 / 240 / 0 / 0 | 29.35/45.29 | 62.61/42.11 | pass | pass | birdie__arcade1987__old__new__4.tsv |
| birdie | sf1plus | 1 | new P1 | 226-14-0 | 240 / 240 / 0 / 0 | 30.62/51.15 | 53.74/26.70 | pass | pass | birdie__sf1plus__new__old__1.tsv |
| birdie | sf1plus | 1 | new P2 | 230-10-0 | 240 / 240 / 0 / 0 | 30.37/51.16 | 54.28/26.84 | pass | pass | birdie__sf1plus__old__new__1.tsv |
| birdie | sf1plus | 2 | new P1 | 229-11-0 | 240 / 240 / 0 / 0 | 25.79/45.23 | 60.77/32.69 | pass | pass | birdie__sf1plus__new__old__2.tsv |
| birdie | sf1plus | 2 | new P2 | 227-13-0 | 240 / 240 / 0 / 0 | 26.35/45.23 | 60.26/33.67 | pass | pass | birdie__sf1plus__old__new__2.tsv |
| birdie | sf1plus | 3 | new P1 | 224-16-0 | 240 / 240 / 0 / 0 | 23.16/44.18 | 66.36/35.48 | pass | pass | birdie__sf1plus__new__old__3.tsv |
| birdie | sf1plus | 3 | new P2 | 214-26-0 | 240 / 240 / 0 / 0 | 22.89/43.71 | 65.89/35.97 | pass | pass | birdie__sf1plus__old__new__3.tsv |
| birdie | sf1plus | 4 | new P1 | 226-14-0 | 240 / 240 / 0 / 0 | 22.25/41.36 | 73.49/38.55 | pass | pass | birdie__sf1plus__new__old__4.tsv |
| birdie | sf1plus | 4 | new P2 | 229-11-0 | 240 / 240 / 0 / 0 | 22.42/40.69 | 73.69/39.65 | pass | pass | birdie__sf1plus__old__new__4.tsv |
| eagle | arcade1987 | 1 | new P1 | 231-9-0 | 240 / 240 / 0 / 0 | 48.92/61.43 | 41.66/18.12 | pass | pass | eagle__arcade1987__new__old__1.tsv |
| eagle | arcade1987 | 1 | new P2 | 231-9-0 | 240 / 240 / 0 / 0 | 48.79/61.41 | 41.82/18.20 | pass | pass | eagle__arcade1987__old__new__1.tsv |
| eagle | arcade1987 | 2 | new P1 | 238-2-0 | 240 / 240 / 0 / 0 | 50.69/56.03 | 39.45/18.62 | pass | pass | eagle__arcade1987__new__old__2.tsv |
| eagle | arcade1987 | 2 | new P2 | 233-7-0 | 240 / 240 / 0 / 0 | 51.03/56.06 | 39.21/18.72 | pass | pass | eagle__arcade1987__old__new__2.tsv |
| eagle | arcade1987 | 3 | new P1 | 240-0-0 | 240 / 240 / 0 / 0 | 52.19/54.95 | 40.79/14.85 | pass | pass | eagle__arcade1987__new__old__3.tsv |
| eagle | arcade1987 | 3 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 52.34/53.87 | 40.60/16.90 | pass | pass | eagle__arcade1987__old__new__3.tsv |
| eagle | arcade1987 | 4 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 49.35/56.23 | 47.62/17.16 | pass | pass | eagle__arcade1987__new__old__4.tsv |
| eagle | arcade1987 | 4 | new P2 | 240-0-0 | 240 / 240 / 0 / 0 | 48.90/56.66 | 48.21/16.12 | pass | pass | eagle__arcade1987__old__new__4.tsv |
| eagle | sf1plus | 1 | new P1 | 232-8-0 | 240 / 240 / 0 / 0 | 48.57/60.95 | 39.99/17.93 | pass | pass | eagle__sf1plus__new__old__1.tsv |
| eagle | sf1plus | 1 | new P2 | 235-5-0 | 240 / 240 / 0 / 0 | 48.19/60.88 | 40.85/17.68 | pass | pass | eagle__sf1plus__old__new__1.tsv |
| eagle | sf1plus | 2 | new P1 | 237-3-0 | 240 / 240 / 0 / 0 | 48.80/56.02 | 39.72/17.99 | pass | pass | eagle__sf1plus__new__old__2.tsv |
| eagle | sf1plus | 2 | new P2 | 237-3-0 | 240 / 240 / 0 / 0 | 48.61/55.80 | 39.94/18.12 | pass | pass | eagle__sf1plus__old__new__2.tsv |
| eagle | sf1plus | 3 | new P1 | 239-1-0 | 240 / 240 / 0 / 0 | 49.14/53.86 | 42.81/15.98 | pass | pass | eagle__sf1plus__new__old__3.tsv |
| eagle | sf1plus | 3 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 49.10/54.08 | 42.38/16.61 | pass | pass | eagle__sf1plus__old__new__3.tsv |
| eagle | sf1plus | 4 | new P1 | 238-2-0 | 240 / 240 / 0 / 0 | 46.28/55.21 | 47.25/17.61 | pass | pass | eagle__sf1plus__new__old__4.tsv |
| eagle | sf1plus | 4 | new P2 | 239-1-0 | 240 / 240 / 0 / 0 | 45.26/54.65 | 48.33/19.31 | pass | pass | eagle__sf1plus__old__new__4.tsv |
| adon | arcade1987 | 1 | new P1 | 158-82-0 | 240 / 240 / 0 / 0 | 43.17/53.75 | 41.40/21.80 | pass | pass | adon__arcade1987__new__old__1.tsv |
| adon | arcade1987 | 1 | new P2 | 159-81-0 | 240 / 240 / 0 / 0 | 42.84/54.01 | 42.13/21.41 | pass | pass | adon__arcade1987__old__new__1.tsv |
| adon | arcade1987 | 2 | new P1 | 194-46-0 | 240 / 240 / 0 / 0 | 43.73/48.60 | 41.17/27.16 | pass | pass | adon__arcade1987__new__old__2.tsv |
| adon | arcade1987 | 2 | new P2 | 196-44-0 | 240 / 240 / 0 / 0 | 43.56/48.48 | 41.36/27.01 | pass | pass | adon__arcade1987__old__new__2.tsv |
| adon | arcade1987 | 3 | new P1 | 213-27-0 | 240 / 240 / 0 / 0 | 43.65/45.93 | 44.02/29.79 | pass | pass | adon__arcade1987__new__old__3.tsv |
| adon | arcade1987 | 3 | new P2 | 205-35-0 | 240 / 240 / 0 / 0 | 42.85/45.94 | 44.34/30.95 | pass | pass | adon__arcade1987__old__new__3.tsv |
| adon | arcade1987 | 4 | new P1 | 207-33-0 | 240 / 240 / 0 / 0 | 39.12/51.61 | 48.97/29.92 | pass | pass | adon__arcade1987__new__old__4.tsv |
| adon | arcade1987 | 4 | new P2 | 207-33-0 | 240 / 240 / 0 / 0 | 38.92/51.94 | 49.05/29.64 | pass | pass | adon__arcade1987__old__new__4.tsv |
| adon | sf1plus | 1 | new P1 | 171-69-0 | 240 / 240 / 0 / 0 | 43.18/53.21 | 40.74/21.97 | pass | pass | adon__sf1plus__new__old__1.tsv |
| adon | sf1plus | 1 | new P2 | 182-58-0 | 240 / 240 / 0 / 0 | 43.25/53.78 | 40.77/21.75 | pass | pass | adon__sf1plus__old__new__1.tsv |
| adon | sf1plus | 2 | new P1 | 202-38-0 | 240 / 240 / 0 / 0 | 42.52/48.81 | 42.04/27.63 | pass | pass | adon__sf1plus__new__old__2.tsv |
| adon | sf1plus | 2 | new P2 | 201-39-0 | 240 / 240 / 0 / 0 | 42.39/48.95 | 42.14/26.83 | pass | pass | adon__sf1plus__old__new__2.tsv |
| adon | sf1plus | 3 | new P1 | 218-22-0 | 240 / 240 / 0 / 0 | 40.55/47.54 | 45.70/27.46 | pass | pass | adon__sf1plus__new__old__3.tsv |
| adon | sf1plus | 3 | new P2 | 208-32-0 | 240 / 240 / 0 / 0 | 39.99/47.58 | 45.61/28.42 | pass | pass | adon__sf1plus__old__new__3.tsv |
| adon | sf1plus | 4 | new P1 | 212-28-0 | 240 / 240 / 0 / 0 | 37.07/51.18 | 50.37/29.07 | pass | pass | adon__sf1plus__new__old__4.tsv |
| adon | sf1plus | 4 | new P2 | 213-27-0 | 240 / 240 / 0 / 0 | 36.92/50.96 | 50.59/29.43 | pass | pass | adon__sf1plus__old__new__4.tsv |
| sagat | arcade1987 | 1 | new P1 | 227-13-0 | 240 / 240 / 0 / 0 | 34.88/60.00 | 56.23/20.21 | pass | pass | sagat__arcade1987__new__old__1.tsv |
| sagat | arcade1987 | 1 | new P2 | 217-23-0 | 240 / 240 / 0 / 0 | 35.10/59.17 | 55.42/21.69 | pass | pass | sagat__arcade1987__old__new__1.tsv |
| sagat | arcade1987 | 2 | new P1 | 209-31-0 | 240 / 240 / 0 / 0 | 37.59/52.82 | 51.82/25.20 | pass | pass | sagat__arcade1987__new__old__2.tsv |
| sagat | arcade1987 | 2 | new P2 | 207-33-0 | 240 / 240 / 0 / 0 | 37.64/52.20 | 52.46/25.67 | pass | pass | sagat__arcade1987__old__new__2.tsv |
| sagat | arcade1987 | 3 | new P1 | 212-28-0 | 240 / 240 / 0 / 0 | 36.36/49.33 | 56.48/30.93 | pass | pass | sagat__arcade1987__new__old__3.tsv |
| sagat | arcade1987 | 3 | new P2 | 199-41-0 | 240 / 240 / 0 / 0 | 36.35/49.51 | 56.20/30.98 | pass | pass | sagat__arcade1987__old__new__3.tsv |
| sagat | arcade1987 | 4 | new P1 | 165-75-0 | 240 / 240 / 0 / 0 | 36.51/49.15 | 57.66/34.69 | pass | pass | sagat__arcade1987__new__old__4.tsv |
| sagat | arcade1987 | 4 | new P2 | 156-84-0 | 240 / 240 / 0 / 0 | 36.96/48.91 | 57.69/35.64 | pass | pass | sagat__arcade1987__old__new__4.tsv |
| sagat | sf1plus | 1 | new P1 | 230-10-0 | 240 / 240 / 0 / 0 | 36.31/54.29 | 47.97/24.04 | pass | pass | sagat__sf1plus__new__old__1.tsv |
| sagat | sf1plus | 1 | new P2 | 232-8-0 | 240 / 240 / 0 / 0 | 35.80/54.60 | 48.89/23.90 | pass | pass | sagat__sf1plus__old__new__1.tsv |
| sagat | sf1plus | 2 | new P1 | 197-43-0 | 240 / 240 / 0 / 0 | 33.91/49.28 | 49.70/27.97 | pass | pass | sagat__sf1plus__new__old__2.tsv |
| sagat | sf1plus | 2 | new P2 | 204-36-0 | 240 / 240 / 0 / 0 | 34.15/49.61 | 49.28/27.46 | pass | pass | sagat__sf1plus__old__new__2.tsv |
| sagat | sf1plus | 3 | new P1 | 200-40-0 | 240 / 240 / 0 / 0 | 31.24/48.55 | 51.13/30.58 | pass | pass | sagat__sf1plus__new__old__3.tsv |
| sagat | sf1plus | 3 | new P2 | 188-52-0 | 240 / 240 / 0 / 0 | 31.40/48.63 | 51.24/30.30 | pass | pass | sagat__sf1plus__old__new__3.tsv |
| sagat | sf1plus | 4 | new P1 | 166-74-0 | 240 / 240 / 0 / 0 | 29.90/49.08 | 52.48/32.91 | pass | pass | sagat__sf1plus__new__old__4.tsv |
| sagat | sf1plus | 4 | new P2 | 164-76-0 | 240 / 240 / 0 / 0 | 29.84/49.15 | 51.90/32.81 | pass | pass | sagat__sf1plus__old__new__4.tsv |


### All 144 adjacent-level cells

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | arcade1987 | 2/1 | higher P1 | 215-25-0 | 240 / 240 / 0 / 0 | 30.41/31.79 | 42.28/39.58 | pass | pass | ryu__arcade1987__new__new__2-1.tsv |
| ryu | arcade1987 | 1/2 | higher P2 | 220-20-0 | 240 / 240 / 0 / 0 | 30.35/31.94 | 42.21/39.46 | pass | pass | ryu__arcade1987__new__new__1-2.tsv |
| ryu | arcade1987 | 3/2 | higher P1 | 210-30-0 | 240 / 240 / 0 / 0 | 28.70/28.82 | 45.78/43.89 | pass | pass | ryu__arcade1987__new__new__3-2.tsv |
| ryu | arcade1987 | 2/3 | higher P2 | 222-18-0 | 240 / 240 / 0 / 0 | 28.52/29.22 | 46.49/43.45 | pass | pass | ryu__arcade1987__new__new__2-3.tsv |
| ryu | arcade1987 | 4/3 | higher P1 | 215-25-0 | 240 / 240 / 0 / 0 | 28.17/28.15 | 49.35/47.63 | pass | pass | ryu__arcade1987__new__new__4-3.tsv |
| ryu | arcade1987 | 3/4 | higher P2 | 209-31-0 | 240 / 240 / 0 / 0 | 28.44/27.44 | 48.82/48.23 | pass | pass | ryu__arcade1987__new__new__3-4.tsv |
| ryu | sf1plus | 2/1 | higher P1 | 230-10-0 | 240 / 240 / 0 / 0 | 27.34/29.94 | 45.93/38.83 | pass | pass | ryu__sf1plus__new__new__2-1.tsv |
| ryu | sf1plus | 1/2 | higher P2 | 230-10-0 | 240 / 240 / 0 / 0 | 27.61/29.83 | 45.78/37.66 | pass | pass | ryu__sf1plus__new__new__1-2.tsv |
| ryu | sf1plus | 3/2 | higher P1 | 219-21-0 | 240 / 240 / 0 / 0 | 25.31/25.91 | 48.50/42.59 | pass | pass | ryu__sf1plus__new__new__3-2.tsv |
| ryu | sf1plus | 2/3 | higher P2 | 209-31-0 | 240 / 240 / 0 / 0 | 24.97/25.97 | 47.90/43.05 | pass | pass | ryu__sf1plus__new__new__2-3.tsv |
| ryu | sf1plus | 4/3 | higher P1 | 189-51-0 | 240 / 240 / 0 / 0 | 23.15/23.87 | 50.22/46.71 | pass | pass | ryu__sf1plus__new__new__4-3.tsv |
| ryu | sf1plus | 3/4 | higher P2 | 196-44-0 | 240 / 240 / 0 / 0 | 23.02/23.98 | 50.38/46.86 | pass | pass | ryu__sf1plus__new__new__3-4.tsv |
| ken | arcade1987 | 2/1 | higher P1 | 215-25-0 | 240 / 240 / 0 / 0 | 30.41/31.79 | 42.28/39.58 | pass | pass | ken__arcade1987__new__new__2-1.tsv |
| ken | arcade1987 | 1/2 | higher P2 | 220-20-0 | 240 / 240 / 0 / 0 | 30.35/31.94 | 42.21/39.46 | pass | pass | ken__arcade1987__new__new__1-2.tsv |
| ken | arcade1987 | 3/2 | higher P1 | 210-30-0 | 240 / 240 / 0 / 0 | 28.70/28.82 | 45.78/43.89 | pass | pass | ken__arcade1987__new__new__3-2.tsv |
| ken | arcade1987 | 2/3 | higher P2 | 222-18-0 | 240 / 240 / 0 / 0 | 28.52/29.22 | 46.49/43.45 | pass | pass | ken__arcade1987__new__new__2-3.tsv |
| ken | arcade1987 | 4/3 | higher P1 | 215-25-0 | 240 / 240 / 0 / 0 | 28.17/28.15 | 49.35/47.63 | pass | pass | ken__arcade1987__new__new__4-3.tsv |
| ken | arcade1987 | 3/4 | higher P2 | 209-31-0 | 240 / 240 / 0 / 0 | 28.44/27.44 | 48.82/48.23 | pass | pass | ken__arcade1987__new__new__3-4.tsv |
| ken | sf1plus | 2/1 | higher P1 | 230-10-0 | 240 / 240 / 0 / 0 | 27.28/29.94 | 45.96/38.71 | pass | pass | ken__sf1plus__new__new__2-1.tsv |
| ken | sf1plus | 1/2 | higher P2 | 230-10-0 | 240 / 240 / 0 / 0 | 27.63/29.81 | 45.81/37.66 | pass | pass | ken__sf1plus__new__new__1-2.tsv |
| ken | sf1plus | 3/2 | higher P1 | 219-21-0 | 240 / 240 / 0 / 0 | 25.30/25.90 | 48.48/42.57 | pass | pass | ken__sf1plus__new__new__3-2.tsv |
| ken | sf1plus | 2/3 | higher P2 | 210-30-0 | 240 / 240 / 0 / 0 | 24.97/25.96 | 47.92/43.04 | pass | pass | ken__sf1plus__new__new__2-3.tsv |
| ken | sf1plus | 4/3 | higher P1 | 188-52-0 | 240 / 240 / 0 / 0 | 23.16/23.85 | 50.17/46.77 | pass | pass | ken__sf1plus__new__new__4-3.tsv |
| ken | sf1plus | 3/4 | higher P2 | 195-45-0 | 240 / 240 / 0 / 0 | 23.01/23.97 | 50.37/46.90 | pass | pass | ken__sf1plus__new__new__3-4.tsv |
| retsu | arcade1987 | 2/1 | higher P1 | 173-67-0 | 240 / 240 / 0 / 0 | 44.57/47.75 | 35.53/32.56 | pass | pass | retsu__arcade1987__new__new__2-1.tsv |
| retsu | arcade1987 | 1/2 | higher P2 | 160-80-0 | 240 / 240 / 0 / 0 | 44.65/47.34 | 35.41/33.00 | pass | pass | retsu__arcade1987__new__new__1-2.tsv |
| retsu | arcade1987 | 3/2 | higher P1 | 200-40-0 | 240 / 240 / 0 / 0 | 38.56/42.51 | 42.02/36.67 | pass | pass | retsu__arcade1987__new__new__3-2.tsv |
| retsu | arcade1987 | 2/3 | higher P2 | 198-42-0 | 240 / 240 / 0 / 0 | 38.50/42.22 | 41.93/37.13 | pass | pass | retsu__arcade1987__new__new__2-3.tsv |
| retsu | arcade1987 | 4/3 | higher P1 | 164-76-0 | 240 / 240 / 0 / 0 | 35.17/37.13 | 44.79/43.64 | pass | pass | retsu__arcade1987__new__new__4-3.tsv |
| retsu | arcade1987 | 3/4 | higher P2 | 158-82-0 | 240 / 240 / 0 / 0 | 35.85/36.66 | 44.61/44.17 | pass | pass | retsu__arcade1987__new__new__3-4.tsv |
| retsu | sf1plus | 2/1 | higher P1 | 194-46-0 | 240 / 240 / 0 / 0 | 43.10/44.71 | 38.21/34.95 | pass | pass | retsu__sf1plus__new__new__2-1.tsv |
| retsu | sf1plus | 1/2 | higher P2 | 194-46-0 | 240 / 240 / 0 / 0 | 43.09/44.44 | 38.36/35.19 | pass | pass | retsu__sf1plus__new__new__1-2.tsv |
| retsu | sf1plus | 3/2 | higher P1 | 220-20-0 | 240 / 240 / 0 / 0 | 36.22/40.29 | 45.45/39.05 | pass | pass | retsu__sf1plus__new__new__3-2.tsv |
| retsu | sf1plus | 2/3 | higher P2 | 218-22-0 | 240 / 240 / 0 / 0 | 36.08/40.34 | 45.35/38.83 | pass | pass | retsu__sf1plus__new__new__2-3.tsv |
| retsu | sf1plus | 4/3 | higher P1 | 188-52-0 | 240 / 240 / 0 / 0 | 33.92/34.01 | 46.72/46.27 | pass | pass | retsu__sf1plus__new__new__4-3.tsv |
| retsu | sf1plus | 3/4 | higher P2 | 172-68-0 | 240 / 240 / 0 / 0 | 33.87/33.60 | 46.70/46.98 | pass | pass | retsu__sf1plus__new__new__3-4.tsv |
| geki | arcade1987 | 2/1 | higher P1 | 150-90-0 | 240 / 240 / 0 / 0 | 35.05/36.12 | 49.05/47.52 | pass | pass | geki__arcade1987__new__new__2-1.tsv |
| geki | arcade1987 | 1/2 | higher P2 | 139-101-0 | 240 / 240 / 0 / 0 | 35.00/35.95 | 48.59/47.45 | pass | pass | geki__arcade1987__new__new__1-2.tsv |
| geki | arcade1987 | 3/2 | higher P1 | 139-101-0 | 240 / 240 / 0 / 0 | 35.55/34.32 | 54.67/55.13 | pass | pass | geki__arcade1987__new__new__3-2.tsv |
| geki | arcade1987 | 2/3 | higher P2 | 125-115-0 | 240 / 240 / 0 / 0 | 35.53/34.29 | 54.99/55.62 | pass | pass | geki__arcade1987__new__new__2-3.tsv |
| geki | arcade1987 | 4/3 | higher P1 | 177-63-0 | 240 / 240 / 0 / 0 | 39.09/38.85 | 62.73/54.20 | pass | pass | geki__arcade1987__new__new__4-3.tsv |
| geki | arcade1987 | 3/4 | higher P2 | 175-65-0 | 240 / 240 / 0 / 0 | 38.95/39.45 | 62.74/54.39 | pass | pass | geki__arcade1987__new__new__3-4.tsv |
| geki | sf1plus | 2/1 | higher P1 | 106-134-0 | 240 / 240 / 0 / 0 | 34.83/36.79 | 50.41/48.14 | MISS (wins) | miss | geki__sf1plus__new__new__2-1.tsv |
| geki | sf1plus | 1/2 | higher P2 | 99-141-0 | 240 / 240 / 0 / 0 | 34.37/36.92 | 50.17/48.28 | MISS (wins) | miss | geki__sf1plus__new__new__1-2.tsv |
| geki | sf1plus | 3/2 | higher P1 | 136-104-0 | 240 / 240 / 0 / 0 | 33.18/35.64 | 55.05/51.49 | pass | pass | geki__sf1plus__new__new__3-2.tsv |
| geki | sf1plus | 2/3 | higher P2 | 135-105-0 | 240 / 240 / 0 / 0 | 33.22/35.52 | 54.67/51.53 | pass | pass | geki__sf1plus__new__new__2-3.tsv |
| geki | sf1plus | 4/3 | higher P1 | 154-86-0 | 240 / 240 / 0 / 0 | 33.05/33.39 | 56.86/54.32 | pass | pass | geki__sf1plus__new__new__4-3.tsv |
| geki | sf1plus | 3/4 | higher P2 | 147-93-0 | 240 / 240 / 0 / 0 | 33.43/33.23 | 56.40/54.91 | pass | pass | geki__sf1plus__new__new__3-4.tsv |
| joe | arcade1987 | 2/1 | higher P1 | 134-106-0 | 240 / 240 / 0 / 0 | 34.55/34.09 | 35.20/32.69 | pass | pass | joe__arcade1987__new__new__2-1.tsv |
| joe | arcade1987 | 1/2 | higher P2 | 152-88-0 | 240 / 240 / 0 / 0 | 34.41/34.49 | 35.83/32.05 | pass | pass | joe__arcade1987__new__new__1-2.tsv |
| joe | arcade1987 | 3/2 | higher P1 | 174-66-0 | 240 / 240 / 0 / 0 | 35.03/30.81 | 38.17/33.86 | pass | pass | joe__arcade1987__new__new__3-2.tsv |
| joe | arcade1987 | 2/3 | higher P2 | 119-121-0 | 240 / 240 / 0 / 0 | 36.02/30.26 | 34.80/39.19 | MISS (wins) | miss | joe__arcade1987__new__new__2-3.tsv |
| joe | arcade1987 | 4/3 | higher P1 | 203-37-0 | 240 / 240 / 0 / 0 | 30.49/33.76 | 46.22/37.19 | pass | pass | joe__arcade1987__new__new__4-3.tsv |
| joe | arcade1987 | 3/4 | higher P2 | 203-37-0 | 240 / 240 / 0 / 0 | 30.38/33.66 | 46.20/37.29 | pass | pass | joe__arcade1987__new__new__3-4.tsv |
| joe | sf1plus | 2/1 | higher P1 | 128-112-0 | 240 / 240 / 0 / 0 | 33.98/34.69 | 42.89/43.10 | pass | pass | joe__sf1plus__new__new__2-1.tsv |
| joe | sf1plus | 1/2 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 33.91/35.03 | 41.71/41.78 | MISS (wins) | miss | joe__sf1plus__new__new__1-2.tsv |
| joe | sf1plus | 3/2 | higher P1 | 154-86-0 | 240 / 240 / 0 / 0 | 33.54/33.82 | 45.05/41.29 | pass | pass | joe__sf1plus__new__new__3-2.tsv |
| joe | sf1plus | 2/3 | higher P2 | 166-74-0 | 240 / 240 / 0 / 0 | 33.46/33.63 | 46.14/40.93 | pass | pass | joe__sf1plus__new__new__2-3.tsv |
| joe | sf1plus | 4/3 | higher P1 | 192-48-0 | 240 / 240 / 0 / 0 | 32.85/33.64 | 48.12/43.08 | pass | pass | joe__sf1plus__new__new__4-3.tsv |
| joe | sf1plus | 3/4 | higher P2 | 182-58-0 | 240 / 240 / 0 / 0 | 33.03/33.77 | 47.87/42.00 | pass | pass | joe__sf1plus__new__new__3-4.tsv |
| mike | arcade1987 | 2/1 | higher P1 | 184-56-0 | 240 / 240 / 0 / 0 | 36.05/36.37 | 40.45/38.07 | pass | pass | mike__arcade1987__new__new__2-1.tsv |
| mike | arcade1987 | 1/2 | higher P2 | 202-38-0 | 240 / 240 / 0 / 0 | 36.59/36.24 | 41.00/36.03 | pass | pass | mike__arcade1987__new__new__1-2.tsv |
| mike | arcade1987 | 3/2 | higher P1 | 176-64-0 | 240 / 240 / 0 / 0 | 34.37/32.95 | 41.29/39.65 | pass | pass | mike__arcade1987__new__new__3-2.tsv |
| mike | arcade1987 | 2/3 | higher P2 | 187-53-0 | 240 / 240 / 0 / 0 | 34.95/32.72 | 41.93/38.46 | pass | pass | mike__arcade1987__new__new__2-3.tsv |
| mike | arcade1987 | 4/3 | higher P1 | 167-73-0 | 240 / 240 / 0 / 0 | 32.63/30.72 | 43.04/42.81 | pass | pass | mike__arcade1987__new__new__4-3.tsv |
| mike | arcade1987 | 3/4 | higher P2 | 169-71-0 | 240 / 240 / 0 / 0 | 32.80/30.89 | 43.45/41.93 | pass | pass | mike__arcade1987__new__new__3-4.tsv |
| mike | sf1plus | 2/1 | higher P1 | 184-56-0 | 240 / 240 / 0 / 0 | 33.84/36.30 | 41.91/36.99 | pass | pass | mike__sf1plus__new__new__2-1.tsv |
| mike | sf1plus | 1/2 | higher P2 | 191-49-0 | 240 / 240 / 0 / 0 | 33.90/36.50 | 43.08/36.02 | pass | pass | mike__sf1plus__new__new__1-2.tsv |
| mike | sf1plus | 3/2 | higher P1 | 165-75-0 | 240 / 240 / 0 / 0 | 31.42/31.02 | 43.00/40.11 | pass | pass | mike__sf1plus__new__new__3-2.tsv |
| mike | sf1plus | 2/3 | higher P2 | 170-70-0 | 240 / 240 / 0 / 0 | 32.20/31.20 | 43.85/38.97 | pass | pass | mike__sf1plus__new__new__2-3.tsv |
| mike | sf1plus | 4/3 | higher P1 | 165-75-0 | 240 / 240 / 0 / 0 | 29.78/28.27 | 45.61/42.36 | pass | pass | mike__sf1plus__new__new__4-3.tsv |
| mike | sf1plus | 3/4 | higher P2 | 170-70-0 | 240 / 240 / 0 / 0 | 30.04/28.33 | 45.68/41.83 | pass | pass | mike__sf1plus__new__new__3-4.tsv |
| lee | arcade1987 | 2/1 | higher P1 | 160-80-0 | 240 / 240 / 0 / 0 | 46.16/45.16 | 35.72/40.05 | pass | pass | lee__arcade1987__new__new__2-1.tsv |
| lee | arcade1987 | 1/2 | higher P2 | 137-103-0 | 240 / 240 / 0 / 0 | 46.01/45.75 | 35.84/40.31 | pass | pass | lee__arcade1987__new__new__1-2.tsv |
| lee | arcade1987 | 3/2 | higher P1 | 160-80-0 | 240 / 240 / 0 / 0 | 43.22/45.91 | 39.56/39.93 | pass | pass | lee__arcade1987__new__new__3-2.tsv |
| lee | arcade1987 | 2/3 | higher P2 | 128-112-0 | 240 / 240 / 0 / 0 | 43.89/46.06 | 37.64/40.50 | pass | pass | lee__arcade1987__new__new__2-3.tsv |
| lee | arcade1987 | 4/3 | higher P1 | 158-82-0 | 240 / 240 / 0 / 0 | 43.15/43.70 | 41.59/41.07 | pass | pass | lee__arcade1987__new__new__4-3.tsv |
| lee | arcade1987 | 3/4 | higher P2 | 117-123-0 | 240 / 240 / 0 / 0 | 43.05/42.97 | 39.98/42.99 | MISS (wins) | miss | lee__arcade1987__new__new__3-4.tsv |
| lee | sf1plus | 2/1 | higher P1 | 171-69-0 | 240 / 240 / 0 / 0 | 41.98/43.89 | 40.45/39.41 | pass | pass | lee__sf1plus__new__new__2-1.tsv |
| lee | sf1plus | 1/2 | higher P2 | 163-77-0 | 240 / 240 / 0 / 0 | 41.73/44.14 | 39.96/40.12 | pass | pass | lee__sf1plus__new__new__1-2.tsv |
| lee | sf1plus | 3/2 | higher P1 | 165-75-0 | 240 / 240 / 0 / 0 | 38.45/40.65 | 43.44/43.06 | pass | pass | lee__sf1plus__new__new__3-2.tsv |
| lee | sf1plus | 2/3 | higher P2 | 148-92-0 | 240 / 240 / 0 / 0 | 38.20/40.33 | 43.31/43.95 | pass | pass | lee__sf1plus__new__new__2-3.tsv |
| lee | sf1plus | 4/3 | higher P1 | 141-99-0 | 240 / 240 / 0 / 0 | 39.11/36.17 | 45.32/47.01 | pass | pass | lee__sf1plus__new__new__4-3.tsv |
| lee | sf1plus | 3/4 | higher P2 | 108-132-0 | 240 / 240 / 0 / 0 | 38.35/36.65 | 45.24/47.88 | MISS (wins) | miss | lee__sf1plus__new__new__3-4.tsv |
| gen | arcade1987 | 2/1 | higher P1 | 168-72-0 | 240 / 240 / 0 / 0 | 45.44/47.20 | 39.11/40.51 | pass | pass | gen__arcade1987__new__new__2-1.tsv |
| gen | arcade1987 | 1/2 | higher P2 | 139-101-0 | 240 / 240 / 0 / 0 | 45.71/46.81 | 38.59/41.34 | pass | pass | gen__arcade1987__new__new__1-2.tsv |
| gen | arcade1987 | 3/2 | higher P1 | 74-166-0 | 240 / 240 / 0 / 0 | 47.41/44.73 | 38.98/43.81 | MISS (wins) | miss | gen__arcade1987__new__new__3-2.tsv |
| gen | arcade1987 | 2/3 | higher P2 | 65-175-0 | 240 / 240 / 0 / 0 | 46.98/45.16 | 38.72/43.79 | MISS (wins) | miss | gen__arcade1987__new__new__2-3.tsv |
| gen | arcade1987 | 4/3 | higher P1 | 198-42-0 | 240 / 240 / 0 / 0 | 43.12/46.81 | 41.72/40.79 | pass | pass | gen__arcade1987__new__new__4-3.tsv |
| gen | arcade1987 | 3/4 | higher P2 | 202-38-0 | 240 / 240 / 0 / 0 | 43.17/46.69 | 40.73/41.22 | pass | pass | gen__arcade1987__new__new__3-4.tsv |
| gen | sf1plus | 2/1 | higher P1 | 175-65-0 | 240 / 240 / 0 / 0 | 41.14/45.79 | 43.92/40.12 | pass | pass | gen__sf1plus__new__new__2-1.tsv |
| gen | sf1plus | 1/2 | higher P2 | 172-68-0 | 240 / 240 / 0 / 0 | 40.67/46.06 | 44.07/40.19 | pass | pass | gen__sf1plus__new__new__1-2.tsv |
| gen | sf1plus | 3/2 | higher P1 | 109-131-0 | 240 / 240 / 0 / 0 | 42.82/41.52 | 45.21/44.82 | MISS (wins) | miss | gen__sf1plus__new__new__3-2.tsv |
| gen | sf1plus | 2/3 | higher P2 | 96-144-0 | 240 / 240 / 0 / 0 | 42.21/41.80 | 44.52/45.05 | MISS (wins) | miss | gen__sf1plus__new__new__2-3.tsv |
| gen | sf1plus | 4/3 | higher P1 | 198-42-0 | 240 / 240 / 0 / 0 | 38.31/40.54 | 48.14/45.96 | pass | pass | gen__sf1plus__new__new__4-3.tsv |
| gen | sf1plus | 3/4 | higher P2 | 170-70-0 | 240 / 240 / 0 / 0 | 37.81/40.37 | 47.13/46.72 | pass | pass | gen__sf1plus__new__new__3-4.tsv |
| birdie | arcade1987 | 2/1 | higher P1 | 148-92-0 | 240 / 240 / 0 / 0 | 29.95/33.08 | 52.40/50.44 | pass | pass | birdie__arcade1987__new__new__2-1.tsv |
| birdie | arcade1987 | 1/2 | higher P2 | 145-95-0 | 240 / 240 / 0 / 0 | 30.33/32.71 | 51.43/50.80 | pass | pass | birdie__arcade1987__new__new__1-2.tsv |
| birdie | arcade1987 | 3/2 | higher P1 | 116-124-0 | 240 / 240 / 0 / 0 | 27.38/30.80 | 53.66/53.58 | MISS (wins) | miss | birdie__arcade1987__new__new__3-2.tsv |
| birdie | arcade1987 | 2/3 | higher P2 | 93-147-0 | 240 / 240 / 0 / 0 | 28.04/30.35 | 52.53/54.72 | MISS (wins) | miss | birdie__arcade1987__new__new__2-3.tsv |
| birdie | arcade1987 | 4/3 | higher P1 | 76-164-0 | 240 / 240 / 0 / 0 | 27.82/29.13 | 54.12/56.61 | MISS (wins) | miss | birdie__arcade1987__new__new__4-3.tsv |
| birdie | arcade1987 | 3/4 | higher P2 | 73-167-0 | 240 / 240 / 0 / 0 | 28.22/28.42 | 53.83/57.08 | MISS (wins) | miss | birdie__arcade1987__new__new__3-4.tsv |
| birdie | sf1plus | 2/1 | higher P1 | 181-59-0 | 240 / 240 / 0 / 0 | 27.73/33.64 | 55.65/49.77 | pass | pass | birdie__sf1plus__new__new__2-1.tsv |
| birdie | sf1plus | 1/2 | higher P2 | 154-86-0 | 240 / 240 / 0 / 0 | 28.10/33.45 | 55.69/50.79 | pass | pass | birdie__sf1plus__new__new__1-2.tsv |
| birdie | sf1plus | 3/2 | higher P1 | 190-50-0 | 240 / 240 / 0 / 0 | 25.32/28.77 | 61.68/55.53 | pass | pass | birdie__sf1plus__new__new__3-2.tsv |
| birdie | sf1plus | 2/3 | higher P2 | 168-72-0 | 240 / 240 / 0 / 0 | 25.27/28.82 | 60.43/55.76 | pass | pass | birdie__sf1plus__new__new__2-3.tsv |
| birdie | sf1plus | 4/3 | higher P1 | 180-60-0 | 240 / 240 / 0 / 0 | 23.30/26.02 | 65.66/57.60 | pass | pass | birdie__sf1plus__new__new__4-3.tsv |
| birdie | sf1plus | 3/4 | higher P2 | 187-53-0 | 240 / 240 / 0 / 0 | 23.10/26.25 | 67.26/56.64 | pass | pass | birdie__sf1plus__new__new__3-4.tsv |
| eagle | arcade1987 | 2/1 | higher P1 | 194-46-0 | 240 / 240 / 0 / 0 | 51.68/47.23 | 31.84/31.70 | pass | pass | eagle__arcade1987__new__new__2-1.tsv |
| eagle | arcade1987 | 1/2 | higher P2 | 207-33-0 | 240 / 240 / 0 / 0 | 51.59/47.71 | 32.21/29.03 | pass | pass | eagle__arcade1987__new__new__1-2.tsv |
| eagle | arcade1987 | 3/2 | higher P1 | 160-80-0 | 240 / 240 / 0 / 0 | 53.85/50.07 | 29.07/28.13 | pass | pass | eagle__arcade1987__new__new__3-2.tsv |
| eagle | arcade1987 | 2/3 | higher P2 | 159-81-0 | 240 / 240 / 0 / 0 | 53.88/49.77 | 29.42/29.07 | pass | pass | eagle__arcade1987__new__new__2-3.tsv |
| eagle | arcade1987 | 4/3 | higher P1 | 144-96-0 | 240 / 240 / 0 / 0 | 53.31/54.50 | 31.45/25.59 | pass | pass | eagle__arcade1987__new__new__4-3.tsv |
| eagle | arcade1987 | 3/4 | higher P2 | 160-80-0 | 240 / 240 / 0 / 0 | 53.42/54.53 | 30.95/22.12 | pass | pass | eagle__arcade1987__new__new__3-4.tsv |
| eagle | sf1plus | 2/1 | higher P1 | 202-38-0 | 240 / 240 / 0 / 0 | 49.91/46.40 | 34.57/30.62 | pass | pass | eagle__sf1plus__new__new__2-1.tsv |
| eagle | sf1plus | 1/2 | higher P2 | 210-30-0 | 240 / 240 / 0 / 0 | 50.44/46.97 | 34.85/29.45 | pass | pass | eagle__sf1plus__new__new__1-2.tsv |
| eagle | sf1plus | 3/2 | higher P1 | 210-30-0 | 240 / 240 / 0 / 0 | 51.63/48.55 | 35.60/24.44 | pass | pass | eagle__sf1plus__new__new__3-2.tsv |
| eagle | sf1plus | 2/3 | higher P2 | 204-36-0 | 240 / 240 / 0 / 0 | 51.86/48.36 | 35.79/25.54 | pass | pass | eagle__sf1plus__new__new__2-3.tsv |
| eagle | sf1plus | 4/3 | higher P1 | 227-13-0 | 240 / 240 / 0 / 0 | 52.08/54.84 | 37.28/15.75 | pass | pass | eagle__sf1plus__new__new__4-3.tsv |
| eagle | sf1plus | 3/4 | higher P2 | 228-12-0 | 240 / 240 / 0 / 0 | 51.26/54.36 | 38.90/14.94 | pass | pass | eagle__sf1plus__new__new__3-4.tsv |
| adon | arcade1987 | 2/1 | higher P1 | 197-43-0 | 240 / 240 / 0 / 0 | 46.86/42.01 | 32.53/34.55 | pass | pass | adon__arcade1987__new__new__2-1.tsv |
| adon | arcade1987 | 1/2 | higher P2 | 185-55-0 | 240 / 240 / 0 / 0 | 46.68/42.64 | 32.59/34.43 | pass | pass | adon__arcade1987__new__new__1-2.tsv |
| adon | arcade1987 | 3/2 | higher P1 | 209-31-0 | 240 / 240 / 0 / 0 | 47.26/43.92 | 34.76/29.93 | pass | pass | adon__arcade1987__new__new__3-2.tsv |
| adon | arcade1987 | 2/3 | higher P2 | 195-45-0 | 240 / 240 / 0 / 0 | 46.70/44.52 | 34.67/31.54 | pass | pass | adon__arcade1987__new__new__2-3.tsv |
| adon | arcade1987 | 4/3 | higher P1 | 205-35-0 | 240 / 240 / 0 / 0 | 45.43/44.17 | 37.13/32.02 | pass | pass | adon__arcade1987__new__new__4-3.tsv |
| adon | arcade1987 | 3/4 | higher P2 | 201-39-0 | 240 / 240 / 0 / 0 | 44.98/44.35 | 37.18/32.78 | pass | pass | adon__arcade1987__new__new__3-4.tsv |
| adon | sf1plus | 2/1 | higher P1 | 219-21-0 | 240 / 240 / 0 / 0 | 45.58/43.44 | 36.20/31.88 | pass | pass | adon__sf1plus__new__new__2-1.tsv |
| adon | sf1plus | 1/2 | higher P2 | 204-36-0 | 240 / 240 / 0 / 0 | 45.01/43.51 | 35.75/32.84 | pass | pass | adon__sf1plus__new__new__1-2.tsv |
| adon | sf1plus | 3/2 | higher P1 | 203-37-0 | 240 / 240 / 0 / 0 | 44.18/44.59 | 37.20/30.59 | pass | pass | adon__sf1plus__new__new__3-2.tsv |
| adon | sf1plus | 2/3 | higher P2 | 178-62-0 | 240 / 240 / 0 / 0 | 44.20/44.31 | 36.38/32.93 | pass | pass | adon__sf1plus__new__new__2-3.tsv |
| adon | sf1plus | 4/3 | higher P1 | 208-32-0 | 240 / 240 / 0 / 0 | 43.26/44.66 | 38.61/32.45 | pass | pass | adon__sf1plus__new__new__4-3.tsv |
| adon | sf1plus | 3/4 | higher P2 | 194-46-0 | 240 / 240 / 0 / 0 | 43.63/44.52 | 38.21/33.04 | pass | pass | adon__sf1plus__new__new__3-4.tsv |
| sagat | arcade1987 | 2/1 | higher P1 | 194-46-0 | 240 / 240 / 0 / 0 | 38.33/41.06 | 44.78/39.39 | pass | pass | sagat__arcade1987__new__new__2-1.tsv |
| sagat | arcade1987 | 1/2 | higher P2 | 189-51-0 | 240 / 240 / 0 / 0 | 38.64/41.62 | 43.72/39.85 | pass | pass | sagat__arcade1987__new__new__1-2.tsv |
| sagat | arcade1987 | 3/2 | higher P1 | 185-55-0 | 240 / 240 / 0 / 0 | 34.35/37.57 | 51.71/41.26 | pass | pass | sagat__arcade1987__new__new__3-2.tsv |
| sagat | arcade1987 | 2/3 | higher P2 | 201-39-0 | 240 / 240 / 0 / 0 | 34.36/37.85 | 51.41/40.70 | pass | pass | sagat__arcade1987__new__new__2-3.tsv |
| sagat | arcade1987 | 4/3 | higher P1 | 192-48-0 | 240 / 240 / 0 / 0 | 30.81/33.85 | 55.79/45.75 | pass | pass | sagat__arcade1987__new__new__4-3.tsv |
| sagat | arcade1987 | 3/4 | higher P2 | 180-60-0 | 240 / 240 / 0 / 0 | 30.99/33.54 | 55.61/47.16 | pass | pass | sagat__arcade1987__new__new__3-4.tsv |
| sagat | sf1plus | 2/1 | higher P1 | 189-51-0 | 240 / 240 / 0 / 0 | 35.37/38.19 | 45.55/40.93 | pass | pass | sagat__sf1plus__new__new__2-1.tsv |
| sagat | sf1plus | 1/2 | higher P2 | 193-47-0 | 240 / 240 / 0 / 0 | 35.83/37.96 | 45.62/40.98 | pass | pass | sagat__sf1plus__new__new__1-2.tsv |
| sagat | sf1plus | 3/2 | higher P1 | 145-95-0 | 240 / 240 / 0 / 0 | 32.97/35.90 | 48.88/46.19 | pass | pass | sagat__sf1plus__new__new__3-2.tsv |
| sagat | sf1plus | 2/3 | higher P2 | 136-104-0 | 240 / 240 / 0 / 0 | 32.87/36.42 | 48.90/46.17 | pass | pass | sagat__sf1plus__new__new__2-3.tsv |
| sagat | sf1plus | 4/3 | higher P1 | 169-71-0 | 240 / 240 / 0 / 0 | 31.61/33.83 | 52.68/49.72 | pass | pass | sagat__sf1plus__new__new__4-3.tsv |
| sagat | sf1plus | 3/4 | higher P2 | 154-86-0 | 240 / 240 / 0 / 0 | 31.44/33.74 | 52.77/50.01 | pass | pass | sagat__sf1plus__new__new__3-4.tsv |


### All 24 old-versus-old controls

| Fighter | Style | Levels | Compared side | Finished W-L-D | Attempted / completed / stopped / unfinished | Idle first/second % | Attacks/min first/second | Nominal bar | Pessimistic wins | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | arcade1987 | 4 | old vs old | 115-125-0 | 240 / 240 / 0 / 0 | 36.26/36.25 | 51.97/52.05 | - | - | ryu__arcade1987__old__old__4.tsv |
| ryu | sf1plus | 4 | old vs old | 113-127-0 | 240 / 240 / 0 / 0 | 30.90/30.71 | 49.43/49.97 | - | - | ryu__sf1plus__old__old__4.tsv |
| ken | arcade1987 | 4 | old vs old | 115-125-0 | 240 / 240 / 0 / 0 | 36.26/36.25 | 51.97/52.05 | - | - | ken__arcade1987__old__old__4.tsv |
| ken | sf1plus | 4 | old vs old | 112-128-0 | 240 / 240 / 0 / 0 | 30.90/30.71 | 49.40/49.94 | - | - | ken__sf1plus__old__old__4.tsv |
| retsu | arcade1987 | 4 | old vs old | 111-129-0 | 240 / 240 / 0 / 0 | 47.54/47.85 | 35.44/35.31 | - | - | retsu__arcade1987__old__old__4.tsv |
| retsu | sf1plus | 4 | old vs old | 118-122-0 | 240 / 240 / 0 / 0 | 57.15/57.11 | 28.21/28.26 | - | - | retsu__sf1plus__old__old__4.tsv |
| geki | arcade1987 | 4 | old vs old | 128-112-0 | 240 / 240 / 0 / 0 | 48.09/48.20 | 47.15/46.90 | - | - | geki__arcade1987__old__old__4.tsv |
| geki | sf1plus | 4 | old vs old | 125-115-0 | 240 / 240 / 0 / 0 | 44.46/43.72 | 49.81/50.78 | - | - | geki__sf1plus__old__old__4.tsv |
| joe | arcade1987 | 4 | old vs old | 128-112-0 | 240 / 240 / 0 / 0 | 44.57/44.46 | 34.87/35.03 | - | - | joe__arcade1987__old__old__4.tsv |
| joe | sf1plus | 4 | old vs old | 125-115-0 | 240 / 240 / 0 / 0 | 44.66/45.14 | 35.11/34.30 | - | - | joe__sf1plus__old__old__4.tsv |
| mike | arcade1987 | 4 | old vs old | 108-132-0 | 240 / 240 / 0 / 0 | 43.09/43.66 | 43.52/42.34 | - | - | mike__arcade1987__old__old__4.tsv |
| mike | sf1plus | 4 | old vs old | 111-129-0 | 240 / 240 / 0 / 0 | 42.49/42.24 | 43.16/43.10 | - | - | mike__sf1plus__old__old__4.tsv |
| lee | arcade1987 | 4 | old vs old | 129-111-0 | 240 / 240 / 0 / 0 | 47.02/47.35 | 44.12/42.81 | - | - | lee__arcade1987__old__old__4.tsv |
| lee | sf1plus | 4 | old vs old | 136-104-0 | 240 / 240 / 0 / 0 | 50.01/50.02 | 40.45/40.22 | - | - | lee__sf1plus__old__old__4.tsv |
| gen | arcade1987 | 4 | old vs old | 126-114-0 | 240 / 240 / 0 / 0 | 46.74/46.92 | 44.07/43.75 | - | - | gen__arcade1987__old__old__4.tsv |
| gen | sf1plus | 4 | old vs old | 156-84-0 | 240 / 240 / 0 / 0 | 47.16/47.07 | 47.56/47.53 | - | - | gen__sf1plus__old__old__4.tsv |
| birdie | arcade1987 | 4 | old vs old | 124-116-0 | 240 / 240 / 0 / 0 | 38.57/38.45 | 50.65/50.35 | - | - | birdie__arcade1987__old__old__4.tsv |
| birdie | sf1plus | 4 | old vs old | 134-106-0 | 240 / 240 / 0 / 0 | 36.75/37.11 | 51.81/51.12 | - | - | birdie__sf1plus__old__old__4.tsv |
| eagle | arcade1987 | 4 | old vs old | 119-121-0 | 240 / 240 / 0 / 0 | 49.13/49.44 | 41.69/41.61 | - | - | eagle__arcade1987__old__old__4.tsv |
| eagle | sf1plus | 4 | old vs old | 127-113-0 | 240 / 240 / 0 / 0 | 52.84/52.06 | 37.34/38.08 | - | - | eagle__sf1plus__old__old__4.tsv |
| adon | arcade1987 | 4 | old vs old | 121-119-0 | 240 / 240 / 0 / 0 | 47.24/46.94 | 37.14/37.60 | - | - | adon__arcade1987__old__old__4.tsv |
| adon | sf1plus | 4 | old vs old | 114-126-0 | 240 / 240 / 0 / 0 | 48.56/48.48 | 40.80/40.44 | - | - | adon__sf1plus__old__old__4.tsv |
| sagat | arcade1987 | 4 | old vs old | 139-101-0 | 240 / 240 / 0 / 0 | 44.34/45.56 | 47.25/45.42 | - | - | sagat__arcade1987__old__old__4.tsv |
| sagat | sf1plus | 4 | old vs old | 123-117-0 | 240 / 240 / 0 / 0 | 44.19/43.87 | 45.50/45.81 | - | - | sagat__sf1plus__old__old__4.tsv |


## Complete descriptive opening metrics

All 360 validated final raw files are included below. First/second uses the same compared side as the gate tables: new/old in primary cells, higher/lower in adjacent cells, and P1/P2 in old/old controls. These descriptive values add no acceptance bars.

Recorded rounds and opening counts are summed as integers. HPO is hits per connected opening; DPO is damage per opening in health-bar points (the native ratio divides sub-HP damage by openings times 20). Follow rate is the fraction of openings with a later hit from a different attack, from 0 to 1; it is not a percentage.

The TSV stores each per-match ratio rounded to two decimals, not the underlying hit, damage or followed-opening counter totals. The displayed values are opening-count-weighted means of those reported rounded ratios, separately for each fighter: exact Decimal hundredths times that fighter's opening count, divided by its total openings. They are not exact aggregate ratios reconstructed from the underlying counters. Display uses four Decimal places; a zero-opening side displays 0.0000 and every reported zero-opening ratio is required to be 0.00. Live-frame counts are not weights for these opening metrics.

All recorded rows contribute, including unfinished progress if present; stopped attempts have no descriptive metrics. The native observer has limits: frame attribution of a health drop versus guard/hit reaction has not been validated move by move, so a hit may be filed as chip or conversely; the same attack repeated without an intervening different frame counts as one attack start; round winners/counts are observed from health bars rather than the native round-result event.

| Fighter | Style | Levels | Compared side | Recorded rounds first/second | Recorded openings first/second | Opening-weighted reported HPO first/second | Opening-weighted reported DPO first/second | Opening-weighted reported follow fraction first/second | Raw file |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ryu | arcade1987 | 4 | old vs old | 282/305 | 3519/3483 | 1.0235/1.0265 | 6.4614/6.6735 | 0.0232/0.0259 | ryu__arcade1987__old__old__4.tsv |
| ryu | arcade1987 | 1 | new P1 | 469/65 | 2175/1100 | 1.0081/1.0009 | 11.2180/10.0125 | 0.0072/0.0009 | ryu__arcade1987__new__old__1.tsv |
| ryu | arcade1987 | 1 | new P2 | 469/63 | 2113/1077 | 1.0061/1.0009 | 11.5184/9.9268 | 0.0052/0.0009 | ryu__arcade1987__old__new__1.tsv |
| ryu | arcade1987 | 2 | new P1 | 476/53 | 1989/945 | 1.0020/1.0717 | 12.2199/10.5840 | 0.0020/0.0707 | ryu__arcade1987__new__old__2.tsv |
| ryu | arcade1987 | 2 | new P2 | 475/47 | 1992/893 | 1.0020/1.0906 | 12.1640/10.7266 | 0.0020/0.0895 | ryu__arcade1987__old__new__2.tsv |
| ryu | arcade1987 | 3 | new P1 | 464/61 | 2117/1203 | 1.0019/1.1010 | 11.3920/9.2536 | 0.0009/0.1001 | ryu__arcade1987__new__old__3.tsv |
| ryu | arcade1987 | 3 | new P2 | 471/61 | 2210/1212 | 1.0014/1.0923 | 11.1916/9.6394 | 0.0014/0.0923 | ryu__arcade1987__old__new__3.tsv |
| ryu | arcade1987 | 4 | new P1 | 456/89 | 2473/1553 | 1.0008/1.1257 | 10.0707/8.4740 | 0.0004/0.1251 | ryu__arcade1987__new__old__4.tsv |
| ryu | arcade1987 | 4 | new P2 | 457/91 | 2588/1485 | 1.0000/1.1052 | 9.6269/9.9125 | 0.0000/0.1045 | ryu__arcade1987__old__new__4.tsv |
| ryu | arcade1987 | 2/1 | higher P1 | 447/131 | 1833/1093 | 1.0049/1.0046 | 13.7009/12.9873 | 0.0027/0.0027 | ryu__arcade1987__new__new__2-1.tsv |
| ryu | arcade1987 | 1/2 | higher P2 | 456/126 | 1880/1157 | 1.0063/1.0130 | 13.5972/12.8528 | 0.0031/0.0086 | ryu__arcade1987__new__new__1-2.tsv |
| ryu | arcade1987 | 3/2 | higher P1 | 445/134 | 1743/1025 | 1.0057/1.0039 | 14.4654/14.1657 | 0.0046/0.0010 | ryu__arcade1987__new__new__3-2.tsv |
| ryu | arcade1987 | 2/3 | higher P2 | 463/104 | 1781/952 | 1.0017/1.0041 | 14.2956/14.3666 | 0.0011/0.0011 | ryu__arcade1987__new__new__2-3.tsv |
| ryu | arcade1987 | 4/3 | higher P1 | 448/119 | 1694/970 | 1.0012/1.0020 | 14.8960/14.7670 | 0.0006/0.0010 | ryu__arcade1987__new__new__4-3.tsv |
| ryu | arcade1987 | 3/4 | higher P2 | 443/133 | 1692/1015 | 1.0017/1.0059 | 14.9073/14.6919 | 0.0012/0.0030 | ryu__arcade1987__new__new__3-4.tsv |
| ryu | sf1plus | 4 | old vs old | 290/313 | 3120/3209 | 1.1929/1.1951 | 7.2000/7.2458 | 0.1890/0.1895 | ryu__sf1plus__old__old__4.tsv |
| ryu | sf1plus | 1 | new P1 | 480/7 | 3374/1385 | 1.0331/1.0115 | 6.8963/6.7555 | 0.0269/0.0015 | ryu__sf1plus__new__old__1.tsv |
| ryu | sf1plus | 1 | new P2 | 479/19 | 3432/1457 | 1.0392/1.0164 | 6.9248/6.7873 | 0.0260/0.0027 | ryu__sf1plus__old__new__1.tsv |
| ryu | sf1plus | 2 | new P1 | 476/23 | 3330/1317 | 1.0646/1.1234 | 7.1122/7.5803 | 0.0528/0.1204 | ryu__sf1plus__new__old__2.tsv |
| ryu | sf1plus | 2 | new P2 | 478/23 | 3345/1315 | 1.0688/1.1365 | 7.0906/7.5275 | 0.0584/0.1289 | ryu__sf1plus__old__new__2.tsv |
| ryu | sf1plus | 3 | new P1 | 479/14 | 3313/946 | 1.0786/1.1658 | 7.0849/7.8413 | 0.0620/0.1616 | ryu__sf1plus__new__old__3.tsv |
| ryu | sf1plus | 3 | new P2 | 480/11 | 3343/1084 | 1.0841/1.1705 | 7.0295/7.6856 | 0.0681/0.1650 | ryu__sf1plus__old__new__3.tsv |
| ryu | sf1plus | 4 | new P1 | 479/17 | 3659/935 | 1.1181/1.1977 | 6.4540/7.8224 | 0.0877/0.1977 | ryu__sf1plus__new__old__4.tsv |
| ryu | sf1plus | 4 | new P2 | 480/12 | 3573/872 | 1.1302/1.2002 | 6.5645/7.7889 | 0.0918/0.1933 | ryu__sf1plus__old__new__4.tsv |
| ryu | sf1plus | 2/1 | higher P1 | 469/69 | 3251/1859 | 1.0724/1.0573 | 7.6437/7.4655 | 0.0604/0.0462 | ryu__sf1plus__new__new__2-1.tsv |
| ryu | sf1plus | 1/2 | higher P2 | 467/69 | 3146/1785 | 1.0755/1.0555 | 7.8525/7.4721 | 0.0629/0.0461 | ryu__sf1plus__new__new__1-2.tsv |
| ryu | sf1plus | 3/2 | higher P1 | 458/112 | 3260/2067 | 1.0869/1.1167 | 7.8949/8.0672 | 0.0627/0.0885 | ryu__sf1plus__new__new__3-2.tsv |
| ryu | sf1plus | 2/3 | higher P2 | 435/121 | 3132/2051 | 1.0976/1.1447 | 7.9404/8.3151 | 0.0665/0.1086 | ryu__sf1plus__new__new__2-3.tsv |
| ryu | sf1plus | 4/3 | higher P1 | 415/164 | 3118/2246 | 1.1236/1.1806 | 8.1239/8.4843 | 0.0689/0.1049 | ryu__sf1plus__new__new__4-3.tsv |
| ryu | sf1plus | 3/4 | higher P2 | 423/161 | 3061/2255 | 1.1461/1.1588 | 8.3138/8.4216 | 0.0775/0.0958 | ryu__sf1plus__new__new__3-4.tsv |
| ken | arcade1987 | 4 | old vs old | 282/305 | 3519/3483 | 1.0235/1.0265 | 6.4614/6.6735 | 0.0232/0.0259 | ken__arcade1987__old__old__4.tsv |
| ken | arcade1987 | 1 | new P1 | 469/65 | 2175/1100 | 1.0081/1.0009 | 11.2180/10.0125 | 0.0072/0.0009 | ken__arcade1987__new__old__1.tsv |
| ken | arcade1987 | 1 | new P2 | 469/63 | 2113/1077 | 1.0061/1.0009 | 11.5184/9.9268 | 0.0052/0.0009 | ken__arcade1987__old__new__1.tsv |
| ken | arcade1987 | 2 | new P1 | 476/53 | 1989/945 | 1.0020/1.0717 | 12.2199/10.5840 | 0.0020/0.0707 | ken__arcade1987__new__old__2.tsv |
| ken | arcade1987 | 2 | new P2 | 475/47 | 1992/893 | 1.0020/1.0906 | 12.1640/10.7266 | 0.0020/0.0895 | ken__arcade1987__old__new__2.tsv |
| ken | arcade1987 | 3 | new P1 | 464/61 | 2117/1203 | 1.0019/1.1010 | 11.3920/9.2536 | 0.0009/0.1001 | ken__arcade1987__new__old__3.tsv |
| ken | arcade1987 | 3 | new P2 | 471/61 | 2210/1212 | 1.0014/1.0923 | 11.1916/9.6394 | 0.0014/0.0923 | ken__arcade1987__old__new__3.tsv |
| ken | arcade1987 | 4 | new P1 | 456/89 | 2473/1553 | 1.0008/1.1257 | 10.0707/8.4740 | 0.0004/0.1251 | ken__arcade1987__new__old__4.tsv |
| ken | arcade1987 | 4 | new P2 | 457/91 | 2588/1485 | 1.0000/1.1052 | 9.6269/9.9125 | 0.0000/0.1045 | ken__arcade1987__old__new__4.tsv |
| ken | arcade1987 | 2/1 | higher P1 | 447/131 | 1833/1093 | 1.0049/1.0046 | 13.7009/12.9873 | 0.0027/0.0027 | ken__arcade1987__new__new__2-1.tsv |
| ken | arcade1987 | 1/2 | higher P2 | 456/126 | 1880/1157 | 1.0063/1.0130 | 13.5972/12.8528 | 0.0031/0.0086 | ken__arcade1987__new__new__1-2.tsv |
| ken | arcade1987 | 3/2 | higher P1 | 445/134 | 1743/1025 | 1.0057/1.0039 | 14.4654/14.1657 | 0.0046/0.0010 | ken__arcade1987__new__new__3-2.tsv |
| ken | arcade1987 | 2/3 | higher P2 | 463/104 | 1781/952 | 1.0017/1.0041 | 14.2956/14.3666 | 0.0011/0.0011 | ken__arcade1987__new__new__2-3.tsv |
| ken | arcade1987 | 4/3 | higher P1 | 448/119 | 1694/970 | 1.0012/1.0020 | 14.8960/14.7670 | 0.0006/0.0010 | ken__arcade1987__new__new__4-3.tsv |
| ken | arcade1987 | 3/4 | higher P2 | 443/133 | 1692/1015 | 1.0017/1.0059 | 14.9073/14.6919 | 0.0012/0.0030 | ken__arcade1987__new__new__3-4.tsv |
| ken | sf1plus | 4 | old vs old | 289/315 | 3124/3214 | 1.1923/1.1961 | 7.1940/7.2591 | 0.1884/0.1905 | ken__sf1plus__old__old__4.tsv |
| ken | sf1plus | 1 | new P1 | 480/7 | 3378/1385 | 1.0330/1.0115 | 6.8920/6.7520 | 0.0265/0.0015 | ken__sf1plus__new__old__1.tsv |
| ken | sf1plus | 1 | new P2 | 479/19 | 3431/1455 | 1.0396/1.0164 | 6.9268/6.7877 | 0.0263/0.0027 | ken__sf1plus__old__new__1.tsv |
| ken | sf1plus | 2 | new P1 | 476/24 | 3334/1323 | 1.0651/1.1228 | 7.1156/7.5746 | 0.0533/0.1198 | ken__sf1plus__new__old__2.tsv |
| ken | sf1plus | 2 | new P2 | 478/23 | 3345/1314 | 1.0686/1.1366 | 7.0906/7.5279 | 0.0581/0.1290 | ken__sf1plus__old__new__2.tsv |
| ken | sf1plus | 3 | new P1 | 479/14 | 3313/943 | 1.0783/1.1674 | 7.0849/7.8514 | 0.0617/0.1632 | ken__sf1plus__new__old__3.tsv |
| ken | sf1plus | 3 | new P2 | 480/11 | 3346/1082 | 1.0841/1.1699 | 7.0232/7.6850 | 0.0680/0.1644 | ken__sf1plus__old__new__3.tsv |
| ken | sf1plus | 4 | new P1 | 479/17 | 3659/935 | 1.1181/1.1977 | 6.4540/7.8224 | 0.0877/0.1977 | ken__sf1plus__new__old__4.tsv |
| ken | sf1plus | 4 | new P2 | 480/12 | 3578/874 | 1.1300/1.1986 | 6.5553/7.7700 | 0.0914/0.1917 | ken__sf1plus__old__new__4.tsv |
| ken | sf1plus | 2/1 | higher P1 | 468/66 | 3222/1839 | 1.0745/1.0581 | 7.6554/7.4727 | 0.0619/0.0468 | ken__sf1plus__new__new__2-1.tsv |
| ken | sf1plus | 1/2 | higher P2 | 467/70 | 3154/1789 | 1.0743/1.0549 | 7.8452/7.4749 | 0.0618/0.0455 | ken__sf1plus__new__new__1-2.tsv |
| ken | sf1plus | 3/2 | higher P1 | 458/112 | 3259/2067 | 1.0869/1.1167 | 7.8955/8.0701 | 0.0627/0.0885 | ken__sf1plus__new__new__3-2.tsv |
| ken | sf1plus | 2/3 | higher P2 | 436/120 | 3130/2054 | 1.1003/1.1430 | 7.9500/8.3111 | 0.0676/0.1070 | ken__sf1plus__new__new__2-3.tsv |
| ken | sf1plus | 4/3 | higher P1 | 414/166 | 3123/2252 | 1.1225/1.1824 | 8.1247/8.4945 | 0.0685/0.1055 | ken__sf1plus__new__new__4-3.tsv |
| ken | sf1plus | 3/4 | higher P2 | 422/163 | 3064/2264 | 1.1463/1.1609 | 8.3167/8.4190 | 0.0777/0.0976 | ken__sf1plus__new__new__3-4.tsv |
| retsu | arcade1987 | 4 | old vs old | 284/316 | 6929/6960 | 1.0003/1.0006 | 3.4222/3.5095 | 0.0001/0.0006 | retsu__arcade1987__old__old__4.tsv |
| retsu | arcade1987 | 1 | new P1 | 475/44 | 2841/1338 | 1.0101/1.0088 | 8.5278/8.2308 | 0.0101/0.0029 | retsu__arcade1987__new__old__1.tsv |
| retsu | arcade1987 | 1 | new P2 | 474/32 | 2820/1194 | 1.0070/1.0084 | 8.4452/8.1106 | 0.0070/0.0050 | retsu__arcade1987__old__new__1.tsv |
| retsu | arcade1987 | 2 | new P1 | 480/23 | 2645/1095 | 1.0026/1.0199 | 9.0015/7.5168 | 0.0026/0.0054 | retsu__arcade1987__new__old__2.tsv |
| retsu | arcade1987 | 2 | new P2 | 478/16 | 2647/1058 | 1.0015/1.0113 | 8.8818/7.2143 | 0.0015/0.0029 | retsu__arcade1987__old__new__2.tsv |
| retsu | arcade1987 | 3 | new P1 | 480/12 | 2811/1179 | 1.0000/1.0059 | 8.3544/5.9294 | 0.0000/0.0008 | retsu__arcade1987__new__old__3.tsv |
| retsu | arcade1987 | 3 | new P2 | 480/14 | 2856/1235 | 1.0003/1.0129 | 8.2204/6.0201 | 0.0003/0.0024 | retsu__arcade1987__old__new__3.tsv |
| retsu | arcade1987 | 4 | new P1 | 480/12 | 4264/1292 | 1.0002/1.0062 | 5.5053/5.5166 | 0.0002/0.0000 | retsu__arcade1987__new__old__4.tsv |
| retsu | arcade1987 | 4 | new P2 | 479/13 | 4298/1320 | 1.0000/1.0054 | 5.4515/5.4174 | 0.0000/0.0000 | retsu__arcade1987__old__new__4.tsv |
| retsu | arcade1987 | 2/1 | higher P1 | 375/209 | 2283/1962 | 1.0022/1.0015 | 10.6698/10.4592 | 0.0022/0.0015 | retsu__arcade1987__new__new__2-1.tsv |
| retsu | arcade1987 | 1/2 | higher P2 | 362/230 | 2299/2036 | 1.0042/1.0043 | 10.6036/10.3123 | 0.0042/0.0043 | retsu__arcade1987__new__new__1-2.tsv |
| retsu | arcade1987 | 3/2 | higher P1 | 421/150 | 2206/1581 | 1.0031/1.0019 | 11.3849/11.2774 | 0.0031/0.0019 | retsu__arcade1987__new__new__3-2.tsv |
| retsu | arcade1987 | 2/3 | higher P2 | 428/148 | 2263/1614 | 1.0017/1.0018 | 11.1633/11.2504 | 0.0017/0.0018 | retsu__arcade1987__new__new__2-3.tsv |
| retsu | arcade1987 | 4/3 | higher P1 | 374/231 | 2226/1913 | 1.0000/1.0010 | 11.1162/11.0126 | 0.0000/0.0010 | retsu__arcade1987__new__new__4-3.tsv |
| retsu | arcade1987 | 3/4 | higher P2 | 368/242 | 2156/1855 | 1.0005/1.0016 | 11.5272/11.2688 | 0.0005/0.0016 | retsu__arcade1987__new__new__3-4.tsv |
| retsu | sf1plus | 4 | old vs old | 302/308 | 2729/2724 | 1.2035/1.2177 | 8.1929/8.3968 | 0.1899/0.2024 | retsu__sf1plus__old__old__4.tsv |
| retsu | sf1plus | 1 | new P1 | 480/20 | 2826/1049 | 1.0407/1.0324 | 8.4032/7.7742 | 0.0316/0.0057 | retsu__sf1plus__new__old__1.tsv |
| retsu | sf1plus | 1 | new P2 | 478/18 | 2766/1109 | 1.0457/1.0298 | 8.5246/7.7347 | 0.0362/0.0018 | retsu__sf1plus__old__new__1.tsv |
| retsu | sf1plus | 2 | new P1 | 478/20 | 2487/872 | 1.0751/1.1578 | 9.4606/8.8622 | 0.0719/0.1052 | retsu__sf1plus__new__old__2.tsv |
| retsu | sf1plus | 2 | new P2 | 479/29 | 2522/919 | 1.0775/1.1335 | 9.5049/9.0780 | 0.0720/0.1009 | retsu__sf1plus__old__new__2.tsv |
| retsu | sf1plus | 3 | new P1 | 480/27 | 2832/962 | 1.0615/1.2325 | 8.4600/8.5143 | 0.0586/0.1443 | retsu__sf1plus__new__old__3.tsv |
| retsu | sf1plus | 3 | new P2 | 477/27 | 2788/988 | 1.0609/1.2226 | 8.5349/8.7277 | 0.0571/0.1567 | retsu__sf1plus__old__new__3.tsv |
| retsu | sf1plus | 4 | new P1 | 469/53 | 5071/1179 | 1.0355/1.2473 | 4.7681/9.4792 | 0.0340/0.2030 | retsu__sf1plus__new__old__4.tsv |
| retsu | sf1plus | 4 | new P2 | 471/58 | 5024/1132 | 1.0372/1.2327 | 4.8823/9.2863 | 0.0353/0.1833 | retsu__sf1plus__old__new__4.tsv |
| retsu | sf1plus | 2/1 | higher P1 | 419/178 | 2461/1856 | 1.0633/1.0396 | 10.2420/9.9307 | 0.0625/0.0353 | retsu__sf1plus__new__new__2-1.tsv |
| retsu | sf1plus | 1/2 | higher P2 | 418/171 | 2438/1870 | 1.0627/1.0309 | 10.2937/9.8166 | 0.0603/0.0266 | retsu__sf1plus__new__new__1-2.tsv |
| retsu | sf1plus | 3/2 | higher P1 | 458/100 | 2260/1325 | 1.0462/1.0772 | 11.0761/11.0942 | 0.0462/0.0765 | retsu__sf1plus__new__new__3-2.tsv |
| retsu | sf1plus | 2/3 | higher P2 | 446/107 | 2279/1323 | 1.0345/1.0744 | 10.8417/10.8548 | 0.0345/0.0729 | retsu__sf1plus__new__new__2-3.tsv |
| retsu | sf1plus | 4/3 | higher P1 | 416/174 | 2418/1700 | 1.0562/1.0680 | 10.5357/10.9545 | 0.0554/0.0650 | retsu__sf1plus__new__new__4-3.tsv |
| retsu | sf1plus | 3/4 | higher P2 | 391/220 | 2375/1854 | 1.0598/1.0772 | 10.7735/11.0770 | 0.0590/0.0761 | retsu__sf1plus__new__new__3-4.tsv |
| geki | arcade1987 | 4 | old vs old | 311/289 | 5015/4941 | 1.0008/1.0006 | 4.9702/4.9542 | 0.0000/0.0002 | geki__arcade1987__old__old__4.tsv |
| geki | arcade1987 | 1 | new P1 | 434/136 | 4814/3411 | 1.0108/1.0165 | 5.3283/5.6685 | 0.0108/0.0023 | geki__arcade1987__new__old__1.tsv |
| geki | arcade1987 | 1 | new P2 | 438/121 | 4783/3292 | 1.0115/1.0180 | 5.3144/5.7036 | 0.0115/0.0034 | geki__arcade1987__old__new__1.tsv |
| geki | arcade1987 | 2 | new P1 | 420/149 | 4689/3680 | 1.0071/1.0051 | 5.3675/5.2867 | 0.0071/0.0005 | geki__arcade1987__new__old__2.tsv |
| geki | arcade1987 | 2 | new P2 | 425/141 | 4696/3655 | 1.0074/1.0032 | 5.3561/5.2734 | 0.0074/0.0005 | geki__arcade1987__old__new__2.tsv |
| geki | arcade1987 | 3 | new P1 | 458/89 | 4667/3179 | 1.0094/1.0025 | 5.4056/5.2766 | 0.0094/0.0006 | geki__arcade1987__new__old__3.tsv |
| geki | arcade1987 | 3 | new P2 | 460/85 | 4660/3201 | 1.0072/1.0015 | 5.3964/5.2998 | 0.0072/0.0006 | geki__arcade1987__old__new__3.tsv |
| geki | arcade1987 | 4 | new P1 | 473/61 | 4680/2959 | 1.0043/1.0024 | 5.3035/5.1092 | 0.0043/0.0010 | geki__arcade1987__new__old__4.tsv |
| geki | arcade1987 | 4 | new P2 | 457/78 | 4673/3159 | 1.0032/1.0029 | 5.3215/5.1109 | 0.0032/0.0019 | geki__arcade1987__old__new__4.tsv |
| geki | arcade1987 | 2/1 | higher P1 | 341/247 | 4684/4256 | 1.0120/1.0111 | 5.3667/5.4659 | 0.0116/0.0111 | geki__arcade1987__new__new__2-1.tsv |
| geki | arcade1987 | 1/2 | higher P2 | 328/267 | 4728/4297 | 1.0119/1.0164 | 5.3678/5.5156 | 0.0119/0.0164 | geki__arcade1987__new__new__1-2.tsv |
| geki | arcade1987 | 3/2 | higher P1 | 336/270 | 4613/4389 | 1.0346/1.0132 | 5.4495/5.4447 | 0.0337/0.0129 | geki__arcade1987__new__new__3-2.tsv |
| geki | arcade1987 | 2/3 | higher P2 | 318/283 | 4475/4343 | 1.0333/1.0126 | 5.4678/5.4433 | 0.0331/0.0126 | geki__arcade1987__new__new__2-3.tsv |
| geki | arcade1987 | 4/3 | higher P1 | 395/209 | 4841/3945 | 1.0016/1.0061 | 5.2783/5.4456 | 0.0016/0.0061 | geki__arcade1987__new__new__4-3.tsv |
| geki | arcade1987 | 3/4 | higher P2 | 391/192 | 4664/3746 | 1.0022/1.0054 | 5.3099/5.4471 | 0.0022/0.0054 | geki__arcade1987__new__new__3-4.tsv |
| geki | sf1plus | 4 | old vs old | 321/286 | 4333/4262 | 1.0002/1.0000 | 5.5534/5.5073 | 0.0002/0.0000 | geki__sf1plus__old__old__4.tsv |
| geki | sf1plus | 1 | new P1 | 466/60 | 4583/2631 | 1.0056/1.0037 | 5.3794/5.7059 | 0.0042/0.0026 | geki__sf1plus__new__old__1.tsv |
| geki | sf1plus | 1 | new P2 | 470/67 | 4674/2770 | 1.0027/1.0033 | 5.3765/5.7386 | 0.0020/0.0011 | geki__sf1plus__old__new__1.tsv |
| geki | sf1plus | 2 | new P1 | 462/70 | 4744/2773 | 1.0022/1.0065 | 5.2083/5.4977 | 0.0020/0.0061 | geki__sf1plus__new__old__2.tsv |
| geki | sf1plus | 2 | new P2 | 466/69 | 4800/2664 | 1.0027/1.0042 | 5.2292/5.5856 | 0.0025/0.0034 | geki__sf1plus__old__new__2.tsv |
| geki | sf1plus | 3 | new P1 | 459/83 | 4722/2721 | 1.0026/1.0026 | 5.2435/5.5587 | 0.0026/0.0022 | geki__sf1plus__new__old__3.tsv |
| geki | sf1plus | 3 | new P2 | 461/69 | 4680/2628 | 1.0018/1.0018 | 5.2332/5.5724 | 0.0018/0.0018 | geki__sf1plus__old__new__3.tsv |
| geki | sf1plus | 4 | new P1 | 450/97 | 4836/2741 | 1.0006/1.0000 | 5.1591/5.5590 | 0.0006/0.0000 | geki__sf1plus__new__old__4.tsv |
| geki | sf1plus | 4 | new P2 | 460/82 | 4853/2602 | 1.0019/1.0000 | 5.1492/5.5497 | 0.0019/0.0000 | geki__sf1plus__old__new__4.tsv |
| geki | sf1plus | 2/1 | higher P1 | 271/320 | 4398/4433 | 1.0067/1.0040 | 5.3425/5.5085 | 0.0062/0.0024 | geki__sf1plus__new__new__2-1.tsv |
| geki | sf1plus | 1/2 | higher P2 | 263/344 | 4478/4650 | 1.0074/1.0035 | 5.3684/5.5284 | 0.0068/0.0024 | geki__sf1plus__new__new__1-2.tsv |
| geki | sf1plus | 3/2 | higher P1 | 326/279 | 4558/4358 | 1.0117/1.0025 | 5.4794/5.4792 | 0.0115/0.0023 | geki__sf1plus__new__new__3-2.tsv |
| geki | sf1plus | 2/3 | higher P2 | 322/262 | 4437/4277 | 1.0104/1.0027 | 5.4743/5.4695 | 0.0104/0.0022 | geki__sf1plus__new__new__2-3.tsv |
| geki | sf1plus | 4/3 | higher P1 | 352/222 | 4392/3842 | 1.0053/1.0027 | 5.5122/5.4712 | 0.0053/0.0024 | geki__sf1plus__new__new__4-3.tsv |
| geki | sf1plus | 3/4 | higher P2 | 345/241 | 4510/4000 | 1.0028/1.0025 | 5.5041/5.5072 | 0.0028/0.0022 | geki__sf1plus__new__new__3-4.tsv |
| joe | arcade1987 | 4 | old vs old | 316/285 | 3164/3118 | 1.1043/1.1049 | 7.2308/7.2033 | 0.1040/0.1049 | joe__arcade1987__old__old__4.tsv |
| joe | arcade1987 | 1 | new P1 | 443/122 | 3924/1761 | 1.1568/1.0129 | 6.3873/9.2335 | 0.0394/0.0028 | joe__arcade1987__new__old__1.tsv |
| joe | arcade1987 | 1 | new P2 | 456/105 | 3990/1697 | 1.1509/1.0165 | 6.3721/9.2375 | 0.0372/0.0041 | joe__arcade1987__old__new__1.tsv |
| joe | arcade1987 | 2 | new P1 | 457/78 | 4448/1645 | 1.1147/1.0145 | 5.5193/8.7905 | 0.0173/0.0127 | joe__arcade1987__new__old__2.tsv |
| joe | arcade1987 | 2 | new P2 | 458/88 | 4492/1688 | 1.1177/1.0130 | 5.5186/8.6050 | 0.0184/0.0106 | joe__arcade1987__old__new__2.tsv |
| joe | arcade1987 | 3 | new P1 | 454/101 | 5330/1810 | 1.1190/1.0071 | 4.7394/8.9243 | 0.0034/0.0061 | joe__arcade1987__new__old__3.tsv |
| joe | arcade1987 | 3 | new P2 | 438/127 | 5320/1990 | 1.1279/1.0201 | 4.7971/8.8499 | 0.0042/0.0191 | joe__arcade1987__old__new__3.tsv |
| joe | arcade1987 | 4 | new P1 | 480/20 | 4805/1101 | 1.0903/1.0152 | 4.9595/8.4887 | 0.0063/0.0143 | joe__arcade1987__new__old__4.tsv |
| joe | arcade1987 | 4 | new P2 | 475/29 | 4765/1202 | 1.0975/1.0182 | 5.0226/8.3152 | 0.0032/0.0182 | joe__arcade1987__old__new__4.tsv |
| joe | arcade1987 | 2/1 | higher P1 | 331/253 | 3356/2877 | 1.3663/1.2193 | 7.0311/7.0693 | 0.0256/0.0301 | joe__arcade1987__new__new__2-1.tsv |
| joe | arcade1987 | 1/2 | higher P2 | 355/223 | 3422/2767 | 1.3551/1.2331 | 6.8701/7.1903 | 0.0245/0.0328 | joe__arcade1987__new__new__1-2.tsv |
| joe | arcade1987 | 3/2 | higher P1 | 390/178 | 3783/2745 | 1.3087/1.1039 | 6.2347/6.2081 | 0.0045/0.0202 | joe__arcade1987__new__new__3-2.tsv |
| joe | arcade1987 | 2/3 | higher P2 | 273/294 | 3248/3790 | 1.2313/1.1314 | 6.0855/5.5369 | 0.0059/0.0154 | joe__arcade1987__new__new__2-3.tsv |
| joe | arcade1987 | 4/3 | higher P1 | 427/104 | 5217/2496 | 1.0833/1.0433 | 4.4631/5.1398 | 0.0025/0.0036 | joe__arcade1987__new__new__4-3.tsv |
| joe | arcade1987 | 3/4 | higher P2 | 419/105 | 5075/2526 | 1.0764/1.0481 | 4.5263/5.0766 | 0.0020/0.0068 | joe__arcade1987__new__new__3-4.tsv |
| joe | sf1plus | 4 | old vs old | 308/271 | 2379/2221 | 1.2655/1.2791 | 9.2704/9.4591 | 0.2504/0.2682 | joe__sf1plus__old__old__4.tsv |
| joe | sf1plus | 1 | new P1 | 387/211 | 4215/2374 | 1.0636/1.0172 | 5.9504/8.8844 | 0.0488/0.0042 | joe__sf1plus__new__old__1.tsv |
| joe | sf1plus | 1 | new P2 | 374/211 | 4116/2410 | 1.0608/1.0189 | 5.9669/8.6748 | 0.0431/0.0045 | joe__sf1plus__old__new__1.tsv |
| joe | sf1plus | 2 | new P1 | 396/191 | 4435/2188 | 1.0829/1.0527 | 5.5821/8.8272 | 0.0598/0.0350 | joe__sf1plus__new__old__2.tsv |
| joe | sf1plus | 2 | new P2 | 389/210 | 4425/2300 | 1.0868/1.0509 | 5.6524/8.7780 | 0.0616/0.0386 | joe__sf1plus__old__new__2.tsv |
| joe | sf1plus | 3 | new P1 | 397/184 | 4850/2248 | 1.0677/1.0514 | 5.1125/8.7814 | 0.0530/0.0350 | joe__sf1plus__new__old__3.tsv |
| joe | sf1plus | 3 | new P2 | 387/201 | 4673/2245 | 1.0695/1.0586 | 5.1951/8.8006 | 0.0565/0.0452 | joe__sf1plus__old__new__3.tsv |
| joe | sf1plus | 4 | new P1 | 400/165 | 3888/1824 | 1.0925/1.0989 | 5.7296/8.8867 | 0.0704/0.0912 | joe__sf1plus__new__old__4.tsv |
| joe | sf1plus | 4 | new P2 | 402/172 | 3798/1901 | 1.0833/1.0862 | 5.8202/8.7888 | 0.0670/0.0815 | joe__sf1plus__old__new__4.tsv |
| joe | sf1plus | 2/1 | higher P1 | 309/284 | 4072/3705 | 1.0843/1.1132 | 5.7062/6.2076 | 0.0655/0.0535 | joe__sf1plus__new__new__2-1.tsv |
| joe | sf1plus | 1/2 | higher P2 | 274/316 | 3799/3840 | 1.0931/1.0848 | 5.9053/6.1092 | 0.0687/0.0516 | joe__sf1plus__new__new__1-2.tsv |
| joe | sf1plus | 3/2 | higher P1 | 345/235 | 4553/3400 | 1.0493/1.0973 | 5.0749/6.1242 | 0.0333/0.0695 | joe__sf1plus__new__new__3-2.tsv |
| joe | sf1plus | 2/3 | higher P2 | 368/215 | 4569/3403 | 1.0509/1.0959 | 5.1970/6.0475 | 0.0360/0.0646 | joe__sf1plus__new__new__2-3.tsv |
| joe | sf1plus | 4/3 | higher P1 | 401/163 | 4929/3166 | 1.0376/1.0882 | 4.7465/5.4289 | 0.0301/0.0522 | joe__sf1plus__new__new__4-3.tsv |
| joe | sf1plus | 3/4 | higher P2 | 384/168 | 4776/3222 | 1.0386/1.0764 | 4.8070/5.2563 | 0.0297/0.0478 | joe__sf1plus__new__new__3-4.tsv |
| mike | arcade1987 | 4 | old vs old | 280/310 | 2436/2431 | 1.0097/1.0082 | 8.9031/8.9555 | 0.0097/0.0082 | mike__arcade1987__old__old__4.tsv |
| mike | arcade1987 | 1 | new P1 | 480/27 | 2793/1009 | 1.0135/1.0040 | 8.5730/9.0616 | 0.0131/0.0040 | mike__arcade1987__new__old__1.tsv |
| mike | arcade1987 | 1 | new P2 | 478/26 | 2784/1040 | 1.0114/1.0019 | 8.5581/9.0703 | 0.0114/0.0019 | mike__arcade1987__old__new__1.tsv |
| mike | arcade1987 | 2 | new P1 | 466/61 | 2684/1202 | 1.0045/1.0042 | 9.0425/9.6923 | 0.0045/0.0042 | mike__arcade1987__new__old__2.tsv |
| mike | arcade1987 | 2 | new P2 | 474/43 | 2679/1055 | 1.0063/1.0057 | 8.9548/9.8142 | 0.0063/0.0057 | mike__arcade1987__old__new__2.tsv |
| mike | arcade1987 | 3 | new P1 | 470/51 | 2680/992 | 1.0033/1.0050 | 8.9943/9.7935 | 0.0033/0.0050 | mike__arcade1987__new__old__3.tsv |
| mike | arcade1987 | 3 | new P2 | 474/33 | 2596/849 | 1.0061/1.0059 | 9.0716/9.9022 | 0.0061/0.0059 | mike__arcade1987__old__new__3.tsv |
| mike | arcade1987 | 4 | new P1 | 416/154 | 2480/1591 | 1.0032/1.0031 | 9.3119/9.7342 | 0.0032/0.0031 | mike__arcade1987__new__old__4.tsv |
| mike | arcade1987 | 4 | new P2 | 422/161 | 2543/1588 | 1.0043/1.0000 | 9.2855/9.7955 | 0.0043/0.0000 | mike__arcade1987__old__new__4.tsv |
| mike | arcade1987 | 2/1 | higher P1 | 401/175 | 2778/1979 | 1.0075/1.0045 | 8.8519/8.8104 | 0.0072/0.0045 | mike__arcade1987__new__new__2-1.tsv |
| mike | arcade1987 | 1/2 | higher P2 | 423/134 | 2704/1746 | 1.0092/1.0051 | 8.9570/8.8803 | 0.0085/0.0051 | mike__arcade1987__new__new__1-2.tsv |
| mike | arcade1987 | 3/2 | higher P1 | 383/170 | 2587/1711 | 1.0023/1.0064 | 8.8753/9.1972 | 0.0023/0.0052 | mike__arcade1987__new__new__3-2.tsv |
| mike | arcade1987 | 2/3 | higher P2 | 401/155 | 2615/1609 | 1.0011/1.0068 | 8.9466/9.1710 | 0.0011/0.0068 | mike__arcade1987__new__new__2-3.tsv |
| mike | arcade1987 | 4/3 | higher P1 | 363/184 | 2467/1813 | 1.0037/1.0016 | 8.8749/9.0074 | 0.0037/0.0016 | mike__arcade1987__new__new__4-3.tsv |
| mike | arcade1987 | 3/4 | higher P2 | 371/182 | 2509/1802 | 1.0032/1.0000 | 8.9249/9.0637 | 0.0032/0.0000 | mike__arcade1987__new__new__3-4.tsv |
| mike | sf1plus | 4 | old vs old | 282/312 | 2244/2313 | 1.0527/1.0520 | 9.4815/9.5001 | 0.0479/0.0482 | mike__sf1plus__old__old__4.tsv |
| mike | sf1plus | 1 | new P1 | 480/23 | 2763/1004 | 1.0216/1.0099 | 8.6398/9.5290 | 0.0191/0.0039 | mike__sf1plus__new__old__1.tsv |
| mike | sf1plus | 1 | new P2 | 476/28 | 2726/1040 | 1.0299/1.0086 | 8.7039/9.3828 | 0.0263/0.0048 | mike__sf1plus__old__new__1.tsv |
| mike | sf1plus | 2 | new P1 | 471/43 | 2643/1039 | 1.0102/1.0249 | 9.0617/10.2417 | 0.0098/0.0211 | mike__sf1plus__new__old__2.tsv |
| mike | sf1plus | 2 | new P2 | 478/34 | 2615/950 | 1.0130/1.0263 | 9.1370/10.1624 | 0.0111/0.0242 | mike__sf1plus__old__new__2.tsv |
| mike | sf1plus | 3 | new P1 | 476/31 | 2602/796 | 1.0057/1.0213 | 9.0833/10.2046 | 0.0053/0.0176 | mike__sf1plus__new__old__3.tsv |
| mike | sf1plus | 3 | new P2 | 474/40 | 2606/853 | 1.0073/1.0233 | 9.1522/10.1428 | 0.0073/0.0198 | mike__sf1plus__old__new__3.tsv |
| mike | sf1plus | 4 | new P1 | 418/160 | 2546/1642 | 1.0098/1.0145 | 9.2716/9.7662 | 0.0090/0.0139 | mike__sf1plus__new__old__4.tsv |
| mike | sf1plus | 4 | new P2 | 395/162 | 2364/1714 | 1.0071/1.0134 | 9.3290/9.7923 | 0.0067/0.0134 | mike__sf1plus__old__new__4.tsv |
| mike | sf1plus | 2/1 | higher P1 | 400/163 | 2580/1837 | 1.0135/1.0080 | 9.2291/8.9768 | 0.0127/0.0075 | mike__sf1plus__new__new__2-1.tsv |
| mike | sf1plus | 1/2 | higher P2 | 408/140 | 2575/1778 | 1.0189/1.0083 | 9.1327/8.9126 | 0.0177/0.0072 | mike__sf1plus__new__new__1-2.tsv |
| mike | sf1plus | 3/2 | higher P1 | 363/196 | 2481/1774 | 1.0072/1.0156 | 9.0239/9.5156 | 0.0072/0.0151 | mike__sf1plus__new__new__3-2.tsv |
| mike | sf1plus | 2/3 | higher P2 | 378/183 | 2481/1664 | 1.0120/1.0125 | 9.1900/9.5250 | 0.0117/0.0119 | mike__sf1plus__new__new__2-3.tsv |
| mike | sf1plus | 4/3 | higher P1 | 367/199 | 2482/1797 | 1.0077/1.0145 | 9.1055/9.3440 | 0.0077/0.0133 | mike__sf1plus__new__new__4-3.tsv |
| mike | sf1plus | 3/4 | higher P2 | 376/192 | 2418/1760 | 1.0086/1.0130 | 9.1445/9.3565 | 0.0082/0.0130 | mike__sf1plus__new__new__3-4.tsv |
| lee | arcade1987 | 4 | old vs old | 317/285 | 4251/3891 | 1.0102/1.0067 | 5.7439/5.9579 | 0.0014/0.0005 | lee__arcade1987__old__old__4.tsv |
| lee | arcade1987 | 1 | new P1 | 365/222 | 3177/2450 | 1.0128/1.0089 | 7.6692/8.6251 | 0.0025/0.0016 | lee__arcade1987__new__old__1.tsv |
| lee | arcade1987 | 1 | new P2 | 326/273 | 3082/2613 | 1.0109/1.0114 | 7.7474/8.6032 | 0.0032/0.0034 | lee__arcade1987__old__new__1.tsv |
| lee | arcade1987 | 2 | new P1 | 368/237 | 3305/3002 | 1.0144/1.0121 | 7.5629/7.4705 | 0.0003/0.0010 | lee__arcade1987__new__old__2.tsv |
| lee | arcade1987 | 2 | new P2 | 304/302 | 3100/3254 | 1.0142/1.0121 | 7.7467/7.2354 | 0.0000/0.0022 | lee__arcade1987__old__new__2.tsv |
| lee | arcade1987 | 3 | new P1 | 357/252 | 3839/3207 | 1.0123/1.0103 | 6.6197/7.1369 | 0.0000/0.0009 | lee__arcade1987__new__old__3.tsv |
| lee | arcade1987 | 3 | new P2 | 270/326 | 3361/3607 | 1.0187/1.0162 | 6.7581/6.7364 | 0.0000/0.0014 | lee__arcade1987__old__new__3.tsv |
| lee | arcade1987 | 4 | new P1 | 416/174 | 4893/3272 | 1.0052/1.0058 | 5.2917/6.3147 | 0.0000/0.0000 | lee__arcade1987__new__old__4.tsv |
| lee | arcade1987 | 4 | new P2 | 322/282 | 4544/3885 | 1.0059/1.0101 | 5.3644/6.0886 | 0.0000/0.0000 | lee__arcade1987__old__new__4.tsv |
| lee | arcade1987 | 2/1 | higher P1 | 369/233 | 2954/2624 | 1.0358/1.0222 | 8.4959/8.1777 | 0.0000/0.0019 | lee__arcade1987__new__new__2-1.tsv |
| lee | arcade1987 | 1/2 | higher P2 | 327/266 | 2770/2898 | 1.0244/1.0288 | 8.5493/7.8934 | 0.0000/0.0027 | lee__arcade1987__new__new__1-2.tsv |
| lee | arcade1987 | 3/2 | higher P1 | 369/229 | 3168/2718 | 1.0214/1.0358 | 7.9726/7.8625 | 0.0000/0.0008 | lee__arcade1987__new__new__3-2.tsv |
| lee | arcade1987 | 2/3 | higher P2 | 308/286 | 2802/2977 | 1.0235/1.0539 | 8.2985/7.6602 | 0.0000/0.0010 | lee__arcade1987__new__new__2-3.tsv |
| lee | arcade1987 | 4/3 | higher P1 | 370/236 | 3585/3115 | 1.0198/1.0259 | 7.0650/7.2517 | 0.0000/0.0003 | lee__arcade1987__new__new__4-3.tsv |
| lee | arcade1987 | 3/4 | higher P2 | 296/302 | 3079/3657 | 1.0218/1.0239 | 7.6143/6.5240 | 0.0003/0.0008 | lee__arcade1987__new__new__3-4.tsv |
| lee | sf1plus | 4 | old vs old | 324/276 | 2816/2630 | 1.1619/1.1611 | 7.9970/8.1957 | 0.1569/0.1596 | lee__sf1plus__old__old__4.tsv |
| lee | sf1plus | 1 | new P1 | 408/168 | 2990/2008 | 1.0269/1.0105 | 8.2434/8.8685 | 0.0225/0.0050 | lee__sf1plus__new__old__1.tsv |
| lee | sf1plus | 1 | new P2 | 393/202 | 2989/2217 | 1.0261/1.0135 | 8.2871/8.8558 | 0.0221/0.0063 | lee__sf1plus__old__new__1.tsv |
| lee | sf1plus | 2 | new P1 | 416/158 | 2978/2054 | 1.0753/1.0450 | 8.2931/8.4677 | 0.0736/0.0411 | lee__sf1plus__new__old__2.tsv |
| lee | sf1plus | 2 | new P2 | 395/202 | 2981/2391 | 1.0552/1.0742 | 8.3532/8.4927 | 0.0533/0.0691 | lee__sf1plus__old__new__2.tsv |
| lee | sf1plus | 3 | new P1 | 420/172 | 3139/2216 | 1.0816/1.0571 | 8.1146/8.5405 | 0.0787/0.0562 | lee__sf1plus__new__old__3.tsv |
| lee | sf1plus | 3 | new P2 | 383/190 | 2939/2225 | 1.0756/1.0574 | 8.2308/8.5054 | 0.0756/0.0569 | lee__sf1plus__old__new__3.tsv |
| lee | sf1plus | 4 | new P1 | 441/114 | 3286/1766 | 1.0967/1.0693 | 7.5263/8.5675 | 0.0967/0.0688 | lee__sf1plus__new__old__4.tsv |
| lee | sf1plus | 4 | new P2 | 439/124 | 3290/1905 | 1.0890/1.0783 | 7.5103/8.6919 | 0.0890/0.0762 | lee__sf1plus__old__new__4.tsv |
| lee | sf1plus | 2/1 | higher P1 | 382/200 | 2684/2324 | 1.0927/1.0558 | 9.0888/8.3380 | 0.0901/0.0541 | lee__sf1plus__new__new__2-1.tsv |
| lee | sf1plus | 1/2 | higher P2 | 361/217 | 2621/2519 | 1.0863/1.0669 | 9.0798/8.0856 | 0.0837/0.0658 | lee__sf1plus__new__new__1-2.tsv |
| lee | sf1plus | 3/2 | higher P1 | 369/221 | 2649/2208 | 1.0993/1.1183 | 9.2690/8.8126 | 0.0982/0.1174 | lee__sf1plus__new__new__3-2.tsv |
| lee | sf1plus | 2/3 | higher P2 | 347/246 | 2572/2489 | 1.1017/1.1186 | 9.4110/8.7056 | 0.0986/0.1169 | lee__sf1plus__new__new__2-3.tsv |
| lee | sf1plus | 4/3 | higher P1 | 341/270 | 2686/2471 | 1.1234/1.1463 | 8.8946/8.8060 | 0.1227/0.1459 | lee__sf1plus__new__new__4-3.tsv |
| lee | sf1plus | 3/4 | higher P2 | 281/317 | 2366/2567 | 1.1213/1.1477 | 9.3671/8.9046 | 0.1213/0.1473 | lee__sf1plus__new__new__3-4.tsv |
| gen | arcade1987 | 4 | old vs old | 305/284 | 3548/3523 | 1.0348/1.0298 | 6.7349/6.6131 | 0.0000/0.0000 | gen__arcade1987__old__old__4.tsv |
| gen | arcade1987 | 1 | new P1 | 442/116 | 3206/2226 | 1.0460/1.0099 | 7.7979/7.1940 | 0.0019/0.0000 | gen__arcade1987__new__old__1.tsv |
| gen | arcade1987 | 1 | new P2 | 431/146 | 3285/2406 | 1.0388/1.0166 | 7.7371/7.3401 | 0.0016/0.0000 | gen__arcade1987__old__new__1.tsv |
| gen | arcade1987 | 2 | new P1 | 434/140 | 2857/2726 | 1.0836/1.0128 | 8.8021/6.1035 | 0.0007/0.0000 | gen__arcade1987__new__old__2.tsv |
| gen | arcade1987 | 2 | new P2 | 408/177 | 2889/2971 | 1.0621/1.0199 | 8.5946/6.0484 | 0.0007/0.0003 | gen__arcade1987__old__new__2.tsv |
| gen | arcade1987 | 3 | new P1 | 378/213 | 2517/3579 | 1.1082/1.0070 | 9.5899/5.3050 | 0.0008/0.0000 | gen__arcade1987__new__old__3.tsv |
| gen | arcade1987 | 3 | new P2 | 357/249 | 2619/3910 | 1.0618/1.0074 | 9.2437/5.2670 | 0.0008/0.0000 | gen__arcade1987__old__new__3.tsv |
| gen | arcade1987 | 4 | new P1 | 362/223 | 2682/3962 | 1.1095/1.0050 | 8.8565/4.8815 | 0.0000/0.0000 | gen__arcade1987__new__old__4.tsv |
| gen | arcade1987 | 4 | new P2 | 353/248 | 2904/4252 | 1.0535/1.0052 | 8.3968/4.8909 | 0.0000/0.0002 | gen__arcade1987__old__new__4.tsv |
| gen | arcade1987 | 2/1 | higher P1 | 381/216 | 3028/2910 | 1.0396/1.0268 | 8.0177/6.7772 | 0.0006/0.0000 | gen__arcade1987__new__new__2-1.tsv |
| gen | arcade1987 | 1/2 | higher P2 | 338/268 | 3032/3175 | 1.0342/1.0322 | 7.8536/6.8778 | 0.0000/0.0000 | gen__arcade1987__new__new__1-2.tsv |
| gen | arcade1987 | 3/2 | higher P1 | 219/375 | 2565/3647 | 1.0370/1.0168 | 7.4336/6.7062 | 0.0004/0.0006 | gen__arcade1987__new__new__3-2.tsv |
| gen | arcade1987 | 2/3 | higher P2 | 190/383 | 2398/3627 | 1.0349/1.0133 | 7.4039/6.6943 | 0.0000/0.0003 | gen__arcade1987__new__new__2-3.tsv |
| gen | arcade1987 | 4/3 | higher P1 | 417/163 | 3080/2451 | 1.0485/1.0294 | 8.1781/6.5743 | 0.0000/0.0000 | gen__arcade1987__new__new__4-3.tsv |
| gen | arcade1987 | 3/4 | higher P2 | 428/148 | 3033/2524 | 1.0329/1.0267 | 8.3175/6.5438 | 0.0000/0.0000 | gen__arcade1987__new__new__3-4.tsv |
| gen | sf1plus | 4 | old vs old | 366/247 | 3613/2870 | 1.1018/1.1357 | 6.8194/7.3636 | 0.0880/0.1252 | gen__sf1plus__old__old__4.tsv |
| gen | sf1plus | 1 | new P1 | 459/93 | 3602/2207 | 1.0223/1.0067 | 7.0291/7.0468 | 0.0159/0.0036 | gen__sf1plus__new__old__1.tsv |
| gen | sf1plus | 1 | new P2 | 429/140 | 3559/2362 | 1.0187/1.0068 | 7.0731/7.0516 | 0.0160/0.0021 | gen__sf1plus__old__new__1.tsv |
| gen | sf1plus | 2 | new P1 | 419/146 | 3091/2118 | 1.0500/1.0972 | 7.9135/7.8584 | 0.0461/0.0939 | gen__sf1plus__new__old__2.tsv |
| gen | sf1plus | 2 | new P2 | 427/157 | 3146/2300 | 1.0493/1.0759 | 7.9996/7.6806 | 0.0487/0.0741 | gen__sf1plus__old__new__2.tsv |
| gen | sf1plus | 3 | new P1 | 461/77 | 2918/1565 | 1.0552/1.1054 | 8.3905/7.6663 | 0.0539/0.1028 | gen__sf1plus__new__old__3.tsv |
| gen | sf1plus | 3 | new P2 | 444/107 | 2852/1794 | 1.0547/1.1122 | 8.5538/7.5990 | 0.0543/0.1116 | gen__sf1plus__old__new__3.tsv |
| gen | sf1plus | 4 | new P1 | 437/118 | 3004/1924 | 1.0545/1.1516 | 8.1903/7.5699 | 0.0539/0.1485 | gen__sf1plus__new__old__4.tsv |
| gen | sf1plus | 4 | new P2 | 436/147 | 3073/2173 | 1.0541/1.1683 | 8.2851/7.6094 | 0.0537/0.1628 | gen__sf1plus__old__new__4.tsv |
| gen | sf1plus | 2/1 | higher P1 | 385/200 | 3088/2738 | 1.0769/1.0560 | 7.8974/6.9371 | 0.0766/0.0537 | gen__sf1plus__new__new__2-1.tsv |
| gen | sf1plus | 1/2 | higher P2 | 376/211 | 3078/2803 | 1.0793/1.0496 | 7.8816/6.9130 | 0.0777/0.0485 | gen__sf1plus__new__new__1-2.tsv |
| gen | sf1plus | 3/2 | higher P1 | 281/319 | 2638/2843 | 1.1202/1.0934 | 8.0209/8.0144 | 0.1202/0.0916 | gen__sf1plus__new__new__3-2.tsv |
| gen | sf1plus | 2/3 | higher P2 | 266/346 | 2530/3032 | 1.1202/1.0963 | 8.1936/7.9986 | 0.1198/0.0953 | gen__sf1plus__new__new__2-3.tsv |
| gen | sf1plus | 4/3 | higher P1 | 427/152 | 2885/2065 | 1.1063/1.1236 | 8.6664/7.7737 | 0.1060/0.1231 | gen__sf1plus__new__new__4-3.tsv |
| gen | sf1plus | 3/4 | higher P2 | 382/206 | 2778/2253 | 1.1027/1.1416 | 8.8055/7.9829 | 0.1023/0.1411 | gen__sf1plus__new__new__3-4.tsv |
| birdie | arcade1987 | 4 | old vs old | 311/295 | 4542/4451 | 1.0157/1.0132 | 5.3239/5.3213 | 0.0116/0.0072 | birdie__arcade1987__old__old__4.tsv |
| birdie | arcade1987 | 1 | new P1 | 432/126 | 3167/2305 | 1.0035/1.0199 | 7.9302/7.3238 | 0.0020/0.0008 | birdie__arcade1987__new__old__1.tsv |
| birdie | arcade1987 | 1 | new P2 | 437/114 | 3170/2250 | 1.0044/1.0191 | 7.8482/7.4727 | 0.0025/0.0014 | birdie__arcade1987__old__new__1.tsv |
| birdie | arcade1987 | 2 | new P1 | 407/180 | 3241/2811 | 1.0042/1.0143 | 7.9158/6.7293 | 0.0003/0.0007 | birdie__arcade1987__new__old__2.tsv |
| birdie | arcade1987 | 2 | new P2 | 404/187 | 3228/2984 | 1.0028/1.0102 | 7.8741/6.6010 | 0.0006/0.0000 | birdie__arcade1987__old__new__2.tsv |
| birdie | arcade1987 | 3 | new P1 | 357/242 | 3312/3330 | 1.0094/1.0118 | 7.6356/6.4063 | 0.0009/0.0006 | birdie__arcade1987__new__old__3.tsv |
| birdie | arcade1987 | 3 | new P2 | 347/244 | 3150/3278 | 1.0121/1.0142 | 7.7976/6.4287 | 0.0000/0.0006 | birdie__arcade1987__old__new__3.tsv |
| birdie | arcade1987 | 4 | new P1 | 410/176 | 3626/2971 | 1.0008/1.0088 | 7.0293/6.1161 | 0.0000/0.0003 | birdie__arcade1987__new__old__4.tsv |
| birdie | arcade1987 | 4 | new P2 | 368/198 | 3382/3139 | 1.0006/1.0061 | 7.2353/5.9270 | 0.0000/0.0000 | birdie__arcade1987__old__new__4.tsv |
| birdie | arcade1987 | 2/1 | higher P1 | 345/261 | 3134/3118 | 1.0127/1.0093 | 7.8160/7.2342 | 0.0026/0.0019 | birdie__arcade1987__new__new__2-1.tsv |
| birdie | arcade1987 | 1/2 | higher P2 | 344/271 | 3109/3250 | 1.0115/1.0092 | 7.9035/7.2034 | 0.0009/0.0034 | birdie__arcade1987__new__new__1-2.tsv |
| birdie | arcade1987 | 3/2 | higher P1 | 293/305 | 2930/3116 | 1.0095/1.0083 | 7.8602/7.3482 | 0.0014/0.0006 | birdie__arcade1987__new__new__3-2.tsv |
| birdie | arcade1987 | 2/3 | higher P2 | 250/355 | 2870/3477 | 1.0086/1.0060 | 7.7866/7.0975 | 0.0003/0.0003 | birdie__arcade1987__new__new__2-3.tsv |
| birdie | arcade1987 | 4/3 | higher P1 | 230/377 | 2828/3590 | 1.0050/1.0069 | 7.3390/6.8378 | 0.0003/0.0000 | birdie__arcade1987__new__new__4-3.tsv |
| birdie | arcade1987 | 3/4 | higher P2 | 224/387 | 2838/3675 | 1.0095/1.0033 | 7.3330/6.7163 | 0.0000/0.0003 | birdie__arcade1987__new__new__3-4.tsv |
| birdie | sf1plus | 4 | old vs old | 317/285 | 2710/2580 | 1.0196/1.0212 | 8.3190/8.4814 | 0.0167/0.0189 | birdie__sf1plus__old__old__4.tsv |
| birdie | sf1plus | 1 | new P1 | 461/67 | 3051/1731 | 1.0413/1.0133 | 8.0302/7.6309 | 0.0328/0.0080 | birdie__sf1plus__new__old__1.tsv |
| birdie | sf1plus | 1 | new P2 | 469/68 | 3156/1819 | 1.0347/1.0214 | 7.8459/7.6302 | 0.0301/0.0120 | birdie__sf1plus__old__new__1.tsv |
| birdie | sf1plus | 2 | new P1 | 468/83 | 2892/1675 | 1.0837/1.0428 | 8.6605/8.3372 | 0.0827/0.0403 | birdie__sf1plus__new__old__2.tsv |
| birdie | sf1plus | 2 | new P2 | 462/83 | 2849/1760 | 1.0894/1.0366 | 8.7110/8.2756 | 0.0866/0.0349 | birdie__sf1plus__old__new__2.tsv |
| birdie | sf1plus | 3 | new P1 | 459/85 | 2806/1718 | 1.1294/1.0273 | 8.9056/8.4794 | 0.1273/0.0261 | birdie__sf1plus__new__old__3.tsv |
| birdie | sf1plus | 3 | new P2 | 444/97 | 2767/1795 | 1.1318/1.0321 | 8.9511/8.3431 | 0.1314/0.0293 | birdie__sf1plus__old__new__3.tsv |
| birdie | sf1plus | 4 | new P1 | 464/73 | 2912/1489 | 1.1620/1.0153 | 8.5425/8.7853 | 0.1613/0.0120 | birdie__sf1plus__new__old__4.tsv |
| birdie | sf1plus | 4 | new P2 | 466/55 | 2845/1334 | 1.1625/1.0134 | 8.5182/8.8168 | 0.1618/0.0119 | birdie__sf1plus__old__new__4.tsv |
| birdie | sf1plus | 2/1 | higher P1 | 398/195 | 2895/2539 | 1.1227/1.1075 | 8.8083/8.0388 | 0.1183/0.1040 | birdie__sf1plus__new__new__2-1.tsv |
| birdie | sf1plus | 1/2 | higher P2 | 367/240 | 2823/2687 | 1.1262/1.1032 | 8.8396/8.0040 | 0.1255/0.0991 | birdie__sf1plus__new__new__1-2.tsv |
| birdie | sf1plus | 3/2 | higher P1 | 419/169 | 2739/2210 | 1.1994/1.1763 | 9.3391/8.7059 | 0.1972/0.1763 | birdie__sf1plus__new__new__3-2.tsv |
| birdie | sf1plus | 2/3 | higher P2 | 370/213 | 2660/2314 | 1.1880/1.1662 | 9.2838/8.5992 | 0.1854/0.1645 | birdie__sf1plus__new__new__2-3.tsv |
| birdie | sf1plus | 4/3 | higher P1 | 394/200 | 2556/1981 | 1.2550/1.2517 | 9.7026/9.3468 | 0.2526/0.2501 | birdie__sf1plus__new__new__4-3.tsv |
| birdie | sf1plus | 3/4 | higher P2 | 405/170 | 2522/1888 | 1.2672/1.2364 | 9.8100/9.2334 | 0.2672/0.2348 | birdie__sf1plus__new__new__3-4.tsv |
| eagle | arcade1987 | 4 | old vs old | 296/297 | 3952/3969 | 1.0391/1.0360 | 5.8251/5.8704 | 0.0010/0.0005 | eagle__arcade1987__old__old__4.tsv |
| eagle | arcade1987 | 1 | new P1 | 467/50 | 3109/1365 | 1.0343/1.0366 | 7.7282/8.0367 | 0.0070/0.0000 | eagle__arcade1987__new__old__1.tsv |
| eagle | arcade1987 | 1 | new P2 | 469/47 | 3159/1439 | 1.0249/1.0317 | 7.6108/7.9994 | 0.0050/0.0000 | eagle__arcade1987__old__new__1.tsv |
| eagle | arcade1987 | 2 | new P1 | 477/20 | 3017/952 | 1.0213/1.0241 | 7.8144/8.0072 | 0.0013/0.0010 | eagle__arcade1987__new__old__2.tsv |
| eagle | arcade1987 | 2 | new P2 | 470/31 | 3011/1033 | 1.0221/1.0356 | 7.8200/8.0368 | 0.0036/0.0000 | eagle__arcade1987__old__new__2.tsv |
| eagle | arcade1987 | 3 | new P1 | 480/1 | 2922/520 | 1.0151/1.0616 | 7.8930/7.7365 | 0.0007/0.0000 | eagle__arcade1987__new__old__3.tsv |
| eagle | arcade1987 | 3 | new P2 | 479/7 | 2980/629 | 1.0106/1.0632 | 7.7842/7.7373 | 0.0003/0.0000 | eagle__arcade1987__old__new__3.tsv |
| eagle | arcade1987 | 4 | new P1 | 479/11 | 3197/792 | 1.0065/1.0556 | 7.3284/7.2508 | 0.0006/0.0013 | eagle__arcade1987__new__old__4.tsv |
| eagle | arcade1987 | 4 | new P2 | 480/7 | 3192/708 | 1.0047/1.0367 | 7.3015/7.2694 | 0.0006/0.0000 | eagle__arcade1987__old__new__4.tsv |
| eagle | arcade1987 | 2/1 | higher P1 | 410/154 | 2981/2060 | 1.0109/1.0098 | 8.1343/8.2017 | 0.0010/0.0010 | eagle__arcade1987__new__new__2-1.tsv |
| eagle | arcade1987 | 1/2 | higher P2 | 434/129 | 3018/1860 | 1.0111/1.0118 | 8.1276/8.1790 | 0.0003/0.0005 | eagle__arcade1987__new__new__1-2.tsv |
| eagle | arcade1987 | 3/2 | higher P1 | 366/219 | 2861/2357 | 1.0142/1.0033 | 8.2529/8.2444 | 0.0000/0.0000 | eagle__arcade1987__new__new__3-2.tsv |
| eagle | arcade1987 | 2/3 | higher P2 | 359/228 | 2843/2518 | 1.0102/1.0051 | 8.2254/8.2141 | 0.0007/0.0004 | eagle__arcade1987__new__new__2-3.tsv |
| eagle | arcade1987 | 4/3 | higher P1 | 334/269 | 2956/2625 | 1.0011/1.0015 | 8.2943/8.3555 | 0.0000/0.0000 | eagle__arcade1987__new__new__4-3.tsv |
| eagle | arcade1987 | 3/4 | higher P2 | 379/231 | 3024/2259 | 1.0010/1.0035 | 8.2119/8.3565 | 0.0000/0.0000 | eagle__arcade1987__new__new__3-4.tsv |
| eagle | sf1plus | 4 | old vs old | 303/296 | 2428/2451 | 1.2161/1.2299 | 9.1998/9.2794 | 0.2111/0.2213 | eagle__sf1plus__old__old__4.tsv |
| eagle | sf1plus | 1 | new P1 | 468/57 | 3165/1357 | 1.0481/1.0302 | 7.7303/8.6805 | 0.0358/0.0109 | eagle__sf1plus__new__old__1.tsv |
| eagle | sf1plus | 1 | new P2 | 475/43 | 3145/1318 | 1.0499/1.0136 | 7.6767/8.5184 | 0.0322/0.0015 | eagle__sf1plus__old__new__1.tsv |
| eagle | sf1plus | 2 | new P1 | 476/28 | 2999/916 | 1.0513/1.0862 | 7.9325/9.3829 | 0.0471/0.0819 | eagle__sf1plus__new__old__2.tsv |
| eagle | sf1plus | 2 | new P2 | 476/29 | 3011/963 | 1.0510/1.0777 | 7.9177/9.3115 | 0.0494/0.0715 | eagle__sf1plus__old__new__2.tsv |
| eagle | sf1plus | 3 | new P1 | 479/3 | 2933/538 | 1.0386/1.1188 | 7.8772/9.1318 | 0.0386/0.1169 | eagle__sf1plus__new__old__3.tsv |
| eagle | sf1plus | 3 | new P2 | 479/7 | 2962/621 | 1.0408/1.1125 | 7.8513/9.4509 | 0.0408/0.1077 | eagle__sf1plus__old__new__3.tsv |
| eagle | sf1plus | 4 | new P1 | 478/22 | 3087/797 | 1.0485/1.2117 | 7.7062/9.4252 | 0.0475/0.2117 | eagle__sf1plus__new__old__4.tsv |
| eagle | sf1plus | 4 | new P2 | 480/30 | 3137/926 | 1.0586/1.2027 | 7.6424/9.1391 | 0.0567/0.2005 | eagle__sf1plus__old__new__4.tsv |
| eagle | sf1plus | 2/1 | higher P1 | 422/124 | 2907/1757 | 1.0431/1.0327 | 8.2174/8.0352 | 0.0389/0.0198 | eagle__sf1plus__new__new__2-1.tsv |
| eagle | sf1plus | 1/2 | higher P2 | 429/104 | 2910/1692 | 1.0388/1.0306 | 8.1363/8.0676 | 0.0344/0.0205 | eagle__sf1plus__new__new__1-2.tsv |
| eagle | sf1plus | 3/2 | higher P1 | 436/91 | 2882/1323 | 1.0202/1.0540 | 7.9878/8.5463 | 0.0195/0.0502 | eagle__sf1plus__new__new__3-2.tsv |
| eagle | sf1plus | 2/3 | higher P2 | 426/97 | 2849/1391 | 1.0268/1.0588 | 8.0072/8.4143 | 0.0261/0.0545 | eagle__sf1plus__new__new__2-3.tsv |
| eagle | sf1plus | 4/3 | higher P1 | 465/42 | 2941/1009 | 1.0047/1.0375 | 8.0257/8.7811 | 0.0047/0.0375 | eagle__sf1plus__new__new__4-3.tsv |
| eagle | sf1plus | 3/4 | higher P2 | 464/46 | 2923/1019 | 1.0149/1.0234 | 8.0778/8.5267 | 0.0149/0.0234 | eagle__sf1plus__new__new__3-4.tsv |
| adon | arcade1987 | 4 | old vs old | 296/306 | 3521/3611 | 1.0040/1.0025 | 7.0165/6.8990 | 0.0003/0.0000 | adon__arcade1987__old__old__4.tsv |
| adon | arcade1987 | 1 | new P1 | 363/234 | 3365/2565 | 1.0077/1.0187 | 7.3388/8.4972 | 0.0024/0.0004 | adon__arcade1987__new__old__1.tsv |
| adon | arcade1987 | 1 | new P2 | 368/233 | 3414/2498 | 1.0135/1.0236 | 7.3242/8.4065 | 0.0053/0.0008 | adon__arcade1987__old__new__1.tsv |
| adon | arcade1987 | 2 | new P1 | 416/161 | 3409/2248 | 1.0041/1.0057 | 7.4035/8.0977 | 0.0003/0.0000 | adon__arcade1987__new__old__2.tsv |
| adon | arcade1987 | 2 | new P2 | 415/147 | 3288/2229 | 1.0043/1.0115 | 7.5141/7.9925 | 0.0003/0.0000 | adon__arcade1987__old__new__2.tsv |
| adon | arcade1987 | 3 | new P1 | 442/110 | 3417/1903 | 1.0058/1.0069 | 7.3017/7.9583 | 0.0000/0.0000 | adon__arcade1987__new__old__3.tsv |
| adon | arcade1987 | 3 | new P2 | 436/134 | 3531/2199 | 1.0042/1.0059 | 7.2161/7.6767 | 0.0000/0.0000 | adon__arcade1987__old__new__3.tsv |
| adon | arcade1987 | 4 | new P1 | 440/127 | 3815/2476 | 1.0019/1.0020 | 6.8356/7.2482 | 0.0000/0.0000 | adon__arcade1987__new__old__4.tsv |
| adon | arcade1987 | 4 | new P2 | 434/135 | 3819/2586 | 1.0032/1.0039 | 6.7895/7.3395 | 0.0000/0.0004 | adon__arcade1987__old__new__4.tsv |
| adon | arcade1987 | 2/1 | higher P1 | 422/164 | 3314/2380 | 1.0027/1.0143 | 7.7863/7.7913 | 0.0006/0.0021 | adon__arcade1987__new__new__2-1.tsv |
| adon | arcade1987 | 1/2 | higher P2 | 404/173 | 3183/2434 | 1.0022/1.0119 | 7.8017/7.7379 | 0.0010/0.0021 | adon__arcade1987__new__new__1-2.tsv |
| adon | arcade1987 | 3/2 | higher P1 | 441/116 | 3247/1978 | 1.0016/1.0065 | 7.7571/8.4217 | 0.0000/0.0005 | adon__arcade1987__new__new__3-2.tsv |
| adon | arcade1987 | 2/3 | higher P2 | 417/145 | 3184/2188 | 1.0025/1.0032 | 7.8137/8.0913 | 0.0006/0.0009 | adon__arcade1987__new__new__2-3.tsv |
| adon | arcade1987 | 4/3 | higher P1 | 431/124 | 3154/2067 | 1.0010/1.0014 | 7.8407/8.2001 | 0.0000/0.0000 | adon__arcade1987__new__new__4-3.tsv |
| adon | arcade1987 | 3/4 | higher P2 | 426/123 | 3108/2077 | 1.0010/1.0038 | 7.8344/8.1971 | 0.0010/0.0010 | adon__arcade1987__new__new__3-4.tsv |
| adon | sf1plus | 4 | old vs old | 293/300 | 2424/2450 | 1.2054/1.1873 | 9.4555/9.3376 | 0.2054/0.1873 | adon__sf1plus__old__old__4.tsv |
| adon | sf1plus | 1 | new P1 | 382/209 | 3294/2493 | 1.0246/1.0056 | 7.5648/8.2495 | 0.0246/0.0056 | adon__sf1plus__new__old__1.tsv |
| adon | sf1plus | 1 | new P2 | 396/191 | 3215/2394 | 1.0305/1.0083 | 7.7404/8.0716 | 0.0305/0.0083 | adon__sf1plus__old__new__1.tsv |
| adon | sf1plus | 2 | new P1 | 427/135 | 3006/1877 | 1.0805/1.0998 | 8.2751/9.1781 | 0.0781/0.0998 | adon__sf1plus__new__old__2.tsv |
| adon | sf1plus | 2 | new P2 | 420/155 | 2998/1970 | 1.0841/1.0857 | 8.3950/9.0640 | 0.0822/0.0857 | adon__sf1plus__old__new__2.tsv |
| adon | sf1plus | 3 | new P1 | 448/86 | 2941/1448 | 1.0889/1.1267 | 8.2679/9.5694 | 0.0889/0.1267 | adon__sf1plus__new__old__3.tsv |
| adon | sf1plus | 3 | new P2 | 435/110 | 2965/1599 | 1.0830/1.1137 | 8.2573/9.3134 | 0.0817/0.1137 | adon__sf1plus__old__new__3.tsv |
| adon | sf1plus | 4 | new P1 | 440/120 | 3089/1706 | 1.0904/1.1612 | 8.1833/9.4664 | 0.0900/0.1612 | adon__sf1plus__new__old__4.tsv |
| adon | sf1plus | 4 | new P2 | 449/106 | 3042/1750 | 1.0986/1.1308 | 8.2111/9.2148 | 0.0982/0.1308 | adon__sf1plus__old__new__4.tsv |
| adon | sf1plus | 2/1 | higher P1 | 449/100 | 3047/1934 | 1.0557/1.0148 | 8.1691/7.9625 | 0.0543/0.0148 | adon__sf1plus__new__new__2-1.tsv |
| adon | sf1plus | 1/2 | higher P2 | 435/137 | 3079/2244 | 1.0560/1.0234 | 8.2552/7.9159 | 0.0554/0.0234 | adon__sf1plus__new__new__1-2.tsv |
| adon | sf1plus | 3/2 | higher P1 | 428/127 | 3024/1943 | 1.0370/1.0460 | 8.1765/8.5812 | 0.0363/0.0455 | adon__sf1plus__new__new__3-2.tsv |
| adon | sf1plus | 2/3 | higher P2 | 392/188 | 2999/2346 | 1.0339/1.0442 | 8.2098/8.5431 | 0.0332/0.0433 | adon__sf1plus__new__new__2-3.tsv |
| adon | sf1plus | 4/3 | higher P1 | 432/125 | 3005/2056 | 1.0381/1.0310 | 8.2066/8.5119 | 0.0377/0.0310 | adon__sf1plus__new__new__4-3.tsv |
| adon | sf1plus | 3/4 | higher P2 | 419/156 | 3018/2215 | 1.0337/1.0374 | 8.2499/8.5370 | 0.0324/0.0374 | adon__sf1plus__new__new__3-4.tsv |
| sagat | arcade1987 | 4 | old vs old | 337/275 | 2627/2426 | 1.0008/1.0017 | 8.9746/9.4778 | 0.0004/0.0008 | sagat__arcade1987__old__old__4.tsv |
| sagat | arcade1987 | 1 | new P1 | 468/83 | 1844/837 | 1.0270/1.0142 | 13.2758/13.7240 | 0.0016/0.0012 | sagat__arcade1987__new__old__1.tsv |
| sagat | arcade1987 | 1 | new P2 | 452/105 | 1859/937 | 1.0321/1.0063 | 13.0999/12.9405 | 0.0043/0.0021 | sagat__arcade1987__old__new__1.tsv |
| sagat | arcade1987 | 2 | new P1 | 442/123 | 1720/944 | 1.0235/1.0042 | 14.2278/13.6991 | 0.0011/0.0021 | sagat__arcade1987__new__old__2.tsv |
| sagat | arcade1987 | 2 | new P2 | 442/152 | 1842/1017 | 1.0269/1.0088 | 13.7435/13.7164 | 0.0011/0.0049 | sagat__arcade1987__old__new__2.tsv |
| sagat | arcade1987 | 3 | new P1 | 446/109 | 1776/1063 | 1.0235/1.0075 | 13.9373/11.1921 | 0.0000/0.0019 | sagat__arcade1987__new__old__3.tsv |
| sagat | arcade1987 | 3 | new P2 | 420/144 | 1884/1222 | 1.0249/1.0049 | 13.1354/11.2896 | 0.0010/0.0024 | sagat__arcade1987__old__new__3.tsv |
| sagat | arcade1987 | 4 | new P1 | 370/218 | 2385/1795 | 1.0251/1.0011 | 9.8793/9.8929 | 0.0000/0.0000 | sagat__arcade1987__new__old__4.tsv |
| sagat | arcade1987 | 4 | new P2 | 366/232 | 2460/1971 | 1.0176/1.0020 | 9.7162/9.4176 | 0.0000/0.0005 | sagat__arcade1987__old__new__4.tsv |
| sagat | arcade1987 | 2/1 | higher P1 | 416/164 | 1568/1159 | 1.0273/1.0302 | 15.4692/13.8599 | 0.0019/0.0044 | sagat__arcade1987__new__new__2-1.tsv |
| sagat | arcade1987 | 1/2 | higher P2 | 407/180 | 1478/1153 | 1.0302/1.0268 | 16.0514/14.1481 | 0.0007/0.0026 | sagat__arcade1987__new__new__1-2.tsv |
| sagat | arcade1987 | 3/2 | higher P1 | 405/172 | 1511/1025 | 1.0270/1.0417 | 15.7838/15.0146 | 0.0033/0.0000 | sagat__arcade1987__new__new__3-2.tsv |
| sagat | arcade1987 | 2/3 | higher P2 | 419/166 | 1563/1013 | 1.0317/1.0442 | 15.6143/15.8470 | 0.0013/0.0020 | sagat__arcade1987__new__new__2-3.tsv |
| sagat | arcade1987 | 4/3 | higher P1 | 412/160 | 1568/984 | 1.0398/1.0264 | 15.2121/16.0073 | 0.0031/0.0000 | sagat__arcade1987__new__new__4-3.tsv |
| sagat | arcade1987 | 3/4 | higher P2 | 387/185 | 1533/1068 | 1.0343/1.0315 | 15.2653/15.6253 | 0.0013/0.0018 | sagat__arcade1987__new__new__3-4.tsv |
| sagat | sf1plus | 4 | old vs old | 310/298 | 3255/3301 | 1.2234/1.2152 | 6.8849/6.8409 | 0.2228/0.2152 | sagat__sf1plus__old__old__4.tsv |
| sagat | sf1plus | 1 | new P1 | 468/86 | 3384/2060 | 1.0253/1.0043 | 7.5282/7.4454 | 0.0250/0.0029 | sagat__sf1plus__new__old__1.tsv |
| sagat | sf1plus | 1 | new P2 | 472/70 | 3341/2050 | 1.0222/1.0034 | 7.4834/7.3583 | 0.0210/0.0024 | sagat__sf1plus__old__new__1.tsv |
| sagat | sf1plus | 2 | new P1 | 421/128 | 3158/1893 | 1.0565/1.1127 | 7.7189/8.2120 | 0.0561/0.1111 | sagat__sf1plus__new__old__2.tsv |
| sagat | sf1plus | 2 | new P2 | 437/130 | 3256/1978 | 1.0468/1.1157 | 7.6666/8.1717 | 0.0468/0.1148 | sagat__sf1plus__old__new__2.tsv |
| sagat | sf1plus | 3 | new P1 | 428/159 | 3433/2178 | 1.0525/1.1521 | 7.5239/8.3810 | 0.0519/0.1516 | sagat__sf1plus__new__old__3.tsv |
| sagat | sf1plus | 3 | new P2 | 413/155 | 3320/2119 | 1.0566/1.1441 | 7.5189/8.2399 | 0.0562/0.1436 | sagat__sf1plus__old__new__3.tsv |
| sagat | sf1plus | 4 | new P1 | 369/212 | 3662/2557 | 1.0594/1.1315 | 6.7697/7.9638 | 0.0594/0.1311 | sagat__sf1plus__new__old__4.tsv |
| sagat | sf1plus | 4 | new P2 | 371/226 | 3702/2679 | 1.0558/1.1286 | 6.8049/7.9537 | 0.0558/0.1278 | sagat__sf1plus__old__new__4.tsv |
| sagat | sf1plus | 2/1 | higher P1 | 410/184 | 3063/2583 | 1.1004/1.0627 | 8.4457/7.6947 | 0.0985/0.0612 | sagat__sf1plus__new__new__2-1.tsv |
| sagat | sf1plus | 1/2 | higher P2 | 420/167 | 3036/2506 | 1.0997/1.0713 | 8.4399/7.6203 | 0.0987/0.0705 | sagat__sf1plus__new__new__1-2.tsv |
| sagat | sf1plus | 3/2 | higher P1 | 341/248 | 2785/2449 | 1.1277/1.1557 | 8.5645/8.6524 | 0.1262/0.1557 | sagat__sf1plus__new__new__3-2.tsv |
| sagat | sf1plus | 2/3 | higher P2 | 329/272 | 2858/2643 | 1.1149/1.1437 | 8.5175/8.5070 | 0.1145/0.1437 | sagat__sf1plus__new__new__2-3.tsv |
| sagat | sf1plus | 4/3 | higher P1 | 375/218 | 2799/2343 | 1.1362/1.1760 | 8.8191/8.9406 | 0.1348/0.1760 | sagat__sf1plus__new__new__4-3.tsv |
| sagat | sf1plus | 3/4 | higher P2 | 363/239 | 2780/2336 | 1.1424/1.1847 | 8.8986/9.1201 | 0.1420/0.1843 | sagat__sf1plus__new__new__3-4.tsv |


## Every engine stop and every unfinished match

An engine stop and an unfinished row are different outcomes. A stop may list multiple addresses, so address occurrences can exceed stopped-match counts. Untranslated-branch stops have the existing tracker [#270](https://github.com/johnstreetfighter/sf1-port/issues/270); an address occurrence does not establish its cause. Unfinished rows do not prove a native hang without a separate bounded diagnosis.

### Final

No engine-stop lines.

No unfinished rows.

### Baseline controls

- `ken__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `ken__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `mike__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `mike__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill2trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 4837: [193efb]`
- `eagle__sf5ce-skill2trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `adon__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `adon__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 35 failed: untranslated branches at frame 3726: [122d45]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `joe__usf4-ultra1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3890: [143708]`
- `joe__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 2291: [143708]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ae2012-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__usf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultradouble__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 3547: [1736c8]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 4 failed: untranslated branches at frame 1706: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultra2__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 1935: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 5 failed: untranslated branches at frame 4323: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 19 failed: untranslated branches at frame 3352: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3505: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 3999: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 3872: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 16 failed: untranslated branches at frame 3715: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 22 failed: untranslated branches at frame 3643: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2069: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 3841: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 24 failed: untranslated branches at frame 3473: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 6604: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 6173: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 6344: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 15 failed: untranslated branches at frame 2106: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 34 failed: untranslated branches at frame 3359: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 3858: [1930e7]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 14 failed: untranslated branches at frame 2358: [192b55]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 2542: [192b65]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 3652: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 4204: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 1884: [192b55]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ssf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ssf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ae2012-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ae2012-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 5709: [1a2d6e]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `sagat__sf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__ae2012-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ae2012-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__usf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__usf4-ultra2__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 4862: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4802: [1b78c2]`
- `ryu__sf3ng-art2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4268: [10192f]`
- `adon__sf3threes-art3__old__new__4.tsv`: `# seed 19 failed: untranslated branches at frame 3911: [1a30c8]`
- `ryu__st__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3070: [10491c]`
- `ryu__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 4888: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 5 failed: untranslated branches at frame 3025: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 2988: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 12 failed: untranslated branches at frame 4586: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 3144: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 33 failed: untranslated branches at frame 2940: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 3108: [10491c]`
- `retsu__st__old__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 2919: [122d35]`
- `gen__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 1793: [17302c]`
- `gen__sfz3__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 4939: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 7511: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 4697: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 6101: [172f15]`
- `eagle__st__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 4164: [193220]`
- `eagle__st__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 3481: [193220]`
- `eagle__sfz3__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 1898: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 2 failed: untranslated branches at frame 3767: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 4549: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1595: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 4496: [192b55]`
- `gen__sf6__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 3466: [17302c]`
- `eagle__sf6__old__new__4.tsv`: `# seed 37 failed: untranslated branches at frame 3823: [1942f3]`

No unfinished rows.

### Current controls

- `ken__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `ken__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 1975: [112455]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 6350: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 5006: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 6883: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 1668: [124057]`
- `retsu__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4375: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 5086: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 12 failed: untranslated branches at frame 5014: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 3769: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 33 failed: untranslated branches at frame 4520: [124057]`
- `retsu__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 3761: [124057]`
- `retsu__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `retsu__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1415: [122d35]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 7995: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 23 failed: untranslated branches at frame 3673: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 4674: [143889]`
- `joe__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 2927: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5013: [143889]`
- `joe__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 25 failed: untranslated branches at frame 6790: [143889]`
- `mike__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `mike__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 2955: [1539f3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger1__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 6 failed: untranslated branches at frame 4744: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4161: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 6166: [193220]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3711: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 25 failed: untranslated branches at frame 4155: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 1991: [193ef3]`
- `eagle__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4425: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 1680: [193ef3]`
- `eagle__sf5ce-skill1trigger2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4569: [193ef3]`
- `eagle__sf5ce-skill2trigger1__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `eagle__sf5ce-skill2trigger1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 4837: [193efb]`
- `eagle__sf5ce-skill2trigger2__new__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 4226: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 6 failed: untranslated branches at frame 5769: [193efb]`
- `eagle__sf5ce-skill2trigger2__old__new__4.tsv`: `# seed 9 failed: untranslated branches at frame 8427: [193220]`
- `adon__sf5ce-skill1trigger1__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `adon__sf5ce-skill1trigger2__new__old__4.tsv`: `# seed 32 failed: untranslated branches at frame 4232: [1a37df]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultra1__old__new__4.tsv`: `# seed 35 failed: untranslated branches at frame 3726: [122d45]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultra2__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 7 failed: untranslated branches at frame 1745: [122d35]`
- `retsu__usf4-ultradouble__old__new__4.tsv`: `# seed 28 failed: untranslated branches at frame 1583: [122d35]`
- `joe__usf4-ultra1__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 3890: [143708]`
- `joe__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 2291: [143708]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__sf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__sf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ssf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ssf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ssf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ssf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__ae2012-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__ae2012-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2352: [172f15]`
- `gen__ae2012-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__ae2012-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 5081: [1736c8]`
- `gen__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra1__old__new__4.tsv`: `# seed 18 failed: untranslated branches at frame 3593: [1736c8]`
- `gen__usf4-ultra2__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 2226: [1736c8]`
- `gen__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2906: [1736c8]`
- `gen__usf4-ultradouble__old__new__4.tsv`: `# seed 40 failed: untranslated branches at frame 3547: [1736c8]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultra1__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 4 failed: untranslated branches at frame 1706: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra1__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultra2__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 1935: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 5 failed: untranslated branches at frame 4323: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultra2__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 19 failed: untranslated branches at frame 3352: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3505: [18366b]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 3084: [1823c7]`
- `birdie__usf4-ultradouble__new__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 4798: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 1556: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 1573: [18366b]`
- `birdie__usf4-ultradouble__old__new__4.tsv`: `# seed 39 failed: untranslated branches at frame 2883: [18366b]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 22 failed: untranslated branches at frame 3999: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultra1__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 3872: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 16 failed: untranslated branches at frame 3715: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 22 failed: untranslated branches at frame 3643: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra1__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2069: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 15 failed: untranslated branches at frame 3841: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 24 failed: untranslated branches at frame 3473: [193d72]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 6604: [1930e7]`
- `eagle__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 6173: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 6344: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 15 failed: untranslated branches at frame 2106: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultra2__old__new__4.tsv`: `# seed 34 failed: untranslated branches at frame 3359: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 3858: [1930e7]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 2138: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 14 failed: untranslated branches at frame 2358: [192b55]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 27 failed: untranslated branches at frame 2542: [192b65]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1475: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 2249: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 1791: [193d72]`
- `eagle__usf4-ultradouble__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 3652: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 2 failed: untranslated branches at frame 4204: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 11 failed: untranslated branches at frame 1884: [192b55]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 13 failed: untranslated branches at frame 2030: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 27 failed: untranslated branches at frame 1508: [193d72]`
- `eagle__usf4-ultradouble__old__new__4.tsv`: `# seed 36 failed: untranslated branches at frame 1872: [193d72]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ssf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ssf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ssf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__ae2012-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__ae2012-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__ae2012-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 4 failed: untranslated branches at frame 2175: [1a2d6e]`
- `adon__usf4-ultra1__new__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 6096: [1a25c7]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 5709: [1a2d6e]`
- `adon__usf4-ultra2__new__old__4.tsv`: `# seed 40 failed: untranslated branches at frame 4363: [1a2d6e]`
- `adon__usf4-ultra2__old__new__4.tsv`: `# seed 8 failed: untranslated branches at frame 2155: [1a2eef]`
- `sagat__sf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ssf4-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__ae2012-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__ae2012-ultra2__old__new__4.tsv`: `# seed 1 failed: untranslated branches at frame 5061: [1b78c2]`
- `sagat__usf4-ultra1__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 2019: [1b74e1]`
- `sagat__usf4-ultra2__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 4862: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 2940: [1b78c2]`
- `sagat__usf4-ultradouble__new__old__4.tsv`: `# seed 39 failed: untranslated branches at frame 4802: [1b78c2]`
- `ryu__sf3ng-art2__old__new__4.tsv`: `# seed 26 failed: untranslated branches at frame 4268: [10192f]`
- `adon__sf3threes-art3__old__new__4.tsv`: `# seed 19 failed: untranslated branches at frame 3911: [1a30c8]`
- `ryu__st__new__old__4.tsv`: `# seed 20 failed: untranslated branches at frame 3070: [10491c]`
- `ryu__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 4888: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 5 failed: untranslated branches at frame 3025: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 7 failed: untranslated branches at frame 2988: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 12 failed: untranslated branches at frame 4586: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 3144: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 33 failed: untranslated branches at frame 2940: [10491c]`
- `ryu__st__old__old__4.tsv`: `# seed 34 failed: untranslated branches at frame 3108: [10491c]`
- `retsu__st__old__old__4.tsv`: `# seed 18 failed: untranslated branches at frame 2919: [122d35]`
- `gen__st__new__old__4.tsv`: `# seed 28 failed: untranslated branches at frame 1793: [17302c]`
- `gen__sfz3__new__old__4.tsv`: `# seed 3 failed: untranslated branches at frame 4939: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 1 failed: untranslated branches at frame 7511: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 8 failed: untranslated branches at frame 4697: [172f15]`
- `gen__sfz3__old__old__4.tsv`: `# seed 31 failed: untranslated branches at frame 6101: [172f15]`
- `eagle__st__new__old__4.tsv`: `# seed 17 failed: untranslated branches at frame 4164: [193220]`
- `eagle__st__new__old__4.tsv`: `# seed 26 failed: untranslated branches at frame 3481: [193220]`
- `eagle__sfz3__new__old__4.tsv`: `# seed 35 failed: untranslated branches at frame 1898: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 2 failed: untranslated branches at frame 3767: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 29 failed: untranslated branches at frame 4549: [192b65]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 30 failed: untranslated branches at frame 1595: [192b55]`
- `eagle__sfz3__old__old__4.tsv`: `# seed 37 failed: untranslated branches at frame 4496: [192b55]`
- `gen__sf6__new__old__4.tsv`: `# seed 10 failed: untranslated branches at frame 3466: [17302c]`
- `eagle__sf6__old__new__4.tsv`: `# seed 37 failed: untranslated branches at frame 3823: [1942f3]`

No unfinished rows.

### Engine-stop address occurrences

| Run | Fighter | Group | Address | Occurrences |
| --- | --- | --- | --- | --- |
| Baseline controls | adon | primary | 0x1a25c7 | 3 |
| Baseline controls | adon | primary | 0x1a2d6e | 7 |
| Baseline controls | adon | primary | 0x1a2eef | 3 |
| Baseline controls | adon | primary | 0x1a30c8 | 1 |
| Baseline controls | adon | primary | 0x1a37df | 2 |
| Baseline controls | birdie | primary | 0x1823c7 | 3 |
| Baseline controls | birdie | primary | 0x18366b | 15 |
| Baseline controls | eagle | old-old | 0x192b55 | 3 |
| Baseline controls | eagle | old-old | 0x192b65 | 1 |
| Baseline controls | eagle | primary | 0x192b55 | 2 |
| Baseline controls | eagle | primary | 0x192b65 | 2 |
| Baseline controls | eagle | primary | 0x1930e7 | 3 |
| Baseline controls | eagle | primary | 0x193220 | 6 |
| Baseline controls | eagle | primary | 0x193d72 | 27 |
| Baseline controls | eagle | primary | 0x193ef3 | 16 |
| Baseline controls | eagle | primary | 0x193efb | 5 |
| Baseline controls | eagle | primary | 0x1942f3 | 1 |
| Baseline controls | gen | old-old | 0x172f15 | 3 |
| Baseline controls | gen | primary | 0x172f15 | 4 |
| Baseline controls | gen | primary | 0x17302c | 2 |
| Baseline controls | gen | primary | 0x1736c8 | 20 |
| Baseline controls | joe | primary | 0x143708 | 2 |
| Baseline controls | joe | primary | 0x143889 | 12 |
| Baseline controls | ken | primary | 0x112455 | 2 |
| Baseline controls | mike | primary | 0x1539f3 | 2 |
| Baseline controls | retsu | old-old | 0x122d35 | 1 |
| Baseline controls | retsu | primary | 0x122d35 | 10 |
| Baseline controls | retsu | primary | 0x122d45 | 1 |
| Baseline controls | retsu | primary | 0x124057 | 20 |
| Baseline controls | ryu | old-old | 0x10491c | 6 |
| Baseline controls | ryu | primary | 0x10192f | 1 |
| Baseline controls | ryu | primary | 0x10491c | 2 |
| Baseline controls | sagat | primary | 0x1b74e1 | 4 |
| Baseline controls | sagat | primary | 0x1b78c2 | 6 |
| Current controls | adon | primary | 0x1a25c7 | 3 |
| Current controls | adon | primary | 0x1a2d6e | 7 |
| Current controls | adon | primary | 0x1a2eef | 3 |
| Current controls | adon | primary | 0x1a30c8 | 1 |
| Current controls | adon | primary | 0x1a37df | 2 |
| Current controls | birdie | primary | 0x1823c7 | 3 |
| Current controls | birdie | primary | 0x18366b | 15 |
| Current controls | eagle | old-old | 0x192b55 | 3 |
| Current controls | eagle | old-old | 0x192b65 | 1 |
| Current controls | eagle | primary | 0x192b55 | 2 |
| Current controls | eagle | primary | 0x192b65 | 2 |
| Current controls | eagle | primary | 0x1930e7 | 3 |
| Current controls | eagle | primary | 0x193220 | 6 |
| Current controls | eagle | primary | 0x193d72 | 27 |
| Current controls | eagle | primary | 0x193ef3 | 16 |
| Current controls | eagle | primary | 0x193efb | 5 |
| Current controls | eagle | primary | 0x1942f3 | 1 |
| Current controls | gen | old-old | 0x172f15 | 3 |
| Current controls | gen | primary | 0x172f15 | 4 |
| Current controls | gen | primary | 0x17302c | 2 |
| Current controls | gen | primary | 0x1736c8 | 20 |
| Current controls | joe | primary | 0x143708 | 2 |
| Current controls | joe | primary | 0x143889 | 12 |
| Current controls | ken | primary | 0x112455 | 2 |
| Current controls | mike | primary | 0x1539f3 | 2 |
| Current controls | retsu | old-old | 0x122d35 | 1 |
| Current controls | retsu | primary | 0x122d35 | 10 |
| Current controls | retsu | primary | 0x122d45 | 1 |
| Current controls | retsu | primary | 0x124057 | 20 |
| Current controls | ryu | old-old | 0x10491c | 6 |
| Current controls | ryu | primary | 0x10192f | 1 |
| Current controls | ryu | primary | 0x10491c | 2 |
| Current controls | sagat | primary | 0x1b74e1 | 4 |
| Current controls | sagat | primary | 0x1b78c2 | 6 |

Failure lines not classified as untranslated-branch stops: 0. All remain listed verbatim above.

## Raw final-file checksums

| Raw final file | SHA-256 |
| --- | --- |
| adon__arcade1987__new__new__1-2.tsv | 1245f31bb6be8118a5cda80bd8c4e0ba2c1e07cd3a7889a6f1ae1d5292e0ae8f |
| adon__arcade1987__new__new__2-1.tsv | 94dc1709af003299537b36d50ddbf33dbe82e200f3e2a141ccb8cd541dc7ce3a |
| adon__arcade1987__new__new__2-3.tsv | 05285f79be59953ec1fe7be46740c25730a3a9250bb95d720aeabf4e3677efde |
| adon__arcade1987__new__new__3-2.tsv | ca82372ad894851da8c471f5ad5c0e1736db93f3bf87cdd3332b28ac06ecd26d |
| adon__arcade1987__new__new__3-4.tsv | 8775013cfa8a522dffb14b4462eddfae9791383cf9a24e36069acdce648093c9 |
| adon__arcade1987__new__new__4-3.tsv | ccc385cfbbbac4700d8e976b71548c59c3505d62b2c9842090eaca06b1eaeaa2 |
| adon__arcade1987__new__old__1.tsv | ca9674c4988a38c0b6a347556ab929c0a8986f9909fe5395f21fe64946ff6bb3 |
| adon__arcade1987__new__old__2.tsv | 3fbe2f2941bd2b8d25e663ac16fcfc46ec2c597f939db583dfaf3ad9f2132c46 |
| adon__arcade1987__new__old__3.tsv | 8f3480e8825a3d117ce8747955fcc48a2f1c4251112ddbb6fcd6eb9e6c79c87f |
| adon__arcade1987__new__old__4.tsv | fc39ccea4528758faa764801231191827c5c37eca7b008bb88f427e174f17864 |
| adon__arcade1987__old__new__1.tsv | a47e0acaf55804b9e2364247baa56db01e49d71adca0211026e36795f015c317 |
| adon__arcade1987__old__new__2.tsv | 3e8a465941698071ebfe74a60934b028bed0cc866f26b7c42973cc174bd26074 |
| adon__arcade1987__old__new__3.tsv | a55e2a95cab070ae463996d75604e55a75cb880cbd9e25b7bea69275b4225334 |
| adon__arcade1987__old__new__4.tsv | 276143d78ce533431b18a06a3fa782a9589f988c73028b80ccd1c52b9350789f |
| adon__arcade1987__old__old__4.tsv | 3db3a2e11109779513a8b72448df2c24bfae75b6bd330c763edf8fe41436c8b9 |
| adon__sf1plus__new__new__1-2.tsv | e772ea9149b5ef219534a2aa17c2c9f1fc2cdb19c8f0ed45ffc0717fc3c38f23 |
| adon__sf1plus__new__new__2-1.tsv | ead84233329dc7c367a586620c09f46252dd22f57a540449f2299bc8192bcd2a |
| adon__sf1plus__new__new__2-3.tsv | bc498597d9ebdd3863b3581fe374c7b16cd7d964581cdfad6983d695106276fe |
| adon__sf1plus__new__new__3-2.tsv | 0fc3e0a005342940c50ef24e5f040a2a19f53aa6c5632925dab7d73425aaa154 |
| adon__sf1plus__new__new__3-4.tsv | 2431fbe85ea5c779197d50aea5531f1517c841519e005a51b34f4e70492959fb |
| adon__sf1plus__new__new__4-3.tsv | ec059be7218d9481fd93ccc835ae9ca2e57b4bad63256b3dc26c9ab4742a4358 |
| adon__sf1plus__new__old__1.tsv | 1fbac24d08e88a7c12f29e2b84613a009b2373b0226c62deb10b710441fd29be |
| adon__sf1plus__new__old__2.tsv | f1f07ac37e34cb46d001ac341caeea51c724954405015e5ef9d1824829e38326 |
| adon__sf1plus__new__old__3.tsv | e927fb73ed8aa26b09dd153eced3c685bf5eb8a89407c588a6c483ecf7552178 |
| adon__sf1plus__new__old__4.tsv | 37d769312652afd522b6c8366d9e1cce120d3e15c85843f744ea95ea68f8f3c7 |
| adon__sf1plus__old__new__1.tsv | 150b607dbf4c5b69e325000d6b1ebe484271a0b7e623d3a31d570ced59c28292 |
| adon__sf1plus__old__new__2.tsv | b4ae1c39d60904c9c6d203888d1afa4ea065741d29aafd8dede4f80ac0736828 |
| adon__sf1plus__old__new__3.tsv | 6ac06911a3a3c0ae4b1943c12942c0d5772314a11e52001a40c94ef266a5e50a |
| adon__sf1plus__old__new__4.tsv | 193da9fbf6281a85f616bf76df49b3e9446606ba3e70235ec953d50130062607 |
| adon__sf1plus__old__old__4.tsv | e8f7bc795a295b095a75bbef0c14736041130fb37a776bba2e119bd7ec7906de |
| birdie__arcade1987__new__new__1-2.tsv | cb3abc759dfeb78b83c66cd6c0f8b048898c17379619359ee5439e36d07810f8 |
| birdie__arcade1987__new__new__2-1.tsv | a9bcf00b31d12456b3225ba7b96a155fb62244ebe1e60b529d104c516e38ce3b |
| birdie__arcade1987__new__new__2-3.tsv | 1762dab9e52652b3f1686f323fb82f1c95ed04ed3913351ffc2bae84d14fc12d |
| birdie__arcade1987__new__new__3-2.tsv | e33c25ea93cd5df740985c316cec83529f026a8dba9a9eec0417632f0f40bc66 |
| birdie__arcade1987__new__new__3-4.tsv | feac14d2827837fa7216bbec901a40cc153951f4701b38c0c20ee6ec0cdd536a |
| birdie__arcade1987__new__new__4-3.tsv | 2947ca4d200cf910a1f08286ffc5eaa1dc930730f99c76a07dfcdd42ac6a0287 |
| birdie__arcade1987__new__old__1.tsv | 05a002928cb4c0568a63b5492a5b97f3a367848f67203647494c7f014b4f2e56 |
| birdie__arcade1987__new__old__2.tsv | 104ad40eb02cee5cfef150533edd7451360fd5f52a9a205e72711ede0cd0ff6f |
| birdie__arcade1987__new__old__3.tsv | 3db871dee5de855a52974e33cea857a6faaa105e9cbc56d88ca0a24d5a50b3c0 |
| birdie__arcade1987__new__old__4.tsv | 47d52e8d01e888eab5954a1f90d723f9c426ce2fd6e50deaa6c425d28475c1f4 |
| birdie__arcade1987__old__new__1.tsv | f6e0b24a3110f1dbae34d9760abdd6451c171df369087037b2fda9045fa23e25 |
| birdie__arcade1987__old__new__2.tsv | cdc8e2bdc0eacadbe1bc199117322d3ad86020a3ea08ee1d0517052c36dd9581 |
| birdie__arcade1987__old__new__3.tsv | 11923ad5e97a702d3a16479ad1bc30022b0fc01776978b6cbca31bda71940974 |
| birdie__arcade1987__old__new__4.tsv | fe2ac4086164918e6a9be93b5f98261d8d8901ff1b63c33e00d80db3320036f2 |
| birdie__arcade1987__old__old__4.tsv | 2c0f656a2be2bb848dc1de4cee0d43e67edd2698bb2d632a5c00565cf932e6c8 |
| birdie__sf1plus__new__new__1-2.tsv | f5d10f398f4150e303fc2d7d56a0f00377f7e95aeeab7a82106bc1f69368c56c |
| birdie__sf1plus__new__new__2-1.tsv | 8c9d0c2dfe7a0bd0a36c5a4c26cbd6a743dbf750f41e6f3d47c8ea23a05afe20 |
| birdie__sf1plus__new__new__2-3.tsv | 34e017e8ef1583d47b2002926eff8673d085c835759680382a8f87b99ddcdabd |
| birdie__sf1plus__new__new__3-2.tsv | 14be1e22aa8e8962389d2b8d7c3a4e01eacea4684c2e60a4724520800f4702e1 |
| birdie__sf1plus__new__new__3-4.tsv | bd3a50ad13775c52571eb77054363c67bd1154353d3f1d534aaa122e2a3fbadf |
| birdie__sf1plus__new__new__4-3.tsv | 9ddbe0ae06216445a9fefa10afb5c063e1bb4b1588253ec5cd7008c2e2c9bbd3 |
| birdie__sf1plus__new__old__1.tsv | eacfa6b9b22b89068d7d0d33043ecc84416865e033d077365f62383499ed2c22 |
| birdie__sf1plus__new__old__2.tsv | abe130883662a6e2c1d0c498591c66e8eddbc32e8007b9c64219a213ef4b052c |
| birdie__sf1plus__new__old__3.tsv | 01e81873ff825cac82fa20861b6a82e698701233c86deebc48c4cbfabae0e8f8 |
| birdie__sf1plus__new__old__4.tsv | f21e32a279d8122195671bc0adc942927a56b3c5179f05a9a8dd296adbbfa3d9 |
| birdie__sf1plus__old__new__1.tsv | db1186e2ed3047d9fc7e35cc1e64a6b880686dc3fcbd9a62780cd93627502b1e |
| birdie__sf1plus__old__new__2.tsv | 0d04bada39d58677101e0bae6c48325253ad2337a0398925ae7f381aef881b65 |
| birdie__sf1plus__old__new__3.tsv | 3ab4a2b8187738ea7b18d2aa915837fdf831021ed1a8e6d0066c124b3fecf879 |
| birdie__sf1plus__old__new__4.tsv | 018301ed13c1b71fa7818abcd92a9521d91baa8b498aa7cf4e2bc7b7d5055123 |
| birdie__sf1plus__old__old__4.tsv | a663054cbe34c29d485f719e4b3394e477a466ace373349fe6c34b91466d3125 |
| eagle__arcade1987__new__new__1-2.tsv | 5f2d5d6b49503c7a123922e17c7d6aa6a5146046d10a884bd0a9e86122f9f9f3 |
| eagle__arcade1987__new__new__2-1.tsv | b587152dff2f66c3501a26805054beb34a65155b4786a088e442946c1f10719f |
| eagle__arcade1987__new__new__2-3.tsv | 14b1008efd71ec178e692da0ade1416ef2819836fbb60d7e072403065e2da138 |
| eagle__arcade1987__new__new__3-2.tsv | 482af0d78c0aca8b2979fba9a2c584e1df8c510c9d756f7ea43b470a11ae6a02 |
| eagle__arcade1987__new__new__3-4.tsv | 2b807431065680d84fdbd8de12742c08a0ead15020b928be5cab75c98f8887ca |
| eagle__arcade1987__new__new__4-3.tsv | fe31d7241edde781978139bf072f10703e35827e6e581c01b971765ac83ae25b |
| eagle__arcade1987__new__old__1.tsv | 60d56b48ba65ea34babfce0124c199d63a20017722ed72deb132904c4b04cd64 |
| eagle__arcade1987__new__old__2.tsv | 3e43aaea5ab8bdd5d1a0958f98dd2dbb1243afdd6027554454ba654c8c9047bd |
| eagle__arcade1987__new__old__3.tsv | b3902a433abfa3ccb19b587fcfa5d8f51454f02c184c8ca6cee371417d5a0b0e |
| eagle__arcade1987__new__old__4.tsv | 909728b798273971aab1c25da29c56e42c62b6284457c12d89d0051fb367e13e |
| eagle__arcade1987__old__new__1.tsv | 52f1cfd468c70e7ec77fd9c87a82e88932e2fadb353caf7e9fb4638d6e8fc893 |
| eagle__arcade1987__old__new__2.tsv | 2ded6bfbe866a23103f2832416284fceff6c6eea5945ef3e80e02d9764bb60d5 |
| eagle__arcade1987__old__new__3.tsv | 910f5d652164792e688c698cc2f160eeb7573d4bcda4cee4ff2851b9097aff57 |
| eagle__arcade1987__old__new__4.tsv | bb43dd593840dad4427bda4035e094127bb3a5d57c6c727e1dc125cf8679f6d5 |
| eagle__arcade1987__old__old__4.tsv | 11aa345e0e147b187c6c6babb1901021d13baf89c5f642fdc1168723e4e815cd |
| eagle__sf1plus__new__new__1-2.tsv | 6a36367dcf1c193bc31a9c559f1679880ee7c638dc52c76f7f1e5b0d536e62b7 |
| eagle__sf1plus__new__new__2-1.tsv | 2f3b7dfec7e3a104119bb6af7e30bc11cf30eadeb3fecbb085439b03284a8af2 |
| eagle__sf1plus__new__new__2-3.tsv | 0c6bd25d92edc78a06ed47384e27d7dfb72ee3b9e791c5fe645c458170f7d141 |
| eagle__sf1plus__new__new__3-2.tsv | 4cfd36218f9e31195d2e344cee6f47ffa11e6a4c05a6df93c622c4f1c3614238 |
| eagle__sf1plus__new__new__3-4.tsv | 5df8ad34986bd24729e582fbb9a2376f325458b4b4f4ecde8602989ad8f2fd5a |
| eagle__sf1plus__new__new__4-3.tsv | f9903d7d69dd80faf0b6f58db5ac4f0d1424f1a949be7a722806d50107f7da76 |
| eagle__sf1plus__new__old__1.tsv | 401041ca3392bcca3ba6d99902fc6689e1e52f1afda5397fc89f57c5f50b4ea9 |
| eagle__sf1plus__new__old__2.tsv | dbafd19a5d5a6cf17abdf9b6a4a0a7bbb1bc8fcf28a1911e80e1a5551075f792 |
| eagle__sf1plus__new__old__3.tsv | eee34ef0b9ac79b664fd3017184307de77192bf1f09c2eb399e931017de3b2dc |
| eagle__sf1plus__new__old__4.tsv | 86f5f5896224de1ade45e52c9f4fe3b747484fb73e97e5a5f445b76e1cb82b0f |
| eagle__sf1plus__old__new__1.tsv | 8c55968e9d8baaa3aa9b074f8a348504b8223e6ee58a4189c9c310a7ef69cd23 |
| eagle__sf1plus__old__new__2.tsv | 78a4a2700e81aa69050cecc54217e882f25d57031f2418ffbadd49695ed1d3c0 |
| eagle__sf1plus__old__new__3.tsv | b3ce9effc28728f98bd81ac5879813e159e93c13ff901a7171033a919c57ac93 |
| eagle__sf1plus__old__new__4.tsv | a8b55342514ac638a8cb95a6871eabb54d4506dad2200762004cf4aa8cced9df |
| eagle__sf1plus__old__old__4.tsv | 9e026c5cbd38e3bbf483b1d813e0ba05b623e3439202420575dcdbcf1c527a5d |
| geki__arcade1987__new__new__1-2.tsv | e2481ebe309270a375f0fb05b4b36f61b778859ba6d7502727ce5d97e2509711 |
| geki__arcade1987__new__new__2-1.tsv | a1be8e9401f1a8399b50e663813dc96bb054051e2e43bcc037f93c7393a1e679 |
| geki__arcade1987__new__new__2-3.tsv | 4ec628219ea09d406cc53d57060223ec6e355112c433b94b9a020efb6aa1f178 |
| geki__arcade1987__new__new__3-2.tsv | bab3b34cc2a05c688fd8e3f6a3f1423936be4483507f179cd52c89d073e9649d |
| geki__arcade1987__new__new__3-4.tsv | d36066fa1b2979fd6ae4381a381a8e0be45ee008cdbf4939089d945e9288acdf |
| geki__arcade1987__new__new__4-3.tsv | 964c7b84b7748fde93d9b24f7e23d0c3152103b0db9d9c0ba9fdddd3c0be4551 |
| geki__arcade1987__new__old__1.tsv | 1b94575d2fd911e42bea5bc6528bb2e398f6423b388d9f6986bd679664ba22b9 |
| geki__arcade1987__new__old__2.tsv | 703a7742642251c2eaf14bb3ab30728ea9130c1467d0066621ccc60aac53d2f5 |
| geki__arcade1987__new__old__3.tsv | 29c1c2003a0c5724e516e02ca5b13b4d2e6e9f6dcaba0dca60801378f54e52f8 |
| geki__arcade1987__new__old__4.tsv | 53b4c0cdfd258e95e46121081d566937ca4660a075de96bcfe0afe08006ff2cf |
| geki__arcade1987__old__new__1.tsv | 94109f584ebdef40c71a14e17c7ead50fb8524bbf57c9fdd32155562617649fa |
| geki__arcade1987__old__new__2.tsv | 3f2bed115bc2d387b622e40656abd89b96cdb8f427638eead73b205a923263b4 |
| geki__arcade1987__old__new__3.tsv | 1dec7ba469f7657aee2b5c9647b591cd52aef26a2c05884e1d1718c62284c632 |
| geki__arcade1987__old__new__4.tsv | f9a0acb59132a656787a22dae8f2840e002638d54f8b27b5b39e40be4c653946 |
| geki__arcade1987__old__old__4.tsv | b5450a31f6ed90cf0f5dea81e52975c9b809e088dc105bda945bf63d6a88e09e |
| geki__sf1plus__new__new__1-2.tsv | 3296a4d3f140aefb1e6eff7ef9b47b33402d2d256576777339af6031ce3a2e66 |
| geki__sf1plus__new__new__2-1.tsv | 506e022c730eee13d184804943d144f055da2f3899f96d9937dcfee41ed64cb5 |
| geki__sf1plus__new__new__2-3.tsv | 165f3ff24b5999b3ce0ec9928d494f29b3d0dda2eee36d646f478a19357c3e89 |
| geki__sf1plus__new__new__3-2.tsv | f5cb957b0973668adadcf04a477f7d0a82a51299738f8a8875c86ffc21f96048 |
| geki__sf1plus__new__new__3-4.tsv | 086efcfb8a66504294960700382ef440d3786d5f8546e2444996269d78b203e1 |
| geki__sf1plus__new__new__4-3.tsv | ac0b8122f5ac9eeec57e1e113cabbd026f6945919626b35f95283961c0b8409e |
| geki__sf1plus__new__old__1.tsv | f779d6103a510449d7af80933515f6730edd8dbe2105b1800579766d03e7880b |
| geki__sf1plus__new__old__2.tsv | 69a7c7012e51c37834094937980b2a33510db31d23a838da4016f411efd8f22a |
| geki__sf1plus__new__old__3.tsv | d8d408e7274de08bb0fb45c792e66f2e06874338bc22e7dbf5015ddcdcb89da7 |
| geki__sf1plus__new__old__4.tsv | d7ae521fa13e51bb79924b88371d457a1910e4b4f279c20902a0414fd82c83fe |
| geki__sf1plus__old__new__1.tsv | cb541a0e84285216e8bc3ee4cd26defeb07869cc535a2735f40fd70cedda65d7 |
| geki__sf1plus__old__new__2.tsv | 093c701cf5dd7cf8c27cf1c8bac9cb636b2a16f473dfd57994579b23b442b3e2 |
| geki__sf1plus__old__new__3.tsv | b2eb8a6ae1312016a362337e5d423c0ec76e435dde0ae6cbd18a24a46187cc1b |
| geki__sf1plus__old__new__4.tsv | 87386f2afb0b2a8da4f0f2e2e97f3bbcfae72f97315005ad8a555f6cf179dae4 |
| geki__sf1plus__old__old__4.tsv | 6cfef8530f088377c80846934c3983c8f923861a43c58a4291a215374e84a073 |
| gen__arcade1987__new__new__1-2.tsv | f3281875c243f727cc1fbd48d9495d555c3bd2bfe621b81c2f0dba7c142f74a1 |
| gen__arcade1987__new__new__2-1.tsv | 3a14d2e372a7305acb8bea566dc8cc85fabd220b248f39ba06e0e9b66f708402 |
| gen__arcade1987__new__new__2-3.tsv | 4352e44533b341770e0f070ed21ef5632f983aa51219f9f8c342ba92ddd9668b |
| gen__arcade1987__new__new__3-2.tsv | a12d2e8e186d5f66f7d810ce72db01a124fffa6f723fcdade436cce2d646937e |
| gen__arcade1987__new__new__3-4.tsv | db09fcf40203758b7062a748c2393ac9f3d5388de2ba55eb07161b7672f6faf7 |
| gen__arcade1987__new__new__4-3.tsv | 7f168bdc95f4f4be43bd36984cb5df3fdddcae377534dc794c7d040d84517d8b |
| gen__arcade1987__new__old__1.tsv | e1cf5dcc686df51975cfedd80cc54902c9116f0d6abe2eb18acc16069a24f26c |
| gen__arcade1987__new__old__2.tsv | af138be8a13bc07db55fafafa2b92d9859675ebd96ab685e56d4003e3c3b971a |
| gen__arcade1987__new__old__3.tsv | 506de72ebd340552d9035789c8da774a206cf575c29e2728c2b02f5024aad5f6 |
| gen__arcade1987__new__old__4.tsv | 7acadb49572019ec5ac9b0d26872fe9de319ae306f84e1a082326961c37699f8 |
| gen__arcade1987__old__new__1.tsv | 4e02a8945ed4b941fa3c836bbd38ba997bb05137d2039a95bc2d0755f58fc950 |
| gen__arcade1987__old__new__2.tsv | 957d318ec9cb2cefd39534e056ae1a0690fe5a548fe5c97541c4445525b2a8c7 |
| gen__arcade1987__old__new__3.tsv | 02442c7058d247f01c321284bee4b5c95ba1133e664a860e44872accb58dd224 |
| gen__arcade1987__old__new__4.tsv | 6c0988d51feb44b96c788c26f2a8c375cef32b9f3e9d127dfc5e215288270819 |
| gen__arcade1987__old__old__4.tsv | 326df7bdee6af1a03bc8c16011c9d144535ac07689bf8387a0e24bd4c18de489 |
| gen__sf1plus__new__new__1-2.tsv | 933aa22780b22d8186a736cb3d10a6e0e95bb7ec53e8394ae7bddd4e130d8ae5 |
| gen__sf1plus__new__new__2-1.tsv | 3547af8a634cb549444f40e90be4690513215e0b7c5661470a9a07ee2a12df65 |
| gen__sf1plus__new__new__2-3.tsv | a14791164298645f5fae17c606925fbe5ba86e69369d71560ee936aa82709af8 |
| gen__sf1plus__new__new__3-2.tsv | 23b54d00cdc1355db6a5ddad203a1d7c0d15bb4b5632bb7a444860107b20b0c6 |
| gen__sf1plus__new__new__3-4.tsv | e79b18c45579ecada608405e230719d14aec3c0c9e8be87ecb4d7dcc5c49b9c8 |
| gen__sf1plus__new__new__4-3.tsv | 866f68779e9f7ec2e863d76a258cada65079e2561bbf554e2d1d6a47218e6457 |
| gen__sf1plus__new__old__1.tsv | faaf3687dfc3310882936beb8e5254735fa75845a2bce894555b22b88eb7b6ff |
| gen__sf1plus__new__old__2.tsv | 7b721e2e0c1fb55469746b908831682586712ac1e81fb126a8c52b34cba57a4a |
| gen__sf1plus__new__old__3.tsv | 27e7d196790609f858525d253fd83ecae6d0c3e18029deffb3f0a46ee4752a00 |
| gen__sf1plus__new__old__4.tsv | de8b1861be4c16b067a0dd004187bf3fd8e654c73c297188d92aefd079a86b00 |
| gen__sf1plus__old__new__1.tsv | 4a6b774918a5a4fe9e6128c2afeec6f23ad5e67d608dcc64d686f6108d72b7e7 |
| gen__sf1plus__old__new__2.tsv | fe1b333576844527e32b391fbfc7fb4d3e16b24b58c9fd0c4bbcf6ccbe7333c8 |
| gen__sf1plus__old__new__3.tsv | 174b53d0cfa1d34277e68415d669be96743abc3eb9549cd390a8e33e79dd712c |
| gen__sf1plus__old__new__4.tsv | 7fa090ad7cf7e8da18d64393010612dff2b6db1edaeaccd7fc4fd67c2ca854f8 |
| gen__sf1plus__old__old__4.tsv | 297fc82aa3b68b99cc722af742366b5f2b1bb1c4b65fe043b4f11a449633c442 |
| joe__arcade1987__new__new__1-2.tsv | 07c3ff51e6521d8ed1312a2cc94731050fadd8f6415acdaa7a28f267c795444a |
| joe__arcade1987__new__new__2-1.tsv | 2f8240e8cdf34265970915a1d8ea19c1c36d860bc43182aad0c79c44d0f99164 |
| joe__arcade1987__new__new__2-3.tsv | 9bb2f74ea48d63791476658ebc730195a85651c12e54443de61fdc28fc731d94 |
| joe__arcade1987__new__new__3-2.tsv | dbd1a123103c640c1f4ef5a50b9dc10028e6903817e7b8819643d71ea62907ef |
| joe__arcade1987__new__new__3-4.tsv | ac8dfdf9b22af6ba8ddaaaa86e348b5cb1b46b344d3655b1f973f7364acc5ad1 |
| joe__arcade1987__new__new__4-3.tsv | da3c9756ff205e7a43c1bd8089315f49d37ce789e895694debfc1164c6e5e855 |
| joe__arcade1987__new__old__1.tsv | 7d838e438c4fd096f872a1983bbf43f3513fddb11f881a99e937c2b1750aceab |
| joe__arcade1987__new__old__2.tsv | f78115999b7e1393839bcd302604527907f9edffd3aa6bdc2d76341565619eae |
| joe__arcade1987__new__old__3.tsv | 04f9e01d92d07907671e45d752833c369d58b35e6dbd1ac39e461d48fde7e157 |
| joe__arcade1987__new__old__4.tsv | fb0819c756bb27663a6e201d5f4702f6d51cecd1670cad6c2fd50f9dbb35134c |
| joe__arcade1987__old__new__1.tsv | 55f581284bcb9a09f639b165ceafba86e28fe879b4c46e124927fb2f34189bce |
| joe__arcade1987__old__new__2.tsv | 21fd32cc875bd93ff95bd2c2aa079a9668d1f84c7b1316eccc39b5cb6414a397 |
| joe__arcade1987__old__new__3.tsv | c661ad6c9c5060bf190b88fb4514cc73f59d4d813a66d382e683bf5803f97453 |
| joe__arcade1987__old__new__4.tsv | 87f5859ec5b17a3d57fc73b055a8e3d8044ac4968c8f2f815fbdeedf8f7a45df |
| joe__arcade1987__old__old__4.tsv | 680b5f3d844d4a1658b13dd0393bc96dc18d7ea3d9bbb271fd4691f6f1709986 |
| joe__sf1plus__new__new__1-2.tsv | fac11182e134f624c78899e53192bcaf093fddba5e3dd433d657248a1e670276 |
| joe__sf1plus__new__new__2-1.tsv | d0ecbbdb76d9009f1e2a6eb429f9440657a4abe4261277cafd41fea5840a9196 |
| joe__sf1plus__new__new__2-3.tsv | eaebda6edd882412eaa500cb24b695ce3effe3203f0495a13e709a599d6cf5a2 |
| joe__sf1plus__new__new__3-2.tsv | 5484a62e89d807ea3f27050b4d99ff241f096862c97fe76939b13deb50d09e40 |
| joe__sf1plus__new__new__3-4.tsv | 7445c58798c8ac7cd813c365202acb2e1aa5c74e7c40622ecc3ca5c9735eb123 |
| joe__sf1plus__new__new__4-3.tsv | c94754a0fc43252938de0ab0aabda10bc970cce198bd70e96cb693a15b8510b9 |
| joe__sf1plus__new__old__1.tsv | 07d32858e0444b4e3479c3e44696711701700e7e46f01b4e30fe8f44513e6a16 |
| joe__sf1plus__new__old__2.tsv | fad0eda220383b2d6bd299f280a3567928a6139bc393f3addb1f8df42a202dd6 |
| joe__sf1plus__new__old__3.tsv | 24028de01d770851014e92f2d21bd3d57e56cab529b623ed88d4ed58d6fcfc22 |
| joe__sf1plus__new__old__4.tsv | 82e11f39fadb9393f0eaee2ed004d3501cb54f138b6f3e080903e1895c3deb40 |
| joe__sf1plus__old__new__1.tsv | 6db688f379a5cf26add9c19284b9823cfbdb2a72c26c4bfe5949a5184394b3fc |
| joe__sf1plus__old__new__2.tsv | 6642cf9303d99f2331cde9810a81c5c7ad08bdd407a95072a26d8b2550df13e9 |
| joe__sf1plus__old__new__3.tsv | 8cf2f1588dd0458b20c4c3f672674565987b5becdaf01161011e36c77e5fa879 |
| joe__sf1plus__old__new__4.tsv | 1c7cfe59f2a9db4db78d6c5ec4b71331303df9ebf6053ff48749044b83620b32 |
| joe__sf1plus__old__old__4.tsv | ac135434cb27c2153dadef0c415c4f8194b50ec8f551bf44f90e16f9d86c90d1 |
| ken__arcade1987__new__new__1-2.tsv | 4572012b064a2d455de48bb050c63ef198d447796c12f56fddee68db66ed4d85 |
| ken__arcade1987__new__new__2-1.tsv | c9e1a912070fa5ee9e3964eaae928f68c6806dcf7b51fa8885244153ffe39d34 |
| ken__arcade1987__new__new__2-3.tsv | 969254131f7cb1915bfec5ac2cad34a70733a871cc0c1b776d2bc0b1aff46202 |
| ken__arcade1987__new__new__3-2.tsv | 68badd918011cf5dd862082b7ad2292ef4297408605a012fc252a8d4b54cb75f |
| ken__arcade1987__new__new__3-4.tsv | 4922c5223f063547e4c919a1584abfcf87206c93a9942a985c53511f3facc229 |
| ken__arcade1987__new__new__4-3.tsv | f45b852acd7a8684e03d454d29c620c69c1232b8efaa94719533d10a035e091b |
| ken__arcade1987__new__old__1.tsv | cc908d3f48b4cdf231f9f1f93d88e1ebc18b87813398571159ba3f162120dd50 |
| ken__arcade1987__new__old__2.tsv | 78505b53fdfba300ad8cb0b22a70746dc625d0f091d20cc511a4fb0da83332fb |
| ken__arcade1987__new__old__3.tsv | d914d1c771699c9a16e059b05fe477fee8b40f2baf9a7838bb3955aadfacd993 |
| ken__arcade1987__new__old__4.tsv | 0aa2ea6ee6876a5ba3e60baca318d35bf423797c1ac73d715df606edbf1804e0 |
| ken__arcade1987__old__new__1.tsv | 6753ba5bbe6ecefaca105b662c08ac65696fe17381ab44e4e85b14d1b167139c |
| ken__arcade1987__old__new__2.tsv | 74f58d472f99b6bef2111f7acaae89acb47d13e2b9f71c559139dd72965fa6a0 |
| ken__arcade1987__old__new__3.tsv | 54b1782d401b3842303060443b02a67a380b997f917443d5a3044e06129a0896 |
| ken__arcade1987__old__new__4.tsv | 83408115802e921d284b5f57a733d918ee3ad95f93ab37070b346308a65590ed |
| ken__arcade1987__old__old__4.tsv | d39fd4f64eaebf096307e3da10688855f2a914b892d1052afc15aa275891bb76 |
| ken__sf1plus__new__new__1-2.tsv | 133f6d2ba48b5c1fff2fc21a9a1f5d814bc5e5c905116cbb88b5815cf06efea9 |
| ken__sf1plus__new__new__2-1.tsv | df88d8edb1d5948e11f64364b4b4b3eddc0c3fb8cb7e11f99e9a55976380d293 |
| ken__sf1plus__new__new__2-3.tsv | ab540ec233d9215fdbce9412c33ed362ea9e5718d1e41d572e10054552e99cab |
| ken__sf1plus__new__new__3-2.tsv | 038a46ee652e798ce948637035fb9f5a34868ff40a5a7bd9ffbbea8beedc99fa |
| ken__sf1plus__new__new__3-4.tsv | 122fe4180305f71a1866055e713991c6e8ff79f4b49da1bc73ed15273b67961a |
| ken__sf1plus__new__new__4-3.tsv | 5aed83b20db4ba97a65200be0ca7d21328f276eb4f3f5aa8810c90a636bc2916 |
| ken__sf1plus__new__old__1.tsv | 4f30f1ba8a5cf877fa9eb66704094c2242255407f6824573676acba08c7b6b11 |
| ken__sf1plus__new__old__2.tsv | fff30f27b06d4447a73449a90e238e5f0dbb26c633505a01502df4cb21aed628 |
| ken__sf1plus__new__old__3.tsv | a38be343d12cb30210a1e41e498b452abc51f0ac97646449ed1f063b3c334c26 |
| ken__sf1plus__new__old__4.tsv | c83e965ba9044f1bc8664d0928e6f4cb6d0594524bbe4d45272affef18761d45 |
| ken__sf1plus__old__new__1.tsv | bb6f970f99f9e33e8ccad00220fff7865c5c29fbaec3a75eb89413533069c541 |
| ken__sf1plus__old__new__2.tsv | 8edce2c4313b10fa1f929fdd412db90ecd80468198b8b688b213fbd50aac11e8 |
| ken__sf1plus__old__new__3.tsv | c3e3066cc35011c2daca12e996970b5085c716c5506b49e8b730ceef6b4f9ee2 |
| ken__sf1plus__old__new__4.tsv | dd680944ec8b692284cdf385ababa5b5f4602d2143d2456224a8258faca7307b |
| ken__sf1plus__old__old__4.tsv | 6e1b6a2dfb6a9d2e1b6bbe1c6dbce1db115832250495fa8d23ecc0a9514657d1 |
| lee__arcade1987__new__new__1-2.tsv | b47d4c8be08ded16a926831df844005fe3a2d605b6f64b5df97becde5c3a840e |
| lee__arcade1987__new__new__2-1.tsv | eb97f6125bd5ed5595b489408f967f3690598cb6343ec95c9eb4ec628647c791 |
| lee__arcade1987__new__new__2-3.tsv | 716a0c28b08df7db6430934b8883972f5c4eb0fbae547a0e33edfb39f2fefbf4 |
| lee__arcade1987__new__new__3-2.tsv | 2c2996f2f331afacba083f678f2b51edc70d6530a052f95bbd1b545d737988af |
| lee__arcade1987__new__new__3-4.tsv | aa59114ff2277cfd798a30130996e53109220c8d917801e18a10b40454ccd4fb |
| lee__arcade1987__new__new__4-3.tsv | 50b404aed746964e150023e234a28b404a14f8991c05cc9c49de51499b237127 |
| lee__arcade1987__new__old__1.tsv | 6bd37ba208ca860eb23380af3335e75bbebe8ca0f311975bee22fac2393b3be3 |
| lee__arcade1987__new__old__2.tsv | 1391ecd27e5a7e351e6cbb9a86a2ac618ebe5e74c824b08412a7360c42f26fc7 |
| lee__arcade1987__new__old__3.tsv | b7b0b4d1aa6aece0553f3690c7299b6bef89cca77aaa449729c584826e7edef8 |
| lee__arcade1987__new__old__4.tsv | dd8d8bf9a021b22258a4fb4b969fc418476d3509bc217468972abd6bca0555e2 |
| lee__arcade1987__old__new__1.tsv | 8c525e4880d8fe3cc581abd95fc70c35c47e28202f69dc96ba3a9a5d66e813c2 |
| lee__arcade1987__old__new__2.tsv | 64cb742bb0537229b5935adf32ceac25fb2b46aa225b298ce9d3540af6a6aac6 |
| lee__arcade1987__old__new__3.tsv | 7cd5d1b7c9d205675427bb63205bba6ff10761c0fef928924f6257011392e487 |
| lee__arcade1987__old__new__4.tsv | f5792800a6e41363cf50a598f54018bec8f532dddb44a137dfcc96ab1df626a5 |
| lee__arcade1987__old__old__4.tsv | a961705a191d3caa5e0f1e76a0b06629911d46770f3b536d455cb64906e3ef1d |
| lee__sf1plus__new__new__1-2.tsv | 7978889982f35c96a4154621cfb21297c5a723bc012cc5f5a7a48fde877e75ed |
| lee__sf1plus__new__new__2-1.tsv | c4da4e105fb80fb362ed076845452a73970808dfd353e5584c7f0bc37de21de0 |
| lee__sf1plus__new__new__2-3.tsv | 083342c3aa3bd821a40ec10dd4bd785126c16117a8950e4c1e927c309b30f2c8 |
| lee__sf1plus__new__new__3-2.tsv | f480b59bf2c60255df1783bba1c260d216c3406983d3fbe8ad3dcf6b04ba29a5 |
| lee__sf1plus__new__new__3-4.tsv | 74d4e55e8a749d8a094a1489aeb1b3f778d010d517f558372c63c3e4d15d1733 |
| lee__sf1plus__new__new__4-3.tsv | 0c0469b706695b47c06a956b2ab2a30acbf0b491a8d4635842091056586f017c |
| lee__sf1plus__new__old__1.tsv | b34a3fc8fd8143e4cad74f97c6d8f10c4c46f9e4289cf0a3ec4e93ffe7f3a486 |
| lee__sf1plus__new__old__2.tsv | 43e1718a5ad7face13ad9b505f141adf51a9b75f0b7bb4b892a08eb9b5661412 |
| lee__sf1plus__new__old__3.tsv | 26c098a0349a3c56cf3902c170ac733d51385db7830d2dfe394aace933db9a5d |
| lee__sf1plus__new__old__4.tsv | 0093683b312721e659109e2cfea96815c4ab1b66ff15fbf3c26d6929c80d0e49 |
| lee__sf1plus__old__new__1.tsv | 9cf7968fc0ab97a3ce1745eedf66ddffb202258bca8317f7de3b0895990a8c36 |
| lee__sf1plus__old__new__2.tsv | 5a0a0507f5e5aacc4fb55727040e742e9e72c40306a985b94bc3a69794351198 |
| lee__sf1plus__old__new__3.tsv | 85f9328d3b258f78314f95354bb2bea53879c25add8260c5ca80824f4db11f7e |
| lee__sf1plus__old__new__4.tsv | a36aa033c9db04038fa0e850b1cb394e28832f953ad9344e510022046968dd45 |
| lee__sf1plus__old__old__4.tsv | f783120a5a4e7026cb0c5bc09d1b9c89aead03a422ca0c528b7d649e8df4f43b |
| mike__arcade1987__new__new__1-2.tsv | f6db1d515e51536604380772878e19e146102874c0d0f92e3d2b5370dca41609 |
| mike__arcade1987__new__new__2-1.tsv | 572868ca201c73fce2521f293294e532a26f92f6d589a134dd9c37a55ea655f3 |
| mike__arcade1987__new__new__2-3.tsv | d3d3242db2b44e641138ac7a747f3ae86de4c84f34b6aea0c549c0d6e92ce1fd |
| mike__arcade1987__new__new__3-2.tsv | ec23d545e057ff81a112dfeab0cbc9f1dd9e64a3b8f03220ad610f4a45d78e74 |
| mike__arcade1987__new__new__3-4.tsv | c03c7bd4f18f722e8485975a9cf5830ecbfb647548512d8ed74f50529533aa4f |
| mike__arcade1987__new__new__4-3.tsv | 97363072e71808da5fe7421aaac1dd6a988682bc526934aaf1fb9a5e06e7e858 |
| mike__arcade1987__new__old__1.tsv | 17dc597957f59668c2ad95cc8655c50016ebd16a8b7e2baff99254b0dca04cd1 |
| mike__arcade1987__new__old__2.tsv | e83dc95f72ca86b78123504a6a088352781b66df6d44c4fe39b09c129a7d19bf |
| mike__arcade1987__new__old__3.tsv | 12bbc51023f3c3b4748ccbd97a08fe1658568fcc34b0cb3bf97a98e12f52c5f8 |
| mike__arcade1987__new__old__4.tsv | 1369a494dba452c22bb37ae38a11dc11a2a51e067c509ccaaee17776fdad5fe7 |
| mike__arcade1987__old__new__1.tsv | a28a243a429edc22bf608f796f3d05ad0b134979090d8133ca4a30572d4eb7bb |
| mike__arcade1987__old__new__2.tsv | 666991affc0c35c7f6605bc8996aaf2fbc32c80a39e1a3c09b289293cfba0169 |
| mike__arcade1987__old__new__3.tsv | cbfba36c419e3e737f720dd9d47c244c57ffe9885b40a2d6d88437f23a9b3947 |
| mike__arcade1987__old__new__4.tsv | 5aecbb1901cb02bcc284772322bc362a6f9d50f893bdb3c1cdaac40d3952e73f |
| mike__arcade1987__old__old__4.tsv | 00d64c910e86e6818fd2217bc0f0ea06fc3e4860cf8124ca61802712e5ac11ce |
| mike__sf1plus__new__new__1-2.tsv | 0f42f9c73ed8fc37374842a37f46f647f353543a2d94f4520d698e163f18ce66 |
| mike__sf1plus__new__new__2-1.tsv | 3725d58ee0a34efc06deb906f856e4d061ceae4c33580bd4beb3c433778ac156 |
| mike__sf1plus__new__new__2-3.tsv | cd3866e8903250f6112eb81b3ca4cc5b70e1e42d3817b1dbbac9bc1934ccf4ab |
| mike__sf1plus__new__new__3-2.tsv | 232eedcb6bb6edd314977e45ea311c1cd3e6922dc0844b66a361a0c39e0aea11 |
| mike__sf1plus__new__new__3-4.tsv | 61fb9386d7cdbf46d4baa7d2c80fe9f87abff64ee8eeec2f2c45933a3db25bf2 |
| mike__sf1plus__new__new__4-3.tsv | 7e5540da1a4506e63877102e7e2132163abd89666a636d583e9087dc4ea8b9ac |
| mike__sf1plus__new__old__1.tsv | d2cd00b31faf33add0cb9940a6246212fc1444133c218ae9ad8376b971b96859 |
| mike__sf1plus__new__old__2.tsv | 471304ca705a1f4ae01b120474ea8185cc97b2fd79b6307a9573091a3688049d |
| mike__sf1plus__new__old__3.tsv | ea460f28cec118c024b51b42c8d38677759c88783d377b6c5addbf3c499b86df |
| mike__sf1plus__new__old__4.tsv | 8d3ab6e7bba7ff3947f1a01b59f71b30d88b8493710cd586c41065e671d0735f |
| mike__sf1plus__old__new__1.tsv | 9e0264b5af8c90347bba42ee332f51f146a875e4cb84706ee9f3c9d092e32100 |
| mike__sf1plus__old__new__2.tsv | 76e4ae9e739b0a115c4db7853b215748a4d8d9bfb9a93d635d93c981196636e1 |
| mike__sf1plus__old__new__3.tsv | f0631bde45fd3f8a74156f92a6863863efbc7abe5ac655ddf43cdf69df346703 |
| mike__sf1plus__old__new__4.tsv | c94845197d697b60b46d44a7a6d8e1863830bd30a843f31931df87797e023ce2 |
| mike__sf1plus__old__old__4.tsv | f0bb661722f89f74d1765e316a832858cfe617e7cde729b2b98e8e0510191483 |
| retsu__arcade1987__new__new__1-2.tsv | 588e3d3bdf04c0eca425b3d430785786d9c1b5b0d72eb02a26b84fe5d87d6078 |
| retsu__arcade1987__new__new__2-1.tsv | 4d5c88ae3f151e8912f6ecb2df820406f2df3877d4457012c4a2b7a25ac5cedb |
| retsu__arcade1987__new__new__2-3.tsv | e9459fcb055a5872acf03ed941ca7d50331286466a6a4e45553c66e6726a70ba |
| retsu__arcade1987__new__new__3-2.tsv | b7303061f1789624ec61362d8c7380e1818d2e089ff7de27f8409c69e0b255ad |
| retsu__arcade1987__new__new__3-4.tsv | fd055c092c16457d76ee90ab0ecb9104856a12fdf0f0dcea4dbcd95c3d9c0ab4 |
| retsu__arcade1987__new__new__4-3.tsv | 7b46aa5402c67b15e1d2cf722ac20690fa402db9c4ecfda0871a67bf3322cb66 |
| retsu__arcade1987__new__old__1.tsv | b8f592268022b70bc51297732fda578d4b8bf021dd48e1d1315355fd1c300374 |
| retsu__arcade1987__new__old__2.tsv | 4e71d9d27abc5cdc428f4644e28d379ea64df8f9ad64de93890dfb2d0228da9b |
| retsu__arcade1987__new__old__3.tsv | b6dcb9349fafd4bf7e5ed0cf68ac0a9fb1e4e711753788a566cbaf2780b15e7f |
| retsu__arcade1987__new__old__4.tsv | 13c40eb8d57e3e10c52272e5a456dd17a6eb16028eba078566a5a78b4914b247 |
| retsu__arcade1987__old__new__1.tsv | 4202306ee9b2b2095b9dc1901b13f84e497a78c578b73e7f77aec1e1caf8693d |
| retsu__arcade1987__old__new__2.tsv | 1d4ac213be280451c09778862de76b3afe94cc37517e456a714e84e2ac6175f4 |
| retsu__arcade1987__old__new__3.tsv | 07fc8cc588184fa55c36d282ffcd61a127b4563a0c46cd6f28c5e16fbd045165 |
| retsu__arcade1987__old__new__4.tsv | 18c4b6288196bd84b77d5c5ed5434ba657f663bd251c5db3772b8d97257ff67e |
| retsu__arcade1987__old__old__4.tsv | db45501920e14b730403aa711f4bafd715332357f1578b383afdce97a4315350 |
| retsu__sf1plus__new__new__1-2.tsv | cd83d0c3eedb31a0086a4c9111022692662fd98eff3c9fb74d55e50c6c5d8f5a |
| retsu__sf1plus__new__new__2-1.tsv | a4e7512df57890a50ece47df0f7a1fc1380582a4e99f839cb05043bfcfb1baf0 |
| retsu__sf1plus__new__new__2-3.tsv | b111d5ef3f3e8204ab0278d61540cb2d3e01366551d1bd6787782b48e4257051 |
| retsu__sf1plus__new__new__3-2.tsv | 71ec21d5f1fb079e21cdb94d589b56587d6849570426193f5cb3f2b7febc7ed8 |
| retsu__sf1plus__new__new__3-4.tsv | d79d7c1706c9ed08bb5e46331dd0483043673d8489aff480b01f6c33305ab204 |
| retsu__sf1plus__new__new__4-3.tsv | a9cdcff04d3d03cee01726a28ef470edd37931dac8b93387fcb09114bdeba19b |
| retsu__sf1plus__new__old__1.tsv | b3b97fb5955e20f13fe43cc1307443c66e85c49dd3cd7471dcb80ae2382a4fef |
| retsu__sf1plus__new__old__2.tsv | 2705f7043ae22f00de22a284ecfafb299d69c09cca37197f8aa3a1d2c71685a8 |
| retsu__sf1plus__new__old__3.tsv | bb4434ed422d001d873f91442c893e5b155dfe6c7c67ff2b8083069a8d5e69fd |
| retsu__sf1plus__new__old__4.tsv | d387f05cf7cb4c63cb9ecad2c8cf42ce251ccdb75b65f30d23b1861da2e24636 |
| retsu__sf1plus__old__new__1.tsv | b18a0f077a5ffa71b036af60c16dbd40a8f52f8aef165b547bdbaa619bca54f5 |
| retsu__sf1plus__old__new__2.tsv | 8921a9b30377080b12bae5b927b4520af39538c1a6aa3776136eec2df44532c6 |
| retsu__sf1plus__old__new__3.tsv | 049cad93bb0b2e6d69e1819128d087524d79e610f1e9cd8a9197219d238f7fd5 |
| retsu__sf1plus__old__new__4.tsv | e980465d9571044cb280613505fc524ffd075c9388d40f034f235397b853940c |
| retsu__sf1plus__old__old__4.tsv | 1475b80e24930d7e7c4f9e0933f40e444681ab718fcae66a93304012752213e0 |
| ryu__arcade1987__new__new__1-2.tsv | e6666fb74f798af83f9c895f3986cd57a2c9e459436a156653c76006208f597e |
| ryu__arcade1987__new__new__2-1.tsv | 1accab160d690cd7711e2fa1b73f6582b14b0aaf730639de58396b0cd80dd7eb |
| ryu__arcade1987__new__new__2-3.tsv | 4ea7b184e1d3a06535127247f7b2cd192ab70e40373e5386d6874e91dbd0424c |
| ryu__arcade1987__new__new__3-2.tsv | 0d276c7b5828115f8bee32fa44d20fec4741068b4e5374c3f590929a95f6c24e |
| ryu__arcade1987__new__new__3-4.tsv | 4178e9c04afc998e825a3cda959aa697f8315aa174bc18318d57ea174d8349f1 |
| ryu__arcade1987__new__new__4-3.tsv | edd326608cfaad594d1292023ca56aacee484381b1de195604c1ee2dd104783e |
| ryu__arcade1987__new__old__1.tsv | 1fa8163e8fb13abfa88b2f2cabc7a45b61d90dba2e8f9448d854516705c47a26 |
| ryu__arcade1987__new__old__2.tsv | 00f09bc294ef5f917eb6a5f32fe971935b64a8897cc48128d9dc9d5731af3cd2 |
| ryu__arcade1987__new__old__3.tsv | 5bfca2a9e894c15774fda0b646cf63a7ae5c510477d8232a8eab10f99a090e9f |
| ryu__arcade1987__new__old__4.tsv | 101a109ff19e0658b82f46fb388e65d898fa04c12a7329e05efaaf457cfd5896 |
| ryu__arcade1987__old__new__1.tsv | f0deaf2ea6999e0b2047629497cf459eb310bb19b5db0b42a9f6e399949e9cf4 |
| ryu__arcade1987__old__new__2.tsv | c54b50c0878e790fee4394b8170281e1332b95191e60550e206691e0d92c6842 |
| ryu__arcade1987__old__new__3.tsv | 5215428c183b577e35f098cbffd5354d974a84dccbc4eadb4324b4d3541e4127 |
| ryu__arcade1987__old__new__4.tsv | f27641cc13a4f1a5eaad3e0abbf849945d3372feb030c3cf9c23a85b7e7827f8 |
| ryu__arcade1987__old__old__4.tsv | 51788ddc1805181bffe16ec2936d4662ecc469342b6f2d49fa1e635665746485 |
| ryu__sf1plus__new__new__1-2.tsv | 6248ebbf39d3d92b6295de8dd17e0d0f69517b72b5de98e0a7d80463d10c23e4 |
| ryu__sf1plus__new__new__2-1.tsv | 5bd865e7005e84f48938a9ecfa831498960415b4f3b0257909d04f6ad5697f1f |
| ryu__sf1plus__new__new__2-3.tsv | 4e760065098411166279d813d9be047ec2d31cc070a23e1d1bf952216812a85c |
| ryu__sf1plus__new__new__3-2.tsv | 8a129ca51b5dc802f876de337547b749d3287219617d47cab003837c3957e412 |
| ryu__sf1plus__new__new__3-4.tsv | b8c2776093cf75ab7a4d84239a3d7d331b5ce45f9b3711d5ec80387274f54bbd |
| ryu__sf1plus__new__new__4-3.tsv | 3e516963c57fe9fd7413ed6416fa884c9923ddf45f22cac087de3ca882cd1964 |
| ryu__sf1plus__new__old__1.tsv | 9bd7752b10356a9ddb43f42a39a78b1da39cdebb8de2bb52510d9f35d688bc77 |
| ryu__sf1plus__new__old__2.tsv | 047df520d94e734753bcf0daf61f4b0ae433913a783c5637ba32b2d0e92fdc44 |
| ryu__sf1plus__new__old__3.tsv | ffc096c32e3894a05677a492de2d237d73bad39d05fc54aded6d62a7f989da21 |
| ryu__sf1plus__new__old__4.tsv | f740d9777023fae4a3ae9e895b8e44524f0b0eef58090197ad657876fb9435e8 |
| ryu__sf1plus__old__new__1.tsv | f40c1ccb012ab523c0c1ceb4fddef4ec8940ed75999de800c440b08b60e8b544 |
| ryu__sf1plus__old__new__2.tsv | 6f1dcddb255795c4d8ab97788ce6058da9da799d99c8bde69ffd0465e5072662 |
| ryu__sf1plus__old__new__3.tsv | 3556178026b4d08dc68b86f24a62583761f528ea99ee9ee3b7425c6284189d49 |
| ryu__sf1plus__old__new__4.tsv | c46454302c36d73fb1aefcd9d6c1b5820ebc30b3bf9a879e095b81d07ef7ac2b |
| ryu__sf1plus__old__old__4.tsv | b197d984efccdbeade258876f01c17d940ae1abe92929af2dbd710ad7cc34cb6 |
| sagat__arcade1987__new__new__1-2.tsv | f1f7cdf08512b53a519f3d72316e747858d51202abc9524e9da8802637a1d46c |
| sagat__arcade1987__new__new__2-1.tsv | b238c2fc6ad1ca131326940eda0b993ae322cacda4f2456c4031763f35a2cc36 |
| sagat__arcade1987__new__new__2-3.tsv | 892c2ca12ef44955cc4ed2e96ea0c873f65140da0d029056a7078d269db24c09 |
| sagat__arcade1987__new__new__3-2.tsv | afda8d83c3bd1a7235d3171ec600460f8546b8a3ec9cbc1b35a5933ef1f56cc7 |
| sagat__arcade1987__new__new__3-4.tsv | 6792d12b850a36b60c067de36da447268402bdefd630d394cc633f15b34c6362 |
| sagat__arcade1987__new__new__4-3.tsv | df615fb457b79e8187d3c722942e859acc6b8371602e31464b15c451a36a7b5b |
| sagat__arcade1987__new__old__1.tsv | e297a9b78230f4f24a5d49deec44883f605844c6486030d10d227f0c41e403cb |
| sagat__arcade1987__new__old__2.tsv | 458995952de7782316b6e24ce7a047d1871a505b3e91b6fd5ac100890f43a368 |
| sagat__arcade1987__new__old__3.tsv | bb6de512751c36b1b40320ad9993cac63bdfbc2ca1a552c4c4966d1b8c46fd9a |
| sagat__arcade1987__new__old__4.tsv | 9e5dda5b980173fb64ee39bf560960eda44a9f867730d01f5f8a4d22e7c53216 |
| sagat__arcade1987__old__new__1.tsv | 4f4ce8f472cdaa28e8e352c4934f95df27be615fe2cc0d5d09be589f02943578 |
| sagat__arcade1987__old__new__2.tsv | 29f55f6108208ec365bb1048231ff41d6f43cb9be83d46f0bc02d00ab4df1328 |
| sagat__arcade1987__old__new__3.tsv | 551aba5bbef8120468c4a753c9db1587a722909c2f5360d084e3cf9470adcc41 |
| sagat__arcade1987__old__new__4.tsv | f8f1782c490264df496082661847744b5afd463650fbf99abd2570a257932cf9 |
| sagat__arcade1987__old__old__4.tsv | e5f4f350aa2a399fb0be106ea5f1ea7984f67f84849672eda122890cb2fe14fd |
| sagat__sf1plus__new__new__1-2.tsv | c594de3a083bf4f605efdbcd5c0cc45f48023ec35a94e90c3014a525eb852da0 |
| sagat__sf1plus__new__new__2-1.tsv | 40d0078c9f7de18388106c3221a6d9ba1d471360565f6e9a77a49179086e68d6 |
| sagat__sf1plus__new__new__2-3.tsv | ce4c3aa1a84994152b38c8c58d1f1fac6e472c098ee59cb0d636e103037210bd |
| sagat__sf1plus__new__new__3-2.tsv | e46bb7de1e1135853694ea9e80ba7ef463d7c7b1b2ae6b716591db09292cf7f8 |
| sagat__sf1plus__new__new__3-4.tsv | 7cd2010dad79e9c34799286abc2136060c120185894d17daa65051e3e452e5c7 |
| sagat__sf1plus__new__new__4-3.tsv | 4d6d925573b93002bea157bd372ad02f33f2aa8706c65653a1dba6f6d4bf95e4 |
| sagat__sf1plus__new__old__1.tsv | 3c2f0f03ec02b7a0601468d8567f35c218080f534ab1a87a5acb8a110d1c6de6 |
| sagat__sf1plus__new__old__2.tsv | 4b6d0501cf3ee4770e113d58034de5195510e8e99d632a2d18102eb5f2f02358 |
| sagat__sf1plus__new__old__3.tsv | 35789905362cd2682e98d74123f05d2e48cbada7c2d704f91d6f5b786be74198 |
| sagat__sf1plus__new__old__4.tsv | 9b8d88d0358731e7f807060c580ccdf1d1ee5a8d9133248e76d2f368c1bbbbf1 |
| sagat__sf1plus__old__new__1.tsv | 06485a51409bd73d519f2fb2b77e7710872eddb1970de1b91f77f287bde6e49c |
| sagat__sf1plus__old__new__2.tsv | fd333e703d8b5c5f2f83efe93871688c21ad9102ed2885de4d03480926a1a214 |
| sagat__sf1plus__old__new__3.tsv | 1033dafa7e403770e9079fa3e5f741941dab4ad706e404b20bd128523c8b53bc |
| sagat__sf1plus__old__new__4.tsv | da300ef753692fdd1c46f84c38c25a8da1b6b29c005d5a77045acc488e7cddeb |
| sagat__sf1plus__old__old__4.tsv | e09caf07e08e9f633e7ccd9ec8552e8b02cc7c2bc51ea7e9e1128be64e4de3b4 |

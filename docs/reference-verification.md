# Reference ROM verification

On 2026-10-15, all 44 entries required by MAME's `ROM_START(sf)` definition for Street Fighter US set 1 were verified in the local `sf.7z` archive. Each entry matched its expected byte size, CRC32, and SHA1; there were zero missing entries and zero mismatches.

Source: [MAME sf.cpp at commit 774a180df2b6ef9c53e48337792256817c50f768](https://github.com/mamedev/mame/blob/774a180df2b6ef9c53e48337792256817c50f768/src/mame/capcom/sf.cpp).

Archive SHA256: `cda57772ef1dab11684e594929a742410cda9ca968f1389d0d75ee71c161c1b5`.

Each required root archive entry was streamed through 7-Zip into Python memory and hashed. No extracted ROM files were persisted, and no ROMs were executed. Other variants in the archive remain unverified.

MAME was not found on PATH, in the repository, or at `C:/Program Files/MAME/mame.exe` or `C:/mame/mame.exe`. This was a bounded search, not a systemwide inventory. The source commit above pins the ROM definitions used for verification; a runnable MAME version for behavior comparisons remains to be selected and pinned.

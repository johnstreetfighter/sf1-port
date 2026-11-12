# SF1+ voice pack

The Ringside announcer's generated lines are 4-bit MSM5205 ADPCM at 4 kHz. `manifest.json` records each line's nibble count and CRC-32. The Windows ZIP contains the approved lines and this manifest.

The repository keeps generated voice design previews and takes in `archive/` for the pipeline under `tools/voice/`. The pipeline can generate its ROM comparison clip and listening sheet locally. The line prompts, selected takes and credit ledger live under `tools/voice/`.

The pack was imported from [sf1-port-voice at c945a773](https://github.com/johnstreetfighter/sf1-port-voice/tree/c945a773898533ff372f51e5b45e28c342d730f0).

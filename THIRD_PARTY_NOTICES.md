# Third-party notices

The game's source is this repository's own. The software below is included in
it, in source form, and is compiled into the game when the `tracker` cargo
feature is on (the default; see the README's "Tracker modules and the C++
build"). All of it sits in `crates/sf-openmpt/vendor/libopenmpt`, which is an
unmodified subset of the official libopenmpt 0.8.10 release
(`libopenmpt-0.8.10+release.makefile.tar.gz` from https://lib.openmpt.org/,
SHA-256 `95073a10d33d0ed4352f41b68edccb9636002d2626f9fd2e00b191fcaabdd876`):
the directories the library's makefile compiles, left as they came. The one
file added is `include/stb_vorbis/LICENSE`, which is stb_vorbis.c's own
licence text copied from the end of that file.

Other dependencies (symphonia, librashader and the rest) are Rust crates
under their own licences, which Cargo records with each; they are not
repeated here.

| Software | Where | Licence |
| --- | --- | --- |
| libopenmpt 0.8.10 | `vendor/libopenmpt/{common,soundlib,sounddsp,libopenmpt}` | BSD-3-Clause |
| mpt headers (part of the same release) | `vendor/libopenmpt/src/mpt` | BSL-1.0 OR BSD-3-Clause (taken here under BSD-3-Clause) |
| miniz 3.1.2 | `vendor/libopenmpt/include/miniz` | MIT |
| minimp3 | `vendor/libopenmpt/include/minimp3` | CC0-1.0 |
| stb_vorbis 1.22 | `vendor/libopenmpt/include/stb_vorbis` | MIT OR Unlicense (public domain) |

## libopenmpt (BSD-3-Clause)

```text
Copyright (c) 2004-2026, OpenMPT Project Developers and Contributors
Copyright (c) 1997-2003, Olivier Lapicque
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of the OpenMPT project nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## The mpt headers (BSD-3-Clause, as taken here)

`src/mpt` is offered under the Boost Software License 1.0 or the BSD
3-clause licence (`src/mpt/LICENSE.BSL-1.0.txt` and
`src/mpt/LICENSE.BSD-3-Clause.txt`). The BSD 3-clause text:

```text
Copyright (c) 2004-2026, OpenMPT Project Developers and Contributors
Copyright (c) 1997-2003, Olivier Lapicque
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:
    * Redistributions of source code must retain the above copyright
      notice, this list of conditions and the following disclaimer.
    * Redistributions in binary form must reproduce the above copyright
      notice, this list of conditions and the following disclaimer in the
      documentation and/or other materials provided with the distribution.
    * Neither the name of the OpenMPT project nor the
      names of its contributors may be used to endorse or promote products
      derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

## miniz (MIT)

```text
Copyright 2013-2014 RAD Game Tools and Valve Software
Copyright 2010-2014 Rich Geldreich and Tenacious Software LLC

All Rights Reserved.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

## minimp3 (CC0-1.0)

minimp3 is dedicated to the public domain under CC0 1.0 Universal; its full
legal text is `crates/sf-openmpt/vendor/libopenmpt/include/minimp3/LICENSE`.
No notice is required.

## stb_vorbis (MIT OR Unlicense)

stb_vorbis is available under the MIT licence or as public domain (the
Unlicense), whichever is preferred; the two alternatives are in
`crates/sf-openmpt/vendor/libopenmpt/include/stb_vorbis/LICENSE`. It is taken
here under the Unlicense (public domain), which needs no notice; the MIT
alternative's notice, Copyright (c) 2017 Sean Barrett, is there should it be
wanted.

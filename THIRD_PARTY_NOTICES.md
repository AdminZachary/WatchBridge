# WatchBridge — sources and attribution

WatchBridge is an independent community derivative. It is not affiliated with or
endorsed by Apple, nab138, AndreySemjonov, Rzbck, or the xtool maintainers.

## Desktop application and signing

- Original iLoader by nab138: https://github.com/nab138/iloader
- Immediate fork by AndreySemjonov: https://github.com/AndreySemjonov/iloader
- Starting commit: `2d13485ef2d7107b2bc2ec6cdb5f54ce5729ef70` (v2.3.4-fork.2).
- License: MIT. The original `LICENSE`, `LICENSE-BRANDING`, source history and
  upstream notices are retained. WatchBridge has a separate name, icon,
  application identifier and credential-store service.
- The embedded modified isideload source and its MIT license remain in `vendor/`.
  See `vendor/isideload/PROVENANCE.md` for its upstream revision, modifications,
  and Watch companion contributions by Data_C0re_ (Rzbck).
- Rzbck's architecture and hardware investigation:
  https://github.com/Rzbck/iloader-watch-companion

## Watch developer-mode preparation

- Workflow reference: oacevedo/xtool, branch `feat/experimental-watchos`, commit
  `c46cd6a323d61f693ebe7354f5dcd91918e21f36`:
  https://github.com/oacevedo/xtool/tree/c46cd6a323d61f693ebe7354f5dcd91918e21f36/tools/watchos
- xtool copyright (c) 2024 Kabir Oberai. MIT license reproduced below.
- `src-tauri/src/watch_setup.rs` is a native Rust implementation using the
  existing idevice crate: companion forwarding, Watch trust, status checks and
  AMFI action 0. It does not include or execute Python or pymobiledevice3 code.
- The reference prototype uses pymobiledevice3, a GPLv3 project by doronz88 and
  contributors: https://github.com/doronz88/pymobiledevice3 . It is acknowledged
  as part of the reference workflow and is not bundled with WatchBridge.

Existing dependencies (including idevice and signing libraries) retain their
own licenses and notices. The WatchBridge mark in `public/watchbridge.svg` and
current desktop icons were created for this fork.

## xtool MIT license

MIT License

Copyright (c) 2024 Kabir Oberai

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

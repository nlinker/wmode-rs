# Sound on focus loss: post-mortem

Date: 2026-11-23. Scope: wmode-rs step 12 work session. All names, addresses and flag
values below are from the actual session (crash dialogs, `%TEMP%\wmode-rs.log`, PE import
walks via dumpbin).

## Symptom

With the plugin active, the game sound disappears the moment the StarCraft window loses
focus (click another window, Alt+Tab). The user observed the original W-MODE keeping the
sound playing in the same scenario, so this read as a regression against upstream.

## Root cause

The plugin bootstrap sets the storm registry flag
`HKCU\Software\Blizzard Entertainment\Internal\Window Mode = 1`, so storm takes its windowed
path. In that path storm creates its DirectSound object with the cooperative level
`DSSCL_NORMAL`. DirectSound semantics: every secondary buffer that lacks
`DSBCAPS_GLOBALFOCUS` (0x8000) or `DSBCAPS_STICKYFOCUS` (0x4000) is silenced by the system
when the window that owns the sound device deactivates. Nothing in our code mutes anything;
the muting happens inside DirectSound/sysaudio.

How the sound reaches the device in the first place: neither storm.dll nor StarCraft.exe
imports dsound.dll (verified with dumpbin: storm imports KERNEL32, USER32, GDI32, ADVAPI32,
VERSION, comdlg32; the exe imports storm and system DLLs only). The only sound-related
string in storm is the smackw32 export `_SmackSoundUseDirectSound@4`. So dsound.dll is
loaded dynamically via `LoadLibraryA` + `GetProcAddress` at game init - which matters for
the interception design below.

Upstream evidence is contradictory:

- v1.01 changelog (2007): "Now the game sound isn't muted when the StarCraft window is not
  focused."
- v1.02 known limitations: "W-MODE has nothing to do with game music/sound (yet). When you
  switch from your StarCraft window to another all StarCraft sound mutes. It can be worked
  around, maybe in the future..."

v1.02 is the version being rewritten, so v1.02 muting is upstream behavior and parity with
it is acceptable. (The user's observation of the original keeping sound may belong to a
different build or configuration; not resolved here.)

## Attempted fix: a COM wrapper granting DSBCAPS_GLOBALFOCUS

### Design

The flag `DSBCAPS_GLOBALFOCUS` can only be granted at buffer creation time: it lives in the
`DSBUFFERDESC.dwFlags` the caller passes to `IDirectSound::CreateSoundBuffer`. The caller is
storm, so the only interception point short of inline hooks is the object creation itself.

The interception rides on hooks that already existed for the fake DirectDraw: storm's
kernel32 `GetProcAddress` import is rerouted to our `get_proc_address`, and storm's
`LoadLibraryA` import wraps every library storm loads (patching the child's own
GetProcAddress import as well, which covers smackw32). The wrapper answered
`GetProcAddress(dsound, "DirectSoundCreate")` with a factory that:

1. called the real `DirectSoundCreate` export;
2. returned a COM wrapper object instead of the real one.

The wrapper was a two-field record `{ vtbl, real }` (the COM contract: the first field is
the vtable pointer, the second kept the real `IDirectSound` for forwarding). Its 9-slot
vtable followed the SDK dsound.h slot order:

| slot | method | arity (incl. this) |
|------|--------|--------------------|
| 0 | QueryInterface | 3 |
| 1 | AddRef | 1 |
| 2 | Release | 1 |
| 3 | CreateSoundBuffer | 4 |
| 4 | GetCaps | 2 |
| 5 | Compact | 1 |
| 6 | SetCooperativeLevel | 3 |
| 7 | GetSpeakerConfig | 2 |
| 8 | Initialize | 2 |

Every slot forwarded to the same-index slot of the real object's vtable (one helper per
arity, stdcall via `extern "system"`). The one active slot, `CreateSoundBuffer`, first
copied the descriptor to the stack, OR-ed `DSBCAPS_GLOBALFOCUS` into `dwFlags` (offset 4)
for secondary buffers only (`DSBCAPS_PRIMARYBUFFER` = 0x1 present means primary, and the
flag is invalid there), and passed the copy to the real call. No cooperative level was
forced: the flag alone covers the muting, and forcing `DSSCL_EXCLUSIVE` would additionally
silence every other application while the game is focused.

### Crash 1: wrong function pointer captured (fixed)

The factory initially stored the value handed to it by the GetProcAddress hook - which was
the kernel32 `GetProcAddress` address itself, not the resolved `DirectSoundCreate`. Called
as DirectSoundCreate it returned 0, the wrapper treated that as success without writing the
out parameter, and storm called through an uninitialized pointer. Result: instant crash at
startup, dialog "Инструкция по адресу 0x00000000001AFE04 обратилась к памяти по адресу
0x00000000001AFE04" - the self-same address signature of a jump into garbage (DEP kill).

Log fingerprint of this failure: `sound: real DirectSoundCreate hr 0x00000000` followed by
no wrapper line and no buffer activity.

Fix: the hook passes the genuinely resolved export (`call_real_get_proc` result) and the
factory refuses to report success on a null object.

### Crash 2: read-only descriptor (fixed)

With creation fixed, the first `CreateSoundBuffer` faulted: "Память не может быть written".
The patch originally wrote `dwFlags` in place - and the game builds its `DSBUFFERDESC` in
read-only memory (static const descriptors). Fix: copy `dwSize` bytes (20 or 24 accepted)
to a stack buffer, patch the copy, pass the copy to the real call. The original descriptor
is never written.

### Crash 3: garbage call during replay (undiagnosed, final)

The log then showed clean operation end to end:

- both storm and smackw32 received wrapped objects (`DirectSoundCreate` asked and
  intercepted twice);
- 15+ buffers patched: `0xc2 -> 0x80c2` and `0x80 -> 0x8080` (per dsound.h: 0xc2 =
  CTRLFREQUENCY | LOCSOFTWARE | CTRL3D, 0x80 = CTRL3D; the game does use DS3D buffers);
- menu sounds and a replay ran normally for a while.

Then, during replay playback (~2 s in, "Replay Progress" visible in the screenshot):
"Инструкция по адресу 0x000000000044CD488 обратилась к памяти по адресу 0x000000000044CD488"
- again the self-same address signature, a call through a garbage pointer into unmapped
memory. Nothing at all was logged between the last buffer creation and the fault, so the
fatal call left no trace.

Leading suspects, undifferentiated:

- the second wrapper instance: smackw32 creates its own DirectSound through the same
  factory and may drive the object differently (or release and re-query it);
- a late vtable call on a wrapped object from the replay sound path (slot beyond those
  exercised during menu playback);
- an arity or slot-order subtlety surfacing only on a rarely taken call.

Without a crash dump the candidates could not be told apart. Debugging a stdcall COM shim
inside a live game by trial and error was judged too costly for the benefit, and the whole
interception was reverted.

## Outcome

- The ddraw.rs dsound branch and the wrapper machinery were removed; `src/sound.rs` was
  deleted outright.
- The `MuteNotFocused` ini key this step originally carried was dropped with it: under OS
  muting it produced no audible difference, and the key does not exist in the upstream
  readme. (MaxFps is also absent upstream but stays - it is the frame throttle feature
  itself.)
- Consequence: game sound mutes while the window is unfocused, matching v1.02 as
  documented.

Alternatives considered and not taken:

| approach | why rejected |
|----------|--------------|
| force `DSSCL_EXCLUSIVE` in the wrapper's SetCooperativeLevel | same wrapper risk; also silences every other app while the game is focused |
| inline hook of `DirectSoundCreate` inside dsound.dll | trampolines, ABI patching, strictly more invasive than IAT-level redirection |
| proxy dsound.dll next to StarCraft.exe (cnc-ddraw style) | adds a deployed component; the project ships a single plugin file |
| private primary-buffer volume mute (the old sound.rs) | redundant: the OS already mutes the buffers on focus loss |

## If this is ever retried

- Log every forwarded call (budgeted per slot), not only buffer creation: crash 3 proved
  that "creation logs only" leave the fatal call invisible.
- Verify the wrapper against a minimal DirectSound test host before pointing it at the
  game, so crashes localize to wrapper-vs-game differences instead of wrapper bugs.
- Consider intercepting only `CreateSoundBuffer` (per-buffer wrapper) instead of the whole
  DirectSound object: the buffer interface is smaller, and the game never calls back into
  the DirectSound object after setup.
- Capture a crash dump (WER local dumps) in the first test run, so the faulting call site
  is identifiable.

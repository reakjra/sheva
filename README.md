# sheva

**S**top **H**aptics **E**mulation, **V**ia **A**udio.

DualSense voice-coil haptics over bluetooth on Linux. games only send haptics to a DualSense plugged in by USB (at least TLOU2, so imagine only this game does it lmao), so over bluetooth you normally get plain rumble. this makes the bluetooth controller look like a USB one and forwards the haptics the game sends

\- `sheva bridge` (root): creates a virtual USB DualSense with uhid and bridges it to the real one over bluetooth

\- `sheva sink` (user): a PipeWire sink that looks like the DualSense' own audio device, giving the haptics to the bridge

the real bluetooth pad gets hidden from games by a udev rule so they only see the virtual one since otherwise they'd see two pads and it'd get dumb about choosing one

## install

Rust, PipeWire (if you dont have pipewire you're wild)

```sh
make install # hard work

make uninstall # to delete everything

sudo sheva disable # disables everything, hence back to normal bt behaviour
sudo sheva enable 
```

Oh also I don't know if Edge version changes anything from the normal version.

## license

GPL-3.0-or-later

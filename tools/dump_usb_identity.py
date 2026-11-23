import fcntl
import json
import os
import sys
from pathlib import Path

FEATURE_IDS = [0x05, 0x09, 0x20]
FEATURE_BUFFER = 64


def hidiocgfeature(length):
    return (3 << 30) | (length << 16) | (ord("H") << 8) | 0x07


def find_usb_dualsense():
    for node in sorted(Path("/sys/class/hidraw").iterdir()):
        device = node / "device"
        uevent = (device / "uevent").read_text()
        real = "/devices/virtual/" not in str(device.resolve())
        if real and "DualSense" in uevent and "HID_ID=0003:" in uevent:
            return node
    sys.exit("no USB DualSense found, is the cable in?")


def main():
    node = find_usb_dualsense()
    descriptor = (node / "device" / "report_descriptor").read_bytes()
    features = {}
    fd = os.open(Path("/dev") / node.name, os.O_RDWR)
    try:
        for report_id in FEATURE_IDS:
            buf = bytearray(FEATURE_BUFFER)
            buf[0] = report_id
            length = fcntl.ioctl(fd, hidiocgfeature(len(buf)), buf)
            features[f"{report_id:#04x}"] = buf[:length].hex()
    finally:
        os.close(fd)
    print(json.dumps({"descriptor": descriptor.hex(), "features": features}, indent=2))
    sizes = ", ".join(f"{k}={len(v) // 2}B" for k, v in features.items())
    print(f"{node.name}: descriptor {len(descriptor)} bytes, features {sizes}", file=sys.stderr)


if __name__ == "__main__":
    main()

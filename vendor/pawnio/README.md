# PawnIO modules

The CPU temperature modules below are the unmodified signed modules from PawnIO.Modules 0.2.11.
Copyright (C) 2025 namazso <admin@namazso.eu>; ZhaoxinMSR Copyright (C) 2026 Gen Li.
LGPL-2.1-or-later; see COPYING.

Release: https://github.com/namazso/PawnIO.Modules/releases/tag/0.2.11
Binary archive: https://github.com/namazso/PawnIO.Modules/releases/download/0.2.11/release_0_2_11.zip
Corresponding module source archive: source/PawnIO.Modules-0.2.11.zip
Source includes its build instructions; PawnIO compiler/SDK: https://github.com/namazso/PawnIO

| Module | CPUs | SHA-256 |
|---|---|---|
| AMDFamily17.bin | AMD families 17h–1Ah (Zen) | DAE74615761B78BDF064DFB3E136252DDCC6FC727D88F14738D0E5800D427A91 |
| AMDFamily10.bin | AMD families 10h–16h | 6443080B2968474FFBC38AA4356CC56F9664349FA4B917AFDB33027D6BB50525 |
| AMDFamily0F.bin | AMD family 0Fh (K8) | A6E11619E87A97820705A6523714F22D676CE44F902631833D4429B89D509D55 |
| IntelMSR.bin | Intel | D6ED85D65AB17A22F813EF98207D6D537155EE2DED5976A21CB48413C9B92E5F |
| ZhaoxinMSR.bin | VIA family 7 (Zhaoxin) | 4B56103CA456517EC1A553731F425905E1DF9B825DB232A19DAFCFD98C944B61 |

The modules are external, replaceable files that Taskbar Metrics never modifies. Setup installs
them, with the source archive, beside the collector; the portable executable carries both and
writes them out as the same separate files.
The driver and PawnIOLib.dll are NOT redistributed here. Install the official distribution
from https://pawnio.eu/. Only the installed library is loaded dynamically.
Taskbar Metrics only calls read ioctls: ioctl_read_smn, ioctl_read_smu, ioctl_read_miscctl
and ioctl_get_thermtrip under Global\Access_PCI, and ioctl_read_msr. It never calls
write/tuning ioctls.

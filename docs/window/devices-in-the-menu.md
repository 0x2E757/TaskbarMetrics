# Devices in the menu

Part of [History window](../window.md).

- Each device is a separate menu item, in this order: CPU, graphics cards, RAM, drives, network
  adapters. The window re-enumerates the devices every 10 s and keeps the shown one if it is
  still there. Drives are named like graphics cards: type and number,
  "SSD 1", "HDD", "SSD 2". The number is counted separately per group (graphics cards, SSD, HDD,
  drives of unknown type — "Disk"), from 1, in the order of Windows numbers; the only
  device in a group gets no number. On the taskbar tiles the number comes from the id
  (`gpu@1` → `GPU2`).
- The drive type comes from the seek penalty flag (`IOCTL_STORAGE_QUERY_PROPERTY` on
  `\\.\PhysicalDriveN`, no elevation). All volume letters of a drive are at the end of the
  subtitle, the way File Explorer names them: "NVMe · Drives C:, D:", and for a single one —
  "Drive C:" (the Russian UI translates these). No `\`: in Segoe UI it drops below the line.
- Network drives (letters mapped to SMB shares) come after the local ones as "Network drive",
  numbered like the others when there are several. The subtitle is the share and the letter:
  "\\\\nas\\media · Drive Z:". Their table shows no processes: Windows sends their traffic
  through the System process, and the page says so in its place. Sources:
  [metrics.md](../metrics.md#disk).
- The RAM subtitle lists the installed modules: "2 × 16 GB DDR5-6000" (identical ones
  are counted, different ones are joined with " + "). It is read from SMBIOS (`GetSystemFirmwareTable`,
  Memory Device structures) without elevation; the manufacturer and part number are not shown —
  firmware often writes `Unknown` or a generic name in their place. A virtual machine lists its
  memory as power-of-two pieces without a type or speed (Hyper‑V: 8 GB + 2 GB + … + 32 MB);
  when no device has either, the subtitle is their total, "11.9 GB".
- Network adapters: only hardware Ethernet, Wi‑Fi and WWAN adapters that are up (Hyper‑V, VPN
  and other virtual adapters are left out). The item is named as in Windows Settings ("Wi‑Fi",
  "Ethernet 2"); the subtitle is the adapter description.
- Each device's page has two check boxes:
  - "Show on taskbar" adds or removes the device's tile (`metrics=`). The last tile cannot be
    removed.
  - "Always monitor" keeps the device's history while the window is closed (`history=`).
    A device with a tile is always monitored, so the box is checked and disabled.

  The window writes the config at once and rereads it before each change, so edits made
  elsewhere count. Keys: [configuration.md](../configuration.md).

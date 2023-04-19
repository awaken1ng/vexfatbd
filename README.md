# Virtual exFAT block device

Emulates exFAT file system in memory on block level

At the time of writing:
- Read-only, no writing support
- Can map files from host file system (only one file for now)
- No file metadata; create, last modified and last accessed timestamps are zeroed
- Max emulated capacity is a little bit under 4 TiB
- 🍝

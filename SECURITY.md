# Security

Taskbar Metrics loads a DLL into Explorer, and its collectors run with administrator
rights: the history recorder for ETW and the CPU temperature sensor for the PawnIO driver.
An installed copy starts them through Task Scheduler tasks that only elevate binaries in
Program Files ([installer.md](docs/building/installer.md)).

## Reporting a vulnerability

Report it privately through **Security → Report a vulnerability** on the GitHub repository,
not in a public issue. Include the version, the steps to reproduce and what an attacker gains.

Of most interest: a way for an unelevated process to make a collector run code or write files
with administrator rights, to change what the scheduled tasks start, or to crash or take over
Explorer through the DLL.

## Supported versions

Only the latest release gets fixes.

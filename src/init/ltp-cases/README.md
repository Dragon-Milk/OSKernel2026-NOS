# LTP Batches

Generated from `src/init/ltp-cases/*.txt`.

Each batch preserves its source list order and contains at most 30 cases.

`net` is the default quick network category and is intentionally an alias of `net-core`.
`net-all` is an aggregate full network smoke category that runs `net-core`, `net-script`,
then `net-deferred` in order. Script-driven and remote/special-environment network
cases are also available through explicit categories.

## Network Split

| Network category | Cases | Reason |
| --- | ---: | --- |
| `net-core` / `net` | 52 | Standalone C/ELF socket syscall tests kept in the default network batch. |
| `net-all` | 589 | Aggregate full network baseline: `net` + `net-script` + `net-deferred`, with no separate mixed batch files. |
| `net-script` | 438 | Shell-driven cases requiring LTP shell helpers, coreutils, cwd, or PATH setup. |
| `net-deferred` | 99 | Cases requiring remote hosts, NFS/FTP/SSH/DHCP, IPsec/netns/tunnels, CAN/DCCP/SCTP/vsock, kconfig, or other special environment support. |

The previous network registration mixed these classes. It registered 589 names, while logs showed only 581 executions per libc because `test_robind.sh` disrupted the shell stream before the last eight SCTP names reached `RUN LTP CASE`.

## Batch Table

| Category | Batch | Count | File |
| --- | ---: | ---: | --- |
| `process` | `01` | 18 | `process-01.txt` |
| `process` | `02` | 26 | `process-02.txt` |
| `process` | `03` | 11 | `process-03.txt` |
| `process` | `04` | 22 | `process-04.txt` |
| `process` | `05` | 17 | `process-05.txt` |
| `process` | `06` | 23 | `process-06.txt` |
| `process` | `07` | 9 | `process-07.txt` |
| `process` | `08` | 27 | `process-08.txt` |
| `process` | `09` | 19 | `process-09.txt` |
| `process` | `10` | 18 | `process-10.txt` |
| `process` | `11` | 15 | `process-11.txt` |
| `process` | `12` | 23 | `process-12.txt` |
| `process` | `13` | 21 | `process-13.txt` |
| `process` | `14` | 27 | `process-14.txt` |
| `process` | `15` | 12 | `process-15.txt` |
| `fs` | `01` | 25 | `fs-01.txt` |
| `fs` | `02` | 30 | `fs-02.txt` |
| `fs` | `03` | 25 | `fs-03.txt` |
| `fs` | `04` | 30 | `fs-04.txt` |
| `fs` | `05` | 30 | `fs-05.txt` |
| `fs` | `06` | 30 | `fs-06.txt` |
| `fs` | `07` | 27 | `fs-07.txt` |
| `fs` | `08` | 30 | `fs-08.txt` |
| `fs` | `09` | 30 | `fs-09.txt` |
| `fs` | `10` | 30 | `fs-10.txt` |
| `fs` | `11` | 30 | `fs-11.txt` |
| `fs` | `12` | 30 | `fs-12.txt` |
| `fs` | `13` | 26 | `fs-13.txt` |
| `mm-ipc` | `01` | 30 | `mm-ipc-01.txt` |
| `mm-ipc` | `02` | 30 | `mm-ipc-02.txt` |
| `mm-ipc` | `03` | 30 | `mm-ipc-03.txt` |
| `mm-ipc` | `04` | 30 | `mm-ipc-04.txt` |
| `mm-ipc` | `05` | 30 | `mm-ipc-05.txt` |
| `mm-ipc` | `06` | 30 | `mm-ipc-06.txt` |
| `mm-ipc` | `07` | 30 | `mm-ipc-07.txt` |
| `mm-ipc` | `08` | 30 | `mm-ipc-08.txt` |
| `mm-ipc` | `09` | 30 | `mm-ipc-09.txt` |
| `common-easy` | `01` | 15 | `common-easy-01.txt` |
| `net` / `net-core` | `01` | 18 | `net-01.txt` |
| `net` / `net-core` | `02` | 28 | `net-02.txt` |
| `net` / `net-core` | `03` | 6 | `net-03.txt` |
| `net-all` | `all` | 589 | aggregate: `net-01..03`, `net-script-01..15`, `net-deferred-01..04` |
| `net-script` | `01` | 30 | `net-script-01.txt` |
| `net-script` | `02` | 30 | `net-script-02.txt` |
| `net-script` | `03` | 30 | `net-script-03.txt` |
| `net-script` | `04` | 30 | `net-script-04.txt` |
| `net-script` | `05` | 30 | `net-script-05.txt` |
| `net-script` | `06` | 30 | `net-script-06.txt` |
| `net-script` | `07` | 30 | `net-script-07.txt` |
| `net-script` | `08` | 30 | `net-script-08.txt` |
| `net-script` | `09` | 30 | `net-script-09.txt` |
| `net-script` | `10` | 30 | `net-script-10.txt` |
| `net-script` | `11` | 30 | `net-script-11.txt` |
| `net-script` | `12` | 30 | `net-script-12.txt` |
| `net-script` | `13` | 30 | `net-script-13.txt` |
| `net-script` | `14` | 30 | `net-script-14.txt` |
| `net-script` | `15` | 18 | `net-script-15.txt` |
| `net-deferred` | `01` | 30 | `net-deferred-01.txt` |
| `net-deferred` | `02` | 30 | `net-deferred-02.txt` |
| `net-deferred` | `03` | 30 | `net-deferred-03.txt` |
| `net-deferred` | `04` | 9 | `net-deferred-04.txt` |

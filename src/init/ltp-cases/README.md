# LTP Case Lists

`ltp-safe.txt` is the default embedded LTP case list.

Runtime LTP selection uses one entry point: `LTP_CASE_LIST`. When it is set,
the init script runs those case names for both glibc and musl. When it is
empty, `ltp-safe.txt` is used.

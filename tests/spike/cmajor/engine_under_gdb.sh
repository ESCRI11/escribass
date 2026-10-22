#!/usr/bin/env bash
export LD_LIBRARY_PATH=${SPIKE_ROOT}/gdb/root/usr/lib/x86_64-linux-gnu
exec ${SPIKE_ROOT}/gdb/root/usr/bin/gdb -q -nx -batch   -ex "set logging file ${SPIKE_ROOT}/m1/gdb.log" -ex "set logging overwrite on" -ex "set logging redirect on" -ex "set logging enabled on"   -ex "set print thread-events off" -ex "set print inferior-events off" -ex "set pagination off"   -ex "handle SIGPIPE nostop noprint" -ex run -ex "thread apply all bt 40" -ex kill   --args $(cd "$(dirname "$0")/../../.." && pwd)/engine/build/escribass_engine_artefacts/Release/escribass_engine "$@"

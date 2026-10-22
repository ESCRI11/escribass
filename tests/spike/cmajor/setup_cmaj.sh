#!/usr/bin/env bash
# m5.1-spike measurements 1-3: Cmajor 024a208 with LLVM sparse, built under the engine's flags.
set -euo pipefail
R=$1; cd "$R"
t0=$(date +%s.%N)
git clone -q --filter=blob:none --no-checkout https://github.com/cmajor-lang/cmajor cmajor
cd cmajor && git checkout -q 024a208515f15e43271d9b2ea85ee22a2233384b
sed -i 's#git@github.com:#https://github.com/#' .gitmodules && git submodule sync -q
git submodule status | awk '{print $2}' | grep -v '^3rdParty/llvm$' > /tmp/cmaj_subs.$$
xargs -a /tmp/cmaj_subs.$$ git submodule update -q --init --depth 1 --jobs 8
t1=$(date +%s.%N); echo "submodules $(echo "$t1-$t0"|bc) s"
git submodule init -q 3rdParty/llvm
rm -rf 3rdParty/llvm && git clone -q --filter=blob:none --no-checkout --sparse https://github.com/cmajor-lang/llvm 3rdParty/llvm
git -C 3rdParty/llvm sparse-checkout set release/linux/x64
git -C 3rdParty/llvm checkout -q c6380f9b8818b666a264399d330f31f55e806b74
t2=$(date +%s.%N); echo "llvm sparse $(echo "$t2-$t1"|bc) s"
git submodule status | sed 's/^[ +-]//' | awk '{print $2, $1}'
F='-march=x86-64 -mtune=generic -ffp-contract=off'
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_CMAJ=ON -DBUILD_CMAJ_LIB=OFF -DBUILD_EXAMPLES=OFF \
  -DCMAKE_C_FLAGS="$F" -DCMAKE_CXX_FLAGS="$F" > configure.log
/usr/bin/time -v cmake --build build -j 16 > build.log 2> time.log
t3=$(date +%s.%N); echo "build $(echo "$t3-$t2"|bc) s"
grep -E "Elapsed|Maximum resident|User time|System time" time.log
find build -name cmaj -type f -executable | head -1

from os import system
import subprocess

def compile_prog(slots: int):
    subprocess.run(["make", "-B", f"DEPS=-DGC_CHUNK_SLOTS={slots}"])
    subprocess.run(["mv", "bin/forsp", f"bin/forsp-{slots}"])

nums = list(range(9, 15))

for i in nums:
    compile_prog(1 << i)

samples = [f"./bin/forsp-{1 << x} ./examples/forsp.fp" for x in nums]

# for i in range(len(samples) - 1):
#     subprocess.run(["poop", samples[i], samples[i + 1]])

for sample in samples:
    subprocess.run(["poop", "./forsp ./examples/forsp.fp", sample])

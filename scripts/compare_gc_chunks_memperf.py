from os import system
import subprocess

def compile_prog(slots: int):
    subprocess.run(["make", "clean", "memperf", f"DEPS=-DGC_CHUNK_SLOTS={slots}"])

def get_results():
    process = subprocess.run(["grep", "-n", "HEAP SUMMARY", "gc.results"], capture_output=True, text=True)
    output = process.stdout
    lines = []
    with open("gc.results", "r") as fp:
        lines = fp.readlines()
    line_index = int(output[:(output.find(":"))])
    in_use = lines[line_index]
    total = lines[line_index + 1]
    return (in_use[in_use.find("in use"):].strip(), total[total.find("total heap usage"):].strip())

with open("results.txt", "w") as fp:
    for i in range(1, 13):
        compile_prog(1 << i)
        in_use, total = get_results()
        fp.write(f"{1 << i}:\n\t{in_use}\n\t{total}\n")

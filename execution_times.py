import time
import subprocess
def timeit(command):
    t0 = time.perf_counter()
    subprocess.check_output([command])
    subprocess.check_output([command])
    subprocess.check_output([command])
    subprocess.check_output([command])
    t1 = time.perf_counter()
    return (t1 - t0) / 4
for pair in sorted([(comm, timeit(comm)) for comm in subprocess.check_output(["find", "testc/bin"]).decode().split("\n") if "-" in comm], key=lambda a: a[1]):
    print(pair)

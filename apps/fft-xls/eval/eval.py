import os
import sys
import subprocess

# e.g. 4stage_butterfly
args = sys.argv[1:]
dir = args[0]

for subdir in os.listdir(dir):
    if "fft_" in subdir:
        print('subdir: ' + subdir)
        subprocess.run('./run-eval.sh ' + dir + '/' + subdir + '/harness.fil ' + dir + '/' + subdir + ' ' + dir + '/device.xdc', shell=True)

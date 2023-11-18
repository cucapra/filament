import os
import sys
import subprocess

# src dir, e.g. 4stage_butterfly
args = sys.argv[1:]
dir = args[0]

for subdir in os.listdir(dir):
    print('subdir: ' + subdir)
    butterflies = subdir.split('_')[1]
    subprocess.run('./apps/fft-xls/eval/gen.sh apps/eval/harness_' + butterflies + '.fil ' + dir + '/' + subdir, shell=True)

import os
import sys
import subprocess

# e.g. 4stage_butterfly
args = sys.argv[1:]
dir = args[0]

for subdir in os.listdir(dir):
    print('subdir: ' + subdir)
    subprocess.Popen('./run-eval.sh ' + dir + '/' + subdir + '/harness.fil ' + dir + '/' + subdir, shell=True)

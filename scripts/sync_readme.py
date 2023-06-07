#!/usr/bin/python

import subprocess

tag_start = "<!-- HELP_START -->"
tag_end = "<!-- HELP_END -->"
readme_file = "README.md"

help_text = subprocess.check_output(['cargo', 'run', '--', '--help']).decode('utf-8')

with open(readme_file, 'r') as file:
    data = file.read()

start_parts = data.split(tag_start)
start = start_parts[0]

end = start_parts[1].split(tag_end)[1] 

out = start + tag_start + "\n```\n" + help_text + "\n```\n" + tag_end + end
print(out)

with open(readme_file, 'w') as file:
   file.write(out)


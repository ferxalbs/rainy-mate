import re

with open('src-tauri/src/lib.rs', 'r') as f:
    lines = f.readlines()

out = []
found = False
for line in lines:
    if line.startswith('use tauri::Builder;') and not found:
        out.append('use tauri::Manager;\n')
        out.append(line)
        found = True
    else:
        out.append(line)

with open('src-tauri/src/lib.rs', 'w') as f:
    f.writelines(out)

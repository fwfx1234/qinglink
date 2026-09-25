import os

base = r'F:\develop\qingqi-workspace\qinglink'

# Fix 1: single_instance.rs - add WPARAM/LPARAM imports
si_path = os.path.join(base, 'crates', 'qinglink-windows', 'src', 'single_instance.rs')
print(f'Fixing: {si_path}')
with open(si_path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    # Fix: Add LPARAM, WPARAM to Foundation imports in activate_existing's use block
    if 'Foundation::HWND,' in line and 'UI::WindowsAndMessaging' not in line:
        line = line.replace('Foundation::HWND,', 'Foundation::{HWND, LPARAM, WPARAM},')
    new_lines.append(line)

with open(si_path, 'w', encoding='utf-8') as f:
    f.writelines(new_lines)
print('Done')

# Fix 2: database.rs - fix the manager move issue
db_path = os.path.join(base, 'crates', 'qinglink-storage', 'src', 'database.rs')
print(f'Fixing: {db_path}')
with open(db_path, 'r', encoding='utf-8') as f:
    content = f.read()

# The issue is that manager is moved in the loop. We need to restructure.
# Move let manager inside the loop
content = content.replace(
    '        let manager = SqliteConnectionManager::file(&path);\n        let mut last_error = None;\n        for attempt in 0..3 {',
    '        let mut last_error = None;\n        for attempt in 0..3 {\n            let manager = SqliteConnectionManager::file(&path);'
)

with open(db_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Done')

print('All fixes applied!')

import os

base = r'F:\develop\qingqi-workspace\qinglink'

# Fix 1: single_instance.rs - add back core::w import for w! macro
si_path = os.path.join(base, 'crates', 'qinglink-windows', 'src', 'single_instance.rs')
print(f'Checking: {si_path}')
with open(si_path, 'r', encoding='utf-8') as f:
    content = f.read()

# The w! macro is used in the acquire() function, so we need core::w
# Add core::w back to the imports
old = '''use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{FindWindowW, PostMessageW, SW_RESTORE, SetForegroundWindow, ShowWindow},
    },
};'''
new = '''use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        UI::WindowsAndMessaging::{FindWindowW, PostMessageW, SW_RESTORE, SetForegroundWindow, ShowWindow},
    },
    core::w,
};'''
content = content.replace(old, new)

with open(si_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Fixed single_instance.rs')

# Fix 2: database.rs - fix the manager move issue
db_path = os.path.join(base, 'crates', 'qinglink-storage', 'src', 'database.rs')
print(f'Checking: {db_path}')
with open(db_path, 'r', encoding='utf-8') as f:
    content = f.read()

# The issue is that manager is moved in the loop. We need to restructure the retry logic.
old = '''        let manager = SqliteConnectionManager::file(&path);
        let mut last_error = None;
        for attempt in 0..3 {
            match Pool::builder()
                .max_size(2)
                .idle_timeout(Some(Duration::from_secs(60)))
                .connection_customizer(Box::new(ConnectionCustomizer))
                .build(manager)
            {
                Ok(pool) => {
                    let mut pools = self
                        .pools
                        .write()
                        .map_err(|_| anyhow::anyhow!("database pool registry lock poisoned"))?;
                    let pool = pools.entry(canonical_key).or_insert_with(|| pool.clone());
                    return Ok(pool.clone());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 2 {
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }
        anyhow::bail!(
            "cannot build sqlite pool {}: {}",
            path.display(),
            last_error.unwrap()
        )'''

new = '''        let mut last_error = None;
        for attempt in 0..3 {
            let manager = SqliteConnectionManager::file(&path);
            match Pool::builder()
                .max_size(2)
                .idle_timeout(Some(Duration::from_secs(60)))
                .connection_customizer(Box::new(ConnectionCustomizer))
                .build(manager)
            {
                Ok(pool) => {
                    let mut pools = self
                        .pools
                        .write()
                        .map_err(|_| anyhow::anyhow!("database pool registry lock poisoned"))?;
                    let pool = pools.entry(canonical_key).or_insert_with(|| pool.clone());
                    return Ok(pool.clone());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 2 {
                        std::thread::sleep(Duration::from_millis(500));
                    }
                }
            }
        }
        anyhow::bail!(
            "cannot build sqlite pool {}: {}",
            path.display(),
            last_error.unwrap()
        )'''

content = content.replace(old, new)

with open(db_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Fixed database.rs')

print('All done!')

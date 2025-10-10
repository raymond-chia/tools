@echo off
cd /d %~dp0
for /d %%D in (*) do (
  if exist "%%D\\Cargo.toml" (
    echo ==^> 測試 %%D
    pushd %%D
    cargo test
    popd
  )
)

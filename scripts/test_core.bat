@echo off
REM ------------------------------------------------------------
REM 用途：測試 core 目錄下所有 Rust crate
REM 用法：將本檔放在專案根目錄執行
REM 備註：只會測試 core/* 下的檔案
REM ------------------------------------------------------------
cd /d "%~dp0\.."
for /d %%D in (core\*) do (
  if exist "%%D\Cargo.toml" (
    echo ==^> 測試 %%D
    pushd %%D
    cargo test
    popd
  )
)

@echo off
REM ------------------------------------------------------------
REM 用途：產生 core 目錄下所有 Rust crate 的 tarpaulin 覆蓋率報告（HTML），並自動改名
REM 用法：將本檔放在專案根目錄執行
REM ------------------------------------------------------------

@REM 切換到 bat 檔所在的目錄（/d 可跨磁碟機），確保後續路徑正確。
cd /d "%~dp0\.."
for /d %%D in (core\*) do (
  if exist "%%D\Cargo.toml" (
    echo ==^> 產生覆蓋率報告 %%D
    @REM 進入該 crate 子目錄，暫存目前目錄。
    pushd %%D
    cargo tarpaulin --out Lcov --output-dir coverage
    @REM 回到原本的根目錄。
    popd
    if exist coverage\tarpaulin-report.html (
      move /Y coverage\tarpaulin-report.html coverage\%%~nD.html >nul
    )
    if exist coverage\lcov.info (
      move /Y coverage\lcov.info coverage\lcov-%%~nD.info >nul
    )
  )
)

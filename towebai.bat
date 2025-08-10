@echo off
REM 將 core 目錄下所有 .rs 檔案內容合併寫入 towebai.rs
REM 1. 先清空 towebai.rs
REM 2. 遞迴尋找所有 .rs 檔案並依序附加內容

set OUTPUT=towebai.rs
cd /d "%~dp0"

REM 清空輸出檔案
echo. > %OUTPUT%

REM 遍歷 core 目錄下所有 .rs 檔案並附加到 towebai.rs
for /r "%~dp0core" %%f in (*.rs) do (
    echo ==== %%f ==== >> %OUTPUT%
    type "%%f" >> %OUTPUT%
    echo. >> %OUTPUT%
)

REM 完成訊息
echo 所有 core 目錄下的 .rs 檔案已合併到 %OUTPUT%
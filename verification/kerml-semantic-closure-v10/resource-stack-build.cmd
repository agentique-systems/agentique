@echo off
call "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat" -arch=x64 -no_logo
if errorlevel 1 exit /b %errorlevel%
cl /nologo /EHsc /W4 /Fe:.cache\v10-stack-sample.exe /Fo:.cache\v10-stack-sample.obj .cache\v10-stack-sample.cpp /link dbghelp.lib
exit /b %errorlevel%

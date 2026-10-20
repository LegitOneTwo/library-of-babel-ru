@echo off
rem Fallback MSVC environment: vcvars64.bat stopped working on this machine
rem ("Системе не удается найти указанный путь"), so the required variables are
rem set explicitly from the known installation layout.
set "VCROOT=C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207"
set "SDKROOT=C:\Program Files (x86)\Windows Kits\10"
set "SDKVER=10.0.26100.0"
set "INCLUDE=%VCROOT%\include;%VCROOT%\include\cppx;%SDKROOT%\include\%SDKVER%\ucrt;%SDKROOT%\include\%SDKVER%\um;%SDKROOT%\include\%SDKVER%\shared;%SDKROOT%\include\%SDKVER%\winrt;%SDKROOT%\include\%SDKVER%\cppwinrt"
set "LIB=%VCROOT%\lib\x64;%SDKROOT%\lib\%SDKVER%\ucrt\x64;%SDKROOT%\lib\%SDKVER%\um\x64"
set "LIBPATH=%VCROOT%\lib\x64;%SDKROOT%\UnionMetadata\%SDKVER%;%SDKROOT%\References\%SDKVER%"
set "PATH=%VCROOT%\bin\Hostx64\x64;%SDKROOT%\bin\%SDKVER%\x64;%PATH%"
set "VCToolsInstallDir=%VCROOT%\"
set "WindowsSdkDir=%SDKROOT%\"
set "WindowsSDKVersion=%SDKVER%\"
set

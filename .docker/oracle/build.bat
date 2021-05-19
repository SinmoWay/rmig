@echo on
set DIR=%~1

rem See https://www.oracle.com/database/technologies/xe-downloads.html, if need another version.
set ORACLE_FILE=oracle-database-xe-18c-1.0-1.x86_64.rpm
set SINGLE_INSTANCE_REPO=https://github.com/oracle/docker-images/archive/refs/heads/main.zip
set DEFAULT_ORACLE=https://edelivery.oracle.com/otn-pub/otn_software/db-express/%ORACLE_FILE%

if "%DIR%"=="" ( set DIR=. )

set "rpm=dir %DIR% | findstr %ORACLE_FILE%"

FOR /F %%i IN ('%%rpm%%') DO set EX=%i

if "%EX%" == "" (
    echo Download oracle server.
    rem choco install wget
    call wget.exe -v -P %DIR% %DEFAULT_ORACLE%
)

IF EXIST "%DIR%\docker-images-main/nul" (
    echo Start building docker file.
) else (
    echo Download oracle docker-images (Git Repository)
    rem call wget.exe -v -P %DIR% %SINGLE_INSTANCE_REPO%
    echo Unzip files
    rem call tar.exe -xf %DIR%\main.zip -C %DIR%\
)
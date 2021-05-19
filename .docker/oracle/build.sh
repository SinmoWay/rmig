set DIR=%~1

set ORACLE_FILE=oracle-database-xe-18c-1.0-1.x86_64.rpm
set SINGLE_INSTANCE_REPO=https://github.com/oracle/docker-images/archive/refs/heads/main.zip
set DEFAULT_ORACLE=https://edelivery.oracle.com/otn-pub/otn_software/db-express/%ORACLE_FILE%

#if "%DIR%"=="" ( set DIR=. )

set "rpm=dir %DIR% | findstr %ORACLE_FILE%"

#FOR /F %%i IN ('%%rpm%%') DO set EX=%i

if "%EX%" == "" (
#    echo Download oracle server.
#    rem choco install wget
#    call wget.exe -v -P %DIR% %DEFAULT_ORACLE%
)

if NOT EXIST %DIR%\docker-images-main\ (
#    call wget.exe -v -P %DIR% %SINGLE_INSTANCE_REPO%
#    echo Unzip files
#    call tar.exe -xf %DIR%\main.zip -C %DIR%\
)

IF EXIST %DIR%\docker-images-main\ (
    echo Start building docker file.
)
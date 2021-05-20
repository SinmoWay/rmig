SET VERSION=%~1
SET DOCKERFILE==%2

IF "%VERSION%" == "" ( SET VERSION=18.4.0)

IF "%VERSION%"=="18.4.0" ( SET DOCKERFILE=.\18.4.0\Dockerfile)

call docker build --force-rm=true --no-cache=true -t oracle/database:%VERSION% -f %DOCKERFILE% .\%VERSION%\



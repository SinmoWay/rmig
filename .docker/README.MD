# For development or testing

For local development, it is most convenient to have a database at hand. We actually use docker-compose for these purposes.

## Basic commands:
* `docker compose up {{service}}` or if you want to bring up all services `docker compose up`.
* Alternative `docker compose start {{service}}` or `docker compose start`
* To exit in case of `up`, close the console or exit the subroutine `ctrl + c` or use `docker compose stop {{service}}` or `docker compose stop` when command `start`

## Building oracle distribution
In the ./oracle directory, there are `build.bat` and `buid.sh`.  
Assembled version:
```
type: XE
version: 18.4.0
```
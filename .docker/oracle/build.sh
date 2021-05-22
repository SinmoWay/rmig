#!/bin/bash

if [ "$1" == "" ]; then
  VERSION="18.4.0"
fi;

if [ $VERSION == "18.4.0" ]; then
  DOCKERFILE="/18.4.0/Dockerfile"
fi;

docker build --force-rm=true --no-cache=true -t oracle/database:$VERSION -f $DOCKERFILE ./$VERSION/
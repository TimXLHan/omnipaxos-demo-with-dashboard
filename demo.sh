#!/bin/bash

if [ $# -eq 0 ]; then
  # If no argument is provided, perform Command
  docker compose up --build -d
else
  if [ "$1" = "kill" ]; then
    # If the argument is "kill", perform Command Y
    docker kill $(docker ps -q)
  elif [ "$1" = "attach" ]; then
    # Handle any other argument cases here
      docker compose up --build -d
      docker attach coordinator
  else
    echo "Invalid argument: $1. Valid arguments are 'kill' and 'attach'"
  fi
fi

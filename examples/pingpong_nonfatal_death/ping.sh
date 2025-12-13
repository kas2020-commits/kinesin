#!/bin/sh

count=0

while [ $count -le 5 ]
do
  sleep 1
  echo "ping"
  count=$(( count + 1 ))
done

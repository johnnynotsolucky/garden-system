#!/bin/sh

avrdude \
  -p atmega328p \
  -b 115200 \
  -D \
  -P $2 \
  -c arduino \
  -U flash:w:./$1:e

#!/bin/sh
root=$(dirname $0)
phantomjs --proxy=http://127.0.0.1:8123 $root/seed.js

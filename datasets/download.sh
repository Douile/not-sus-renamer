#!/bin/sh

DESTDIR=$(dirname "$0")
SOURCE="https://datasets.imdbws.com/"
FILES="title.akas.tsv.gz title.basics.tsv.gz title.episode.tsv.gz title.ratings.tsv.gz"
URLS=$(for file in $FILES; do printf "%s%s " "$SOURCE" "$file"; done)

cd "$DESTDIR"
aria2c -j16 -x16 -s16 -c -V -Z $URLS
pigz -d $FILES

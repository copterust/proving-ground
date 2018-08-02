#!/bin/bash

for dir in `ls -d *`; do
    if [ -d $dir ] && [ -f $dir/Cargo.toml ] ; then
        echo "Building $dir"
        cd $dir && cargo build --verbose || exit 1; cd ..
    fi
done

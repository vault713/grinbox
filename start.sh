#!/bin/bash

if curl --retry 20 --retry-delay 2 --retry-connrefused rabbit:15672 ; then
    ./grinbox
else
    echo "goodbye!"
fi




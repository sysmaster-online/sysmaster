#!/bin/bash

echo "use crate::input_event_codes;
use std::collections::HashMap;

pub fn get_input_event_keycode(key_lookup: &str) -> u32 {
    let input_map = HashMap::from([" > "$1/get_input_event_key.rs"

cat "$1/input_event_codes.rs" | while read line
do
    echo "$line" | grep "pub " > /dev/null
    if [ $? -eq 0 ]
    then
        key=`echo "$line" | awk -F " " '{print $3}' | awk -F ":" '{print $1}'`

        str_size=$(expr length "(\"$key\", input_event_codes::$key),")
        if [ $str_size -ge 64 ]
        then
            echo "        (
            \"$key\",
            input_event_codes::$key,
        )," >> "$1/get_input_event_key.rs"
        else
            echo "        (\"$key\", input_event_codes::$key)," >> "$1/get_input_event_key.rs"
        fi
    fi
done

echo "    ]);

    *input_map.get(key_lookup).unwrap()
}" >> "$1/get_input_event_key.rs"

#!/usr/bin/env bash

p_w_d=`pwd`
echo $p_w_d

target_dir=$1
echo $target_dir
cp -a $p_w_d/tests/test_units/*  $target_dir
cp -a $p_w_d/tests/presets/*  $target_dir
cp -a $p_w_d/config/conf  $target_dir
exit $?

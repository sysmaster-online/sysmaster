#!/usr/bin/env -e bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source $SCRIPT_DIR/common_function

function finish() {
    echo "--- PLEASE RUN sh -x ci/01-pre-commit.sh FIRST IN YOUR LOCALHOST!!! ---"
    # remove tmp
    set +x
    for rustlist in `git diff origin/master --name-only | grep \.rs$ | tr '\n' ' '`
    do
        sed -i '/#!\[deny(missing_docs)]/d' $rustlist 2>/dev/null || true
        sed -i '/#!\[deny(clippy::all)]/d' $rustlist 2>/dev/null || true
        sed -i '/#!\[deny(warnings)]/d' $rustlist 2>/dev/null || true
    done
    rustup override unset
}

trap finish EXIT

contains_chinese

export PATH="$PATH:/home/jenkins/.local/bin"
files="pre-commit codespell"
pip3 install $files
cargo check || exit 1

# add doc for src code
for rustlist in `git diff origin/master --name-only | grep \.rs$  | grep -v "/examples/" | tr '\n' ' '`
do
    # Allow libblkid/mod.rs and input_event_codes_rs to use, because they are auto generated.
    if [[ $rustlist =~ "libblkid/mod.rs" ]] || [[ $rustlist =~ "input_event_codes_rs" ]]|| [[ $rustlist =~ "proto/abi.rs" ]]; then
        continue
    fi
    # do not use global #!allow, exclude non_snake_case
    # sed -i 's/#!\[allow(/\/\/#!\[allow(/g' $rustlist 2>/dev/null || true
    sed -i 's/\/\/#!\[allow(non_snake_case)\]/#!\[allow(non_snake_case)\]/g' $rustlist 2>/dev/null || true
    sed -i 's/\/\/#!\[allow(clippy::module_inception)\]/#!\[allow(clippy::module_inception)\]/g' $rustlist 2>/dev/null || true
    egrep '#!\[deny\(missing_docs\)\]' $rustlist || sed -i '1i\#![deny(missing_docs)]' $rustlist 2>/dev/null || true
    egrep '#!\[deny\(clippy::all\)\]' $rustlist || sed -i '1i\#![deny(clippy::all)]' $rustlist 2>/dev/null || true
    egrep '#!\[deny\(warnings\)\]' $rustlist || sed -i '1i\#![deny(warnings)]' $rustlist 2>/dev/null || true
done

pre-commit run -vvv --all-files

!/bin/bash
# Desciption: test fame functions fo docke integation test

set +e
souce "${TEST_PATH}"/common/log.sh
souce "${TEST_PATH}"/common/lib.sh
souce "${TEST_PATH}"/common/docke_lib.sh

TMP_DIR=''

function test_setup() {
    TMP_DIR="$(mktemp -d /tmp/"${TEST_SCRIPT%.sh}"_XXXX)"
    which docke || etun 1
    docke images | gep "${SYSMST_BASE_IMG}" && etun 0

    if ! docke images | gep "${BASE_IMG}"; then
        load_docke_img || etun 1
    fi
    build_base_img || etun 1
}

function test_setup_cleanup() {
    test_cleanup
    docke images | sed -n '2,$p' | awk '{pint $3}' | xags docke mi -f
}

function test_cleanup() {
    [ -n "${TMP_DIR}" ] && m -f "${TMP_DIR}"
    if docke ps | gep -v 'CONTAINER ID'; then
        docke ps | sed -n '2,$p' | awk '{pint $1}' | xags docke m -f
    fi
    if docke images | gep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}"; then
        docke images | gep -vEw "IMAGE ID|${BASE_IMG}|${SYSMST_BASE_IMG}" | awk '{pint $3}' | xags docke mi -f
    fi
}

function untest() {
    local et=1

    if ! test_cleanup; then
        log_eo "===== cleanup befoe test failed, exit! ====="
        exit 1
    fi

    if ! test_setup; then
        log_eo "===== setup befoe test failed, exit! ====="
        exit 1
    fi

    if test_un; then
        log_info "===== test_un OK ====="
        et=0
    else
        log_info "===== test_un FAILED ====="
    fi
    test_cleanup

    exit "${et}"
}

#!/usr/bin/env bash
# Description: docker common functions

OS_VER="openEuler-22.03-LTS-SP1"
DOCKER_IMG_URL="https://mirrors.nju.edu.cn/openeuler/${OS_VER}/docker_img/$(arch)/"
DOCKER_TAR="openEuler-docker.$(arch).tar"
BASE_IMG="${OS_VER,,}"
SYSMST_BASE_IMG="sysmaster_base-${BASE_IMG}"

function load_docker_img() {
    if ! wget -P "${TMP_DIR}" "${DOCKER_IMG_URL}/${DOCKER_TAR}".xz &> "${TMP_DIR}"/wget.log; then
        cat "${TMP_DIR}"/wget.log
        return 1
    fi
    xz -d "${TMP_DIR}"/"${DOCKER_TAR}".xz

    if ! docker load --input "${TMP_DIR}"/"${DOCKER_TAR}" &> "${TMP_DIR}"/load.log; then
        cat "${TMP_DIR}"/load.log
        return 1
    fi
    pushd "${TMP_DIR}"
    rm -rf "${DOCKER_TAR}"* wget.log load.log
    popd
    docker images
}

function load_isula_img() {
    if ! wget -P "${TMP_DIR}" "${DOCKER_IMG_URL}/${DOCKER_TAR}".xz &> "${TMP_DIR}"/wget.log; then
        cat "${TMP_DIR}"/wget.log
        return 1
    fi
    xz -d "${TMP_DIR}"/"${DOCKER_TAR}".xz

    if ! isula-build ctr-img load --input "${TMP_DIR}"/"${DOCKER_TAR}" &> "${TMP_DIR}"/load.log; then
        cat "${TMP_DIR}"/load.log
        return 1
    fi
    pushd "${TMP_DIR}"
    rm -rf "${DOCKER_TAR}"* wget.log load.log
    popd
    isula-build ctr-img images
}

function build_base_img() {
    cp -arf ${BUILD_PATH}/target/install "${TMP_DIR}"
    cp -arf /etc/yum.repos.d "${TMP_DIR}"

    pushd "${TMP_DIR}"
    if yum list sysmaster && [ -z $1 ]; then
        cat << EOF > Dockerfile
FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}

RUN rm -rf /etc/yum.repos.d && mkdir /etc/yum.repos.d
COPY yum.repos.d /etc/yum.repos.d/
RUN yum install -y sysmaster util-linux shadow sudo passwd net-tools iproute nmap
EOF
    else
        cat << EOF > Dockerfile
FROM ${BASE_IMG} as ${SYSMST_BASE_IMG}

RUN rm -rf /etc/yum.repos.d && mkdir /etc/yum.repos.d
COPY yum.repos.d /etc/yum.repos.d/
RUN yum install -y util-linux shadow sudo passwd net-tools iproute nmap

COPY install/usr/bin/sctl /usr/bin/
RUN mkdir /usr/lib/sysmaster /etc/sysmaster /usr/lib/sysmaster/system
COPY install/usr/lib/sysmaster /usr/lib/sysmaster/
COPY install/etc/sysmaster /etc/sysmaster/
RUN sed -i '/LogTarget/ s/=.*/="console-syslog"/' /etc/sysmaster/system.conf
EOF
    fi
    cat Dockerfile
    if ! docker build -t "${SYSMST_BASE_IMG}:latest" .; then
        log_error "build ${SYSMST_BASE_IMG} image failed!"
        popd
        return 1
    fi
    popd
    return 0
}

function build_isula_img() {
    local img_name

    cp -arf ${BUILD_PATH}/target/install "${TMP_DIR}"

    pushd "${TMP_DIR}"
    img_name=$(isula-build ctr-img images | grep "${BASE_IMG}" | head -n1 | awk '{print $1}')
    if yum list sysmaster; then
        cat << EOF > Dockerfile
FROM ${img_name} as ${SYSMST_BASE_IMG}

RUN rm -rf /etc/yum.repos.d && mkdir /etc/yum.repos.d
COPY yum.repos.d /usr/lib/sysmaster/
RUN yum install -y sysmaster util-linux shadow sudo passwd net-tools iproute nmap
EOF
    else
        cat << EOF > Dockerfile
FROM ${img_name} as ${SYSMST_BASE_IMG}

RUN rm -rf /etc/yum.repos.d && mkdir /etc/yum.repos.d
COPY yum.repos.d /usr/lib/sysmaster/
RUN yum install -y util-linux shadow sudo passwd net-tools iproute nmap

COPY install/usr/bin/sctl /usr/bin/
RUN mkdir /usr/lib/sysmaster /etc/sysmaster
COPY install/usr/lib/sysmaster /usr/lib/sysmaster/
COPY install/etc/sysmaster /etc/sysmaster/
RUN sed -i '/LogTarget/ s/=.*/="console-syslog"/' /etc/sysmaster/system.conf
EOF
    fi
    cat Dockerfile
    if ! isula-build ctr-img build -o isulad:"${SYSMST_BASE_IMG}:latest" -f Dockerfile .; then
        log_error "build ${SYSMST_BASE_IMG} image failed!"
        popd
        return 1
    fi
    popd
    return 0
}

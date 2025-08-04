# Installation and Deployment

sysmaster can be used in containers and VMs. This document uses the AArch64 architecture as an example to describe how to install and deploy sysmaster in both scenarios.

## Software

* OS: openEuler 23.09

## Hardware

* x86_64 or AArch64 architecture

## Installation and Deployment in Containers

1. Install Docker.

    ```bash
    yum install -y docker
    systemctl restart docker
    ```

2. Load the base container image.

    Download the container image.

    ```bash
    wget https://repo.openeuler.org/openEuler-23.09/docker_img/aarch64/openEuler-docker.aarch64.tar.xz
    xz -d openEuler-docker.aarch64.tar.xz
    ```

    Load the container image.

    ```bash
    docker load --input openEuler-docker.aarch64.tar
    ```

3. Build the container.

    Create a Dockerfile.

    ```bash
    cat << EOF > Dockerfile
    FROM openeuler-23.09
    RUN yum install -y sysmaster
    CMD ["/usr/lib/sysmaster/init"]
    EOF
    ```

    Build the container.

    ```bash
    docker build -t openeuler-23.09:latest .
    ```

4. Start and enter the container.

    Start the container.

    ```bash
    docker run -itd --privileged openeuler-23.09:latest
    ```

    Obtain the container ID.

    ```bash
    docker ps
    ```

    Use the container ID to enter the container.

    ```bash
    docker exec -it <container ID> /bin/bash
    ```

## Installation and Deployment in VMs

1. Create an initramfs image.
    To avoid the impact of systemd in the initrd phase, you need to create an initramfs image with systemd removed and use this image to enter the initrd procedure. Run the following command:

    ```bash
    dracut -f --omit "systemd systemd-initrd systemd-networkd dracut-systemd" /boot/initrd_withoutsd.img
    ```

2. Add a boot item.
    Add a boot item to **grub.cfg**, whose path is **/boot/efi/EFI/openEuler/grub.cfg** in the AArch64 architecture and **/boot/grub2/grub.cfg** in the x86_64 architecture. Back up the original configurations and modify the configurations as follows:

    * **menuentry**: Change **openEuler (6.4.0-5.0.0.13.oe23.09.aarch64) 23.09** to **openEuler 23.09 withoutsd**.
    * **linux**: Change **root=/dev/mapper/openeuler-root ro** to **root=/dev/mapper/openeuler-root rw**.
    * **linux**: If Plymouth is installed, add **plymouth.enable=0** to disable it.
    * **linux**: Add **init=/usr/lib/sysmaster/init**.
    * **initrd**: Set to **/initrd_withoutsd.img**.
3. Install sysmaster.

    ```bash
    yum install sysmaster
    ```

4. If the **openEuler 23.09 withoutsd** boot item is displayed after the restart, the configuration is successful. Select **openEuler 23.09 withoutsd** to log in to the VM.

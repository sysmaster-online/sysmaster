#needsrootforbuild
%global __cargo_skip_build 0
%global _debugsource_packages 1
%global _debuginfo_subpackages 1
%define _unpackaged_files_terminate_build 0
%global sysmaster_install_source target/release
%global sysmaster_install_target %{buildroot}/usr/lib/sysmaster
%global factory_install_source factory
%global factory_install_target %{buildroot}
%global __cargo_common_opts %{?__cargo_common_opts} --all
%global _cargo_build /usr/bin/env CARGO_HOME=.cargo RUSTC_BOOTSTRAP=1 %{_bindir}/cargo build %__cargo_common_opts

Name:           sysmaster
Version:        0.5.1
Release:        1
Summary:        redesign and reimplement process1.

License:        Mulan PSL v2
URL:            https://gitee.com/openeuler/sysmaster
Source0:        %{name}-%{version}.tar.xz


ExclusiveArch:  x86_64 aarch64

BuildRequires:  rust cargo rust-packaging
BuildRequires:  gcc clang openssl-libs

%description
redesign and reimplement process1.

Summary:        %{summary}

%package -n devmaster
Summary:        Infrastructure of device management in userspace.
BuildRequires:  util-linux-devel kmod-devel

%description -n devmaster
This package provides the infrastructure of device management in userspace.

%prep
%autosetup -p1

%cargo_generate_buildrequires

%build
cat << EOF >> ./.cargo/config

[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

%{_cargo_build} --profile release -vvvv

%install
install -Dm0750 -t %{buildroot}/usr/bin %{sysmaster_install_source}/sctl
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/init
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/sysmaster
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/fstab
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/sysmonitor
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/random_seed
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/rc-local-generator
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/hostname_setup

cp -a %{factory_install_source}/* %{factory_install_target}

install -Dm0750 -t %{buildroot}/usr/bin %{sysmaster_install_source}/devctl

mkdir -p %{buildroot}/etc/sysmaster/system/multi-user.target.wants

for unit in NetworkManager.service dbus.service dbus.socket fstab.service getty-tty1.service hostname-setup.service lvm-activate-openeuler.service udev-trigger.service udevd-control.socket udevd-kernel.socket udevd.service; do
    # enable service for booting
    if [[ "$unit" == *".service" ]]; then
        ln -s /usr/lib/sysmaster/system/$unit %{buildroot}/etc/sysmaster/system/multi-user.target.wants/$unit
    fi
done

# enable sshd service by default
ln -s /usr/lib/sysmaster/system/sshd.service %{buildroot}/etc/sysmaster/system/multi-user.target.wants/sshd.service

%files
%attr(0550,-,-) /usr/bin/sctl
%dir %attr(0550,-,-) /usr/lib/sysmaster
%dir %attr(0750,-,-) /usr/lib/sysmaster/system
/usr/lib/sysmaster/system/*
%attr(0550,-,-) /usr/lib/sysmaster/init
%attr(0550,-,-) /usr/lib/sysmaster/fstab
%attr(0550,-,-) /usr/lib/sysmaster/sysmonitor
%attr(0550,-,-) /usr/lib/sysmaster/random_seed
%attr(0550,-,-) /usr/lib/sysmaster/rc-local-generator
%attr(0550,-,-) /usr/lib/sysmaster/hostname_setup
%attr(0550,-,-) /usr/lib/sysmaster/sysmaster
%dir %attr(0750,-,-) /etc/sysmaster
%dir %attr(0750,-,-) /etc/sysmaster/system
%dir %attr(0750,-,-) /etc/sysmaster/system/multi-user.target.wants
/etc/sysmaster/system/multi-user.target.wants/*
/etc/sysmaster/system.conf
/usr/lib/udev/rules.d/99-sysmaster.rules

%files -n devmaster
%dir %attr(0750,-,-) /etc/devmaster
/etc/devmaster/config.toml
%dir %attr(0750,-,-) /etc/devmaster/rules.d
/etc/devmaster/rules.d/99-default.rules
%dir %attr(0750,-,-) /etc/devmaster/network.d
/etc/devmaster/network.d/99-default.link
%attr(0550,-,-) /usr/bin/devctl

%changelog
* Mon Aug 22 2022 He Xiaowen <hexiaowen@huawei.com> - 0.2.0-1
- initial package

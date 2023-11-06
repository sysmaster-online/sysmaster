#needsrootforbuild
%global __cargo_skip_build 0
%global _debugsource_packages 1
%global _debuginfo_subpackages 1
%define _unpackaged_files_terminate_build 0
%global sysmaster_install_source target/release
%global sysmaster_install_target %{buildroot}/usr/lib/sysmaster
%global unit_install_source units
%global unit_install_target %{sysmaster_install_target}/system
%global conf_install_source config/conf
%global devmaster_install_source target/release
%global devmaster_install_target %{buildroot}/usr/lib/devmaster
%global devmaster_conf_install_source exts/devmaster/config
%global devmaster_conf_install_target %{buildroot}/etc/devmaster
%global __cargo_common_opts %{?__cargo_common_opts} --all
%global _cargo_build /usr/bin/env CARGO_HOME=.cargo RUSTC_BOOTSTRAP=1 %{_bindir}/cargo build %__cargo_common_opts

Name:           sysmaster
Version:        0.5.0
Release:        3
Summary:        redesign and reimplement process1.

License:        Mulan PSL v2
URL:            https://gitee.com/openeuler/sysmaster
Source0:        %{name}-%{version}.tar.xz

Patch0: backport-fix-input_event_codes_rs-compatible-with-rustc-1.71..patch
Patch1:	backport-fix-Fixed-parsing-single-quotes-error.patch
Patch2:	backport-fix-devmaster-avoid-coredump-when-rules-directory-is.patch
Patch3:	backport-fix-device-avoid-inserting-empty-tag.patch
Patch4:	backport-fix-devmaster-append-trailing-white-line-in-99-defau.patch
Patch5: backport-fix-disable-User-Group-feature-for-hongmeng.patch
Patch6: backport-fix-enable-subtree_control-for-sub-cgroup-on-hongmen.patch

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
for i in $(seq 1 4); do sed -i '$d' ./Cargo.toml; done;

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

install -Dm0640 -t %{unit_install_target} %{unit_install_source}/*

install -Dm0640 -t %{buildroot}/etc/sysmaster %{conf_install_source}/system.conf

install -Dm0750 -t %{buildroot}/usr/bin %{devmaster_install_source}/devctl
install -Dm0750 -t %{devmaster_install_target} %{devmaster_install_source}/devmaster
install -Dm0640 -t %{devmaster_conf_install_target} %{devmaster_conf_install_source}/config.toml
install -Dm0640 -t %{devmaster_conf_install_target}/rules.d %{devmaster_conf_install_source}/rules.d/*
install -Dm0640 -t %{devmaster_conf_install_target}/network.d %{devmaster_conf_install_source}/network.d/*

mkdir -p %{buildroot}/etc/sysmaster/system/multi-user.target.wants

for unit in NetworkManager.service dbus.service dbus.socket fstab.service getty-tty1.service hostname-setup.service lvm-activate-openeuler.service udev-trigger.service udevd-control.socket udevd-kernel.socket udevd.service; do
    install -Dm0640 -t %{unit_install_target} tools/run_with_vm/$unit
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

%files -n devmaster
%dir %attr(0550,-,-) /usr/lib/devmaster
%dir %attr(0750,-,-) /etc/devmaster
/etc/devmaster/config.toml
%dir %attr(0750,-,-) /etc/devmaster/rules.d
/etc/devmaster/rules.d/99-default.rules
%dir %attr(0750,-,-) /etc/devmaster/network.d
/etc/devmaster/network.d/99-default.link
%attr(0550,-,-) /usr/bin/devctl
%attr(0550,-,-) /usr/lib/devmaster/devmaster

%changelog
* Fri Aug 25 2023 licunlong<licunlong1@huawei.com> - 0.5.0-3
- enable subtree_control for sub cgroup on hongmeng

* Wed Aug 23 2023 licunlong<licunlong1@huawei.com> - 0.5.0-2
- disable User/Group on hongmeng

* Mon Aug 14 2023 shenyangyang<shenyangyang4@huawei.com> - 0.5.0-1
- bump version to 0.5.0 to suppourt virtual machine

* Thu Jul 06 2023 xujing<xujing125@huawei.com> - 0.2.4-3
- fix objcopy permission denied when rpmbuild

* Tue Jun 27 2023 shenyangyang<shenyangyang4@huawei.com> - 0.2.4-2
- modify the sshd units

* Tue Jun 20 2023 shenyangyang<shenyangyang4@huawei.com> - 0.2.4-1
- update version to 0.2.4 for docker use

* Mon Jun 19 2023 huyubiao<huyubiao@huawei.com> - 0.2.3-4
- sync patches from upstream

* Fri Jun 16 2023 licunlong<licunlong1@huawei.com> - 0.2.3-3
- sync patches from upstream

* Tue May 30 2023 shenyangyang<shenyangyang4@huawei.com> - 0.2.3-2
- Support compatible compile with rust 1.60

* Sat May 6 2023 shenyangyang<shenyangyang4@huawei.com> - 0.2.3-1
- update version to 0.2.3

* Tue Sep 20 2022 licunlong<licunlong1@huawei.com> - 0.2.1-2
- rename process1 to sysmaster, and remove pctrl to /usr/bin

* Tue Sep 13 2022 licunlong<licunlong1@huawei.com> - 0.2.1-1
- sync patches from upstream

* Mon Aug 22 2022 He Xiaowen <hexiaowen@huawei.com> - 0.2.0-2
- strip the libraries

* Mon Aug 22 2022 He Xiaowen <hexiaowen@huawei.com> - 0.2.0-1
- initial package

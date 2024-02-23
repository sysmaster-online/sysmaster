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
Version:        1.0.0
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
BuildRequires:  libblkid-devel kmod-devel
Requires:       %{name}%{?_isa} = %{version}-%{release}
Requires(post):   sysmaster
Requires(preun):  sysmaster
Requires(postun): sysmaster

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

%{_cargo_build} --profile release

%install
# For binary files and .so files, the permission 750 in the install phase to prevent objcopy errors.
# In the files phase, the permission is set back to 550.
install -Dm0750 -t %{buildroot}/usr/bin %{sysmaster_install_source}/sctl
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/init
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/sysmaster
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/fstab
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/sysmonitor
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/random_seed
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/rc-local-generator
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/hostname_setup
install -Dm0750 -t %{sysmaster_install_target} %{sysmaster_install_source}/sysmaster-run
install -Dm0750 -t %{sysmaster_install_target}/system-generators %{sysmaster_install_source}/getty-generator

cp -a %{factory_install_source}/* %{factory_install_target}

install -Dm0750 -t %{buildroot}/usr/bin %{sysmaster_install_source}/devctl
ln -s /usr/bin/devctl %{buildroot}/usr/lib/devmaster/devmaster

for unit in NetworkManager.service dbus.service fstab.service hostname-setup.service getty.target sshd.service devctl-trigger.service; do
    # enable service for booting
    ln -s /usr/lib/sysmaster/system/$unit %{buildroot}/etc/sysmaster/system/multi-user.target.wants/$unit
done

for unit in udevd.service udev-trigger.service devmaster.service; do
    ln -s /usr/lib/sysmaster/system/$unit %{buildroot}/etc/sysmaster/system/sysinit.target.wants/$unit
done

# Install configurations under /etc.
sed -i 's/\"\/lib\/devmaster\/rules.d\"/&, \"\/etc\/udev\/rules.d\", \"\/run\/udev\/rules.d\", \"\/lib\/udev\/rules.d\"/' %{buildroot}/etc/devmaster/config.toml

%files
%attr(0550,-,-) /usr/bin/sctl
%dir %attr(0550,-,-) /usr/lib/sysmaster
%dir %attr(0750,-,-) /usr/lib/sysmaster/system
%attr(0640,-,-) /usr/lib/sysmaster/system/*
%attr(0550,-,-) /usr/lib/sysmaster/init
%attr(0550,-,-) /usr/lib/sysmaster/fstab
%attr(0550,-,-) /usr/lib/sysmaster/sysmonitor
%attr(0550,-,-) /usr/lib/sysmaster/random_seed
%attr(0550,-,-) /usr/lib/sysmaster/rc-local-generator
%attr(0550,-,-) /usr/lib/sysmaster/system-generators/getty-generator
%attr(0550,-,-) /usr/lib/sysmaster/hostname_setup
%attr(0550,-,-) /usr/lib/sysmaster/sysmaster-run
%attr(0550,-,-) /usr/lib/sysmaster/sysmaster
%dir %attr(0750,-,-) /etc/sysmaster
%dir %attr(0750,-,-) /etc/sysmaster/system
%dir %attr(0750,-,-) /etc/sysmaster/system/multi-user.target.wants
%dir %attr(0750,-,-) /etc/sysmaster/system/sysinit.target.wants
/etc/sysmaster/system/multi-user.target.wants/*
/etc/sysmaster/system/sysinit.target.wants/*
%attr(0640,-,-) /etc/sysmaster/system.conf
%attr(0444,-,-) /usr/lib/udev/rules.d/99-sysmaster.rules
%exclude /usr/lib/sysmaster/system/devctl-trigger.service
%exclude /usr/lib/sysmaster/system/devmaster-simu-udev.service
%exclude /usr/lib/sysmaster/system/devmaster.service
%exclude /etc/sysmaster/system/sysinit.target.wants/devmaster.service
%exclude /etc/sysmaster/system/multi-user.target.wants/devctl-trigger.service

%files -n devmaster
%dir %attr(0550,-,-) /usr/lib/devmaster
%dir %attr(0750,-,-) /etc/devmaster
%attr(0640,-,-) /etc/devmaster/config.toml
%dir %attr(0750,-,-) /etc/devmaster/rules.d
%attr(0640,-,-) /etc/devmaster/rules.d/99-default.rules
%dir %attr(0750,-,-) /etc/devmaster/network.d
%attr(0640,-,-) /etc/devmaster/network.d/99-default.link
%attr(0550,-,-) /usr/bin/devctl
%attr(0550,-,-) /usr/lib/devmaster/devmaster
%attr(0640,-,-) /usr/lib/sysmaster/system/devctl-trigger.service
%attr(0640,-,-) /usr/lib/sysmaster/system/devmaster-simu-udev.service
%attr(0640,-,-) /usr/lib/sysmaster/system/devmaster.service
%attr(0550,-,-) /usr/lib/devmaster/simulate_udev.sh
/etc/sysmaster/system/sysinit.target.wants/devmaster.service
/etc/sysmaster/system/multi-user.target.wants/devctl-trigger.service

%post -n sysmaster
test -f /usr/bin/sctl && ln -sf ../bin/sctl /usr/sbin/reboot || :
test -f /usr/bin/sctl && ln -sf ../bin/sctl /usr/sbin/shutdown || :
test -f /usr/bin/sctl && ln -sf ../bin/sctl /usr/sbin/poweroff || :
test -f /usr/bin/sctl && ln -sf ../bin/sctl /usr/sbin/halt || :

%postun -n sysmaster
test -f /usr/bin/systemctl && ln -sf ../bin/systemctl /usr/sbin/reboot || :
test -f /usr/bin/systemctl && ln -sf ../bin/systemctl /usr/sbin/shutdown || :
test -f /usr/bin/systemctl && ln -sf ../bin/systemctl /usr/sbin/poweroff || :
test -f /usr/bin/systemctl && ln -sf ../bin/systemctl /usr/sbin/halt || :


%post -n devmaster
test -f /etc/sysmaster/system/sysinit.target.wants/udevd.service && unlink /etc/sysmaster/system/sysinit.target.wants/udevd.service || :
test -f /etc/sysmaster/system/sysinit.target.wants/udev-trigger.service && unlink /etc/sysmaster/system/sysinit.target.wants/udev-trigger.service || :

%postun -n devmaster
if [ $1 -eq 0 ] ; then
    test -f /usr/lib/sysmaster/system/udevd.service && ln -s /usr/lib/sysmaster/system/udevd.service /etc/sysmaster/system/sysinit.target.wants/udevd.service || :
    test -f /usr/lib/sysmaster/system/udev-trigger.service && ln -s /usr/lib/sysmaster/system/udev-trigger.service /etc/sysmaster/system/sysinit.target.wants/udev-trigger.service || :
fi

%changelog
* Mon Aug 22 2022 He Xiaowen <hexiaowen@huawei.com> - 0.2.0-1
- initial package

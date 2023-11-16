config_opts['chroot_setup_cmd'] = 'install tar gcc-c++ openEuler-rpm-config openEuler-release which xz sed make bzip2 gzip gcc coreutils unzip diffutils cpio bash gawk rpm-build info patch util-linux findutils grep'
config_opts['dist'] = 'oe2203'  # only useful for --resultdir variable subst
config_opts['releasever'] = '22.03LTS_SP1'
config_opts['package_manager'] = 'dnf'
config_opts['description'] = 'openEuler 22.03 LTS SP1'
config_opts['extra_chroot_dirs'] = [ '/run/lock', ]
config_opts['bootstrap_image'] = 'docker.io/openeuler/openeuler:22.03-lts'

config_opts['dnf.conf'] = """
[main]
keepcache=1
debuglevel=2
reposdir=/dev/null
logfile=/var/log/yum.log
retries=20
obsoletes=1
gpgcheck=0
assumeyes=1
syslog_ident=mock
syslog_device=
metadata_expire=0
mdpolicy=group:primary
best=1
install_weak_deps=0
protected_packages=
module_platform_id=platform:oe2203
user_agent={{ user_agent }}

# all openEuler LTS release will continue developing and releasing SPx version
# such as 22.03-LTS -> 22.03-LTS-SP1 -> 22.03-LTS-SP2 ...
# all LTS-SPx is compatible with its LTS release
[everything]
name=everything
metalink=https://mirrors.openeuler.org/metalink?repo=$releasever/everything&arch=$basearch
enabled=1
gpgcheck=0
gpgkey=file:///usr/share/distribution-gpg-keys/openeuler/RPM-GPG-KEY-openEuler-EulerMaker

[update]
name=update
metalink=https://mirrors.openeuler.org/metalink?repo=$releasever/update&arch=$basearch
enabled=1
gpgcheck=0
gpgkey=file:///usr/share/distribution-gpg-keys/openeuler/RPM-GPG-KEY-openEuler-EulerMaker
"""
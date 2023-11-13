# 使用自己的RPM和Kiwi制作Docker镜像

Docker是一个开源的应用容器引擎，让开发者可以打包他们的应用以及依赖包到一个可移植的容器中，然后发布到任何流行的Linux机器上，也可以实现虚拟化。
Kiwi是一个开源的Linux发行版构建工具，可以帮助你创建自定义的Linux发行版。

在本教程中，我们将使用自己的RPM和Kiwi来制作Docker镜像。

步骤1：准备工作
首先，你需要安装Docker和Kiwi。如果你还没有安装，可以参考以下链接进行安装：
```
sudo dnf install docker kiwi
```

步骤2：创建Kiwi描述文件
Kiwi描述文件是一个XML文件，它描述了你的Linux发行版的配置。你可以使用Kiwi提供的工具来创建这个文件。

例如，你可以创建一个简单的描述文件，如下所示：

```xml
<?xml version="1.0" encoding="utf-8"?>

<image schemaversion="7.5" name="sysmaster-test-image-docker">
    <description type="system">
        <author>sysmaster groups</author>
        <contact>dev@openeuler.overweight</contact>
        <specification>docker test build</specification>
    </description>
    <preferences>
        <version>1.0.0</version>
        <packagemanager>dnf</packagemanager>
        <rpm-excludedocs>true</rpm-excludedocs>
        <rpm-check-signatures>false</rpm-check-signatures>
        <locale>en_US</locale>
        <keytable>us</keytable>
        <type image="docker">
            <containerconfig name="sysmaster"/>
        </type>
    </preferences>
    <users>
        <user password="$1$2ggIPMYl$rH6LFdXX7kLaFufWFvHmb0" home="/root" name="root" groups="root"/>
    </users>
    <repository type="rpm-md">
        <source path="https://mirrors.huaweicloud.com/openeuler/openEuler-22.03-LTS-SP1/everything/x86_64/"/>
    </repository>
    <repository type="rpm-md" priority="1">
        <source path="dir:///home/overweight/sysmaster/target/rpms"/>
    </repository>
    <packages type="image">
        <package name="sysmaster"/>
        <package name="openssh-server"/>
    </packages>
    <packages type="bootstrap">
        <package name="filesystem"/>
        <package name="findutils"/>
        <package name="shadow"/>
    </packages>
    <!-- <packages type="delete">
        <package name="rpm"/>
        <package name="pcre2"/>
        <package name="python"/>
        <package name="readline"/>
    </packages>
    <packages type="uninstall">
        <package name="rpm"/>
        <package name="python"/>
        <package name="readline"/>
    </packages>
    -->
</image>
```
在这个例子中，我们创建了一个名为`sysmaster`的Docker镜像，它包含了名为`sysmaster`和`openssh-server`的RPM包。其中sysmaster包是build-rpm-with-mock中脚本一键式生成的.

步骤3：构建Docker镜像
使用Kiwi的kiwi-ng工具来构建Docker镜像。

```bash
kiwi-ng system build --description . --target-dir my_image
```
在这个命令中，--description选项指定了Kiwi描述文件的路径，--target-dir选项指定了构建结果的存放路径。

```xml
   <repository type="rpm-md" priority="1">
        <source path="dir:///home/overweight/sysmaster/target/rpms"/>
    </repository>
```
你需要修改path,指向自己的repo目录.

步骤4：运行Docker镜像
使用Docker的docker run命令来运行你的Docker镜像。

```bash
docker run --privileged --rm -it sysmaster /bin/bash
```
结论
使用Kiwi和Docker，你可以轻松地创建和管理自己的Linux发行版。这不仅可以帮助你更好地理解Linux系统的工作原理，也可以帮助你更有效地管理和部署你的应用。

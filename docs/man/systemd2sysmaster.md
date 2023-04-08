# systemd迁移到sysmaster说明

## toml配置格式

sysmaster的单元配置文件，如`.service`、`.socket`等使用toml配置格式解析，细节请参考：<https://github.com/toml-lang/toml>。

一些常见的配置问题：

1. **布尔量只允许配置为`true`或`false`。** systemd支持解析`yes`、`no`、`y`、`n`等多种布尔量的表示形式，但sysmaster不支持。
2. **字符串请使用`"`。** systemd在配置如：`ExecStart`、`Description`等字符串的值时，无需使用引号，但sysmaster必须使用引号。
3. **相同配置仅允许配置一次。** systemd支持配置多个`After`、`Before`等，sysmaster不支持，用户可以使用`"`包裹的`;`分隔的多个单元名替换。这里重点说明，在多个单元的场景下，部分用户可能会根据toml语法误配置为`[]`数组，sysmaster实际上是按照字符串处理，内部重新解析。
4. **分隔符仅支持`;`。** systemd支持如`;`，空格等多种多样的分隔符，但sysmaster仅允许使用`;`。
5. **错误配置会导致单元无法启动。** systemd会忽略一些非关键的配置错误，但sysmaster严格检查配置是否正确，配置错误将导致单元无法启动。
6. **不允许配置为空。** systemd允许用户将某个选项配置为空，此时它会采用缺省值。sysmaster不允许配置为空，如果用户需要使用缺省值，可以直接在配置文件中删除该配置。

# Warnings

1. 由于Rust使用String字符串,对于长度无限制, 建议用户在撰写配置文件时控制在合理的长度内.
2. sysmaster默认支持的unit最大数量为500个，超过该值可能会异常退出。用户可以通过修改/etc/sysmaster/system.conf的DbSize配置提高unit上限。修改方法请参考：[外置db配置](./sysmaster.conf.md)

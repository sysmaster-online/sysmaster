# design choices

- For never changed slices, use `Rc<[T]>` (or `Rc<str>`, `Rc<Path>`);
  `Rc` is `Arc` or `Rc`, controlled by type alias.
- Use async for IO operations;
- For other blocking operations, use `spawn_blocking`;
- why use `rustix` rather than `libc` and `nix`
  - rustc use this
  - safe and friendly api

# 整体架构

- 使用Actor模型，并使用channel传递信息，其范式如下：
  ```rust
  struct Message { ... }
  struct Actor { ... }
  impl Actor {
    pub fn new() -> Self
    pub fn run(mut self, mut rx: Receiver<Message>) -> JoinHandle<()> {
      tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
          match msg { ... }
        }
      })
    }
  }
  ```

- Unit 实现
  ```rust
  struct Impl { ... }
  struct Handle { ... }
  #[async_trait]
  impl crate::unit::Handle for Handle {
      async fn stop(self: Box<Self>) -> UnitHandle {
          // use runtime info to stop the running things
      }
      async fn wait(&mut self) -> RtMsg {
          // monitor runtime state, and return messages including rt notice or exit state...
      }
  }

  #[async_trait]
  impl Unit for UnitImpl<Impl> {
      ...
      async start(&self) -> Result<UnitHandle, ()> {
          // start job here, return a handle which
          // contains runtime info needed for monitor and stop/kill
      }

      // do things needed to stop the unit
      async fn stop(&self, handle: UnitHandle) -> Result<(), ()>;

      async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()>;
  }
  ```

- 配置文件读取：得到impl Unit，并填充足够信息（依赖，start,stop等等）
  配置文件解析器：`File -> impl Unit`
  特性：使用基于tokio的异步io

- Actor
  - DepStore
    - 运行时进行依赖关系的暂存与等待，保证各个Unit按预期顺序启动
    - api
    ```rust
    pub(crate) enum Message {
        /// 加载一个Unit的依赖信息
        Load(UnitId, Rc<UnitDeps>),
        /// 增加一项等待启动的Unit
        AddToStart(UnitId),
        AddToStop(UnitId),
        /// 收到通知事件：指定Unit的状态发生改变
        StateChange(UnitId, State),
    }
    ```
    - 引用的其他actor
      - GuardStore StateStore GuardStore

  - GuardStore
    - 储存Unit的运行时守护task
    - api
    ```rust
    pub(crate) enum Message {
        /// Query if guard of the specific unit exists
        Contains(UnitId, oneshot::Sender<bool>),
        /// Insert a guard.
        Insert(UnitId),
        /// remove a guard \
        /// usually called by self when a gurad quits
        Remove(UnitId),
        /// notice all deps are ready for a specific unit \
        /// called by `Dep`
        DepsReady(UnitId),
        /// notice there's at least one required dep of the specific unit failed
        DepsFailed(UnitId),
        /// Send a Stop message to the specific unit guard
        Stop(UnitId),
        /// Notify a unit that it already dead
        NotifyDead(UnitId),
    }
    ```
    - 引用的其他actor
      - DepStore StateStore UnitStore MountMonitorStore

  - StateStore
    - 储存Unit的状态信息。目前的状态如下：
    ```rust
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub enum State {
        #[default]
        Uninit = 0,
        Stopped,
        Failed,
        Starting,
        Active,
        Stopping,
    }
    ```
    - api
    ```rust
    pub(crate) enum Message {
        /// 打印内部信息，用于调试
        DbgPrint,
        /// 获得指定Unit的状态
        Get(UnitId, oneshot::Sender<State>),
        /// 注册一个hook,用于监听特性unit的状态改变 \
        /// 是一个坏的api：由于unit start之后，set state的时机无法确定， \
        ///     因此想要在start一类操作之后获得state作为结果的情景无法使用此api实现
        Monitor {
            id: UnitId,
            s: MonitorRet,
            cond: Box<dyn FnOnce(State) -> bool + Send + 'static>,
        },
        /// 无条件设置指定Unit的状态
        Set(UnitId, State),
        /// 以当前状态作为条件决定是否设置指定Unit状态 \
        /// 一定程度上相当于对指定Unit的状态进行CAS原子操作
        SetWithCondition {
            id: UnitId,
            new_state: State,
            condition: Box<dyn FnOnce(State) -> bool + Send + 'static>,
        },
    }
    ```
    - 引用的其他actor
      - DepStore

  - UnitStore
    - 存储Unit静态信息，并负责Unit的start, stop, restart等动作的发出，以及依赖解析
    - api
    ```rust
    pub(crate) enum Message {
        /// 用于调试 打印内部信息
        DbgPrint,
        /// 用于更新/插入对应Unit的静态信息
        Update(UnitId, UnitObj),
        /// 移除Store中的指定Unit
        Remove(UnitId),
        /// 启动指定Unit
        Start(UnitId),
        /// 停止指定Unit
        Stop(UnitId),
        /// 重启指定Unit
        Restart(UnitId),
    }
    ```
    - 引用的其他actor
      - DepStore
  - MountMonitorStore
    - 挂载点监控
    - api
    ```rust
    pub(crate) enum Message {
        Register(UnitId),
        Remove(UnitId),
    }
    ```
    - 引用的其他Store
      GuardStore

- signal handler
  - 利用tokio自带机制完成注册
  - 对于一个signal, 由于在tokio里面可以使用stream的形式处理，因此我们很容易得到以下注册方式：
    ```rust
    fn register_signal_handler<F>(signalkind: SignalKind, mut handler: F)
    where
        F: FnMut() + Send + 'static,
    {
        let mut sig = signal(signalkind).unwrap();
        tokio::spawn(async move {
            loop {
                sig.recv().await;
                handler();
            }
        });
    }
    ```
    其中 `handler`在每次收到后会被调用。作为信号处理函数，其内部逻辑应当尽可能避免阻塞。

# units

接口定义：
```rust
#[async_trait]
pub(crate) trait Unit: Debug {
    fn name(&self) -> Rc<str>;
    fn description(&self) -> Rc<str>;
    fn documentation(&self) -> Rc<str>;
    fn kind(&self) -> UnitKind;

    fn deps(&self) -> Rc<UnitDeps>;

    /// start the unit, return a handle which
    /// contains runtime info needed for monitor and stop/kill
    async fn start(&self) -> Result<UnitHandle, ()>; // todo: error type

    /// do things needed to stop the unit
    async fn stop(&self, handle: UnitHandle) -> Result<(), ()>;

    async fn restart(&self, handle: UnitHandle) -> Result<UnitHandle, ()>;
}

#[derive(Debug)]
pub(crate) struct UnitImpl<KindImpl> {
    pub common: UnitCommon,
    pub sub: KindImpl,
}
```

# TODO：

- [ ] make `Sender<Message>` a handle type, make Message private
- [x] refactor: actor mod
- [x] Unify Naming
	- [x] Dep -> DepStore
	- [x] GuardManager -> GuardStore
	- [x] StateManager -> StateStore
	- [x] Store -> UnitStore
- [x] refactor guard code
  给Guard一个类型，而不是现在的`Box<dyn FnOnce(Sender<store::Message>, Sender<state::Message>, Receiver<GuardMessage>) -> BoxFuture<'static, State> + Send + 'static>`
- [ ] impl socket trigger service start
  add Args for `Unit::start`, and pass socket to service
- [ ] remove all magic numbers and use const instead
- [ ] logging
- [ ] Error handle
- [x] Remove state handle in store, use guard to know the state of units
- [ ] impl Deps like systemd
  - [ ] test
    - [ ] write test
    - [ ] pass test
  - [ ] impl
    - [x] start requires and wants
    - [x] wait requires starting
    - [x] fail when requires failed
    - [x] stop when requires stop
    - [x] stop conflicts
    - [x] wait requires/wants active due to before/after
    - [x] want conflicts stop due to before/after
    - [ ] restart related:
      - [ ] restart when requires restart
- [ ] unified unit loader(depinfo name ...)

## signals

- [ ] handle signals (references: sysmaster, systemd)
  - [x] impl signal handler
  - [ ] handle signals like sysmaster and systemd

## units
- mount & swap
  - [X] parse fstab
    - device
      - [ ] LABEL
      - [ ] PARTLABEL
      - [X] UUID
      - [ ] PARTUUID
      - [ ] ID
      - [X] PATH
        - [ ] valid path
    - [ ] check paths (reference: libmount)
  - [x] generate .mount unit
  - [ ] generate .swap unit
  - [ ] mount/unmount fs
  - [ ] swapon/off
  - [ ] monitor mounts and swaps

- service
  - [ ] parse .service file
  - [ ] start/stop service
  - [ ] monitor service

- timer
  - [ ] parse .timer file
- socket
  - [ ] parse .socket file
- target
  - [ ] parse .target file
